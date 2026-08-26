//! `gpui.native(name)` — the QuickJS side of [`crate::native`].
//!
//! Everything interesting about native modules is engine independent and lives
//! above this file. What is left here is exactly the two conversions the seam
//! forbids the registry from knowing about (§6.5 rule 1): a script value
//! becomes a [`NativeValue`], and a [`NativeValue`] becomes a script value.
//!
//! ```js
//! import { native } from "gpui";
//!
//! const release = native("release");
//! const steps = release.steps();
//! ```
//!
//! # Two shapes worth knowing
//!
//! - **The module object is built per call, and frozen.** `native("release")`
//!   returns a fresh object whose own properties are the registered functions
//!   and whose prototype is a `Proxy` that reports an unknown name. Own
//!   properties are found without ever consulting the proxy, so the trap is on
//!   the miss path only — the same trade the element prototype makes in
//!   `mod.rs`, without needing its two-pass dance, because a native call is not
//!   on the per-element path. Freezing means a script cannot stash state on the
//!   module and cannot shadow a function with its own.
//! - **Conversion happens inside `FromJs`/`IntoJs`.** A closure passed to
//!   `Func::from` cannot unify the `Ctx<'js>` of its parameter with a
//!   `Value<'js>` in its return type — the two elided lifetimes are distinct to
//!   the compiler. Both directions are therefore expressed as conversions on
//!   `'static` wrapper types, where `'js` appears once.

use rquickjs::{
    Array, Ctx, Exception, FromJs, Function, IntoJs, Object, Result as JsResult, Value,
    function::{Func, Rest},
};

use crate::{
    native::{self, NativeArguments, NativeValue},
    scope,
};

use super::ShellRuntime;

/// How deep an argument may nest.
///
/// A script can hand over a structure of any depth, and conversion is
/// recursive; a limit turns "the host was passed a 100k-deep list" from a
/// blown Rust stack into a message at the call site. Sixteen is far past any
/// record a native function has business receiving.
const MAX_DEPTH: usize = 16;

/// Maximum number of array slots converted across a JavaScript/Rust boundary.
///
/// Sparse arrays cost just as much as dense ones here because bridge semantics
/// preserve every hole as `null`. Keeping one limit for native values and host
/// JSON prevents either conversion path becoming an allocation bypass.
pub(super) const MAX_BRIDGE_ARRAY_ITEMS: usize = 10_000;

pub(super) fn bridge_array_len(ctx: &Ctx<'_>, array: &Array<'_>) -> JsResult<usize> {
    // Do not use `Array::len`: rquickjs 0.12 asserts that QuickJS returned a
    // signed integer, while valid JS arrays may have lengths above i32::MAX and
    // QuickJS represents those as floating-point values.
    let length: Value = array.as_object().get("length")?;
    let Some(length) = length.as_number() else {
        return Err(Exception::throw_type(ctx, "array length is not a number"));
    };
    if !length.is_finite() || length < 0.0 || length.fract() != 0.0 {
        return Err(Exception::throw_type(
            ctx,
            "array length must be a finite non-negative integer",
        ));
    }
    if length > MAX_BRIDGE_ARRAY_ITEMS as f64 {
        return Err(Exception::throw_range(
            ctx,
            &format!(
                "array has {length:.0} items, over the {MAX_BRIDGE_ARRAY_ITEMS}-item bridge limit"
            ),
        ));
    }
    Ok(length as usize)
}

/// Assembles a module object around the bound functions the host registered.
///
/// In JS rather than Rust because a `Proxy` is the whole trick and it reads as
/// four lines here. `then` is withheld along with the `__` names: a module
/// object that answers `then` with a function would be mistaken for a thenable
/// by any `await`, and awaiting one would hang.
const PRELUDE: &str = r#"
globalThis.__native_module = (name, table) => {
  const guard = new Proxy(Object.create(null), {
    get(_target, key) {
      if (typeof key !== "string" || key === "then" || key.startsWith("__")) return undefined;
      return () => __native_unknown(name, key);
    },
  });

  // `defineProperties`, not `Object.assign`: an ordinary assignment consults
  // the prototype chain, and the guard would swallow every one of them.
  const descriptors = {};
  for (const key of Object.keys(table)) {
    descriptors[key] = { value: table[key], enumerable: true };
  }
  return Object.freeze(Object.create(guard, descriptors));
};
"#;

/// Installs `native` on the `gpui` module object.
///
/// The context argument is the engine's; the module object carries its own.
/// They are the same context at run time, but invariance in `'js` makes them
/// distinct types here — the same constraint documented in `host::install`.
pub fn install(_ctx: &Ctx<'_>, module: &Object<'_>) -> JsResult<()> {
    let ctx = module.ctx();
    ctx.eval::<(), _>(PRELUDE)?;

    ctx.globals().set(
        "__native_unknown",
        Func::from(
            |ctx: Ctx<'_>, module: String, function: String| -> JsResult<()> {
                let message = match native::modules().get(&module) {
                    Ok(found) => format!(
                        "native module `{module}` has no function `{function}`; it provides: {}",
                        found.function_names().join(", ")
                    ),
                    // The module resolved a moment ago, when `native()` handed
                    // it out; if it does not now, the host revoked it in
                    // between and that is the more useful thing to report.
                    Err(error) => error.message().to_owned(),
                };
                Err(Exception::throw_type(&ctx, &message))
            },
        ),
    )?;

    module.set(
        "native",
        Func::from(|ctx: Ctx<'_>, name: String| -> JsResult<ModuleBinding> {
            let registry = native::modules();
            let module = registry
                .get(&name)
                .map_err(|error| Exception::throw_message(&ctx, error.message()))?;

            Ok(ModuleBinding {
                functions: module
                    .function_names()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                name,
            })
        }),
    )?;

    Ok(())
}

/// A resolved module, on its way back to the script.
///
/// It holds names rather than functions: the bound functions can only be
/// created from a `Ctx`, and this type exists precisely to postpone that until
/// [`IntoJs`], where the context's lifetime is nameable.
struct ModuleBinding {
    name: String,
    functions: Vec<String>,
}

impl<'js> IntoJs<'js> for ModuleBinding {
    fn into_js(self, ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
        let table = Object::new(ctx.clone())?;
        for function in self.functions {
            let module = self.name.clone();
            table.set(
                function.clone(),
                Func::from(
                    move |ctx: Ctx<'_>, arguments: Rest<Argument>| -> JsResult<Bridged> {
                        let arguments =
                            NativeArguments::new(arguments.0.into_iter().map(|it| it.0));
                        // Timed so a script render can be told apart from the
                        // host work inside it: `quotes()` reading a board out of
                        // an entity is not the script describing itself, and
                        // charging it to JavaScript would be a lie in the
                        // flattering direction.
                        // Looked up and released before dispatching: a host
                        // function reaches for the ambient `App` itself, and
                        // holding it across the call would be two borrows of
                        // one thing.
                        let runtime =
                            scope::with_current_app(|cx| ShellRuntime::global(cx)).flatten();
                        let dispatched = match &runtime {
                            Some(runtime) => runtime
                                .metrics()
                                .time_native(|| native::dispatch(&module, &function, &arguments)),
                            None => native::dispatch(&module, &function, &arguments),
                        };
                        match dispatched {
                            Ok(value) => Ok(Bridged(value)),
                            // The registry's messages never name their own
                            // function, so the call site is named exactly once.
                            Err(error) => Err(Exception::throw_message(
                                &ctx,
                                &format!("`{module}.{function}`: {error}"),
                            )),
                        }
                    },
                ),
            )?;
        }

        let build: Function = ctx.globals().get("__native_module")?;
        build.call((self.name, table))
    }
}

/// One argument, converted on the way in.
struct Argument(NativeValue);

impl<'js> FromJs<'js> for Argument {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        Ok(Self(from_js(ctx, value, 0)?))
    }
}

fn from_js<'js>(ctx: &Ctx<'js>, value: Value<'js>, depth: usize) -> JsResult<NativeValue> {
    if depth > MAX_DEPTH {
        return Err(Exception::throw_type(
            ctx,
            &format!("a native argument may not nest more than {MAX_DEPTH} levels deep"),
        ));
    }

    if value.is_null() || value.is_undefined() {
        return Ok(NativeValue::Null);
    }
    if let Some(flag) = value.as_bool() {
        return Ok(NativeValue::Bool(flag));
    }
    if let Some(number) = value.as_number() {
        return Ok(NativeValue::Number(number));
    }
    if let Some(text) = value.as_string() {
        return Ok(NativeValue::Str(text.to_string()?));
    }
    // Before the object case: an array is an object too.
    if let Some(array) = value.as_array() {
        let length = bridge_array_len(ctx, &array)?;
        let mut values = Vec::new();
        values.try_reserve_exact(length).map_err(|_| {
            Exception::throw_range(
                ctx,
                "native array could not be reserved within memory limits",
            )
        })?;
        for index in 0..length {
            values.push(from_js(ctx, array.get(index)?, depth + 1)?);
        }
        return Ok(NativeValue::Array(values));
    }
    // A function would be a handle, and a handle is the one thing that must not
    // cross: the host could hold it past the call, and past the scope frame
    // that made the surrounding context valid.
    if value.as_function().is_some() {
        return Err(Exception::throw_type(
            ctx,
            "a native function cannot be passed a callback; native calls take and return \
             plain data only",
        ));
    }
    if let Some(object) = value.as_object() {
        let mut fields = Vec::new();
        for entry in object.props::<String, Value>() {
            let (key, value) = entry?;
            fields.push((key, from_js(ctx, value, depth + 1)?));
        }
        return Ok(NativeValue::Object(fields));
    }

    Err(Exception::throw_type(
        ctx,
        "unsupported native argument; expected null, a boolean, a number, a string, \
         an array or a plain object",
    ))
}

/// One result, converted on the way out.
struct Bridged(NativeValue);

impl<'js> IntoJs<'js> for Bridged {
    fn into_js(self, ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
        into_js(ctx, self.0)
    }
}

fn into_js<'js>(ctx: &Ctx<'js>, value: NativeValue) -> JsResult<Value<'js>> {
    Ok(match value {
        NativeValue::Null => Value::new_null(ctx.clone()),
        NativeValue::Bool(flag) => Value::new_bool(ctx.clone(), flag),
        NativeValue::Number(number) => Value::new_number(ctx.clone(), number),
        NativeValue::Str(text) => rquickjs::String::from_str(ctx.clone(), &text)?.into_value(),
        NativeValue::Array(values) => {
            let array = Array::new(ctx.clone())?;
            for (index, value) in values.into_iter().enumerate() {
                array.set(index, into_js(ctx, value)?)?;
            }
            array.into_value()
        }
        NativeValue::Object(fields) => {
            let object = Object::new(ctx.clone())?;
            for (key, value) in fields {
                object.set(key, into_js(ctx, value)?)?;
            }
            object.into_value()
        }
    })
}

#[cfg(test)]
mod tests {
    use rquickjs::{Context as JsContext, Error as JsError, Runtime as JsRuntime};

    use super::*;

    #[test]
    fn a_sparse_huge_native_array_is_a_catchable_error() {
        let runtime = JsRuntime::new().expect("runtime");
        let context = JsContext::full(&runtime).expect("context");
        context.with(|ctx| {
            let value: Value = ctx
                .eval("const values = []; values.length = 0xffffffff; values")
                .expect("sparse array");
            let error = match Argument::from_js(&ctx, value) {
                Ok(_) => panic!("the bridge must refuse a huge sparse array"),
                Err(error) => error,
            };
            assert!(matches!(error, JsError::Exception), "{error}");
            let thrown = ctx.catch();
            let message = thrown
                .as_exception()
                .and_then(|exception| exception.message())
                .unwrap_or_else(|| format!("{thrown:?}"));
            assert!(
                message.contains("array") && message.contains("limit"),
                "{message}"
            );
        });
    }
}
