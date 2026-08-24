//! Script access to retained state.
//!
//! An element description lives for one render pass, but a text input's
//! content, cursor and undo history must survive every pass — so they live in a
//! GPUI entity and the script holds a handle (design doc §7.3). This module is
//! the JavaScript face of [`crate::entities`].
//!
//! Every function here takes and returns scalars. That is not a style
//! preference: a closure that both takes `Ctx<'js>` and returns a borrowed
//! `Object<'js>` cannot unify the two elided lifetimes, so the handle object
//! itself is assembled in the JS prelude, exactly as element objects are.

use std::rc::Weak;

use rquickjs::{
    Ctx, Exception, FromJs, Function, Object, Persistent, Result as JsResult, Value, function::Func,
};

use crate::{
    entities::{self, EntityHandle, InputEventName},
    scope::{self, ScopePhase},
    spec::{Component, SpecId},
};

use super::ShellRuntime;

/// A script callback, persisted at conversion time.
///
/// A closure cannot take both `Ctx<'js>` and `Function<'js>` — the two elided
/// lifetimes will not unify — so the function is saved into a `Persistent`
/// inside `FromJs`, where both are still the same lifetime. The same reason the
/// engine's `Arguments` type exists.
struct Handler(Persistent<Function<'static>>);

impl<'js> FromJs<'js> for Handler {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let function = value
            .as_function()
            .ok_or_else(|| Exception::throw_type(ctx, "expected a function"))?;
        Ok(Self(Persistent::save(ctx, function.clone())))
    }
}

/// Installs the host half of the retained-state API. The prelude wraps these
/// into `InputState` and `Input`.
pub fn install(ctx: &Ctx<'_>, module: &Object<'_>, runtime: Weak<ShellRuntime>) -> JsResult<()> {
    let _ = module;
    let globals = ctx.globals();

    globals.set(
        "__input_state_new",
        Func::from(
            |ctx: Ctx<'_>,
             placeholder: Option<String>,
             value: Option<String>|
             -> JsResult<EntityHandle> {
                let phase = scope::current_phase();
                if matches!(phase, Some(ScopePhase::Render) | Some(ScopePhase::Layout)) {
                    return Err(Exception::throw_type(
                        &ctx,
                        "InputState.new(...) cannot run during render; create state in init() \
                         or in an event handler and keep it on the view",
                    ));
                }

                scope::with_current(|window, cx| {
                    entities::create_input(placeholder, value, window, cx)
                })
                .ok_or_else(|| {
                    Exception::throw_type(
                        &ctx,
                        "InputState.new(...) needs a live host call; call it from init() \
                         or an event handler",
                    )
                })
            },
        ),
    )?;

    globals.set(
        "__input_value",
        Func::from(|ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<String> {
            let state = live(&ctx, handle)?;
            scope::with_current_app(|cx| state.read(cx).value().to_string())
                .ok_or_else(|| needs_call(&ctx, "value()"))
        }),
    )?;

    globals.set(
        "__input_set_value",
        Func::from(
            |ctx: Ctx<'_>, handle: EntityHandle, value: String| -> JsResult<()> {
                let state = live(&ctx, handle)?;
                scope::with_current(|window, cx| {
                    state.update(cx, |state, cx| state.set_value(value, window, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_value()"))
            },
        ),
    )?;

    let subscribe_runtime = runtime.clone();
    globals.set(
        "__input_on",
        Func::from(
            move |ctx: Ctx<'_>,
                  handle: EntityHandle,
                  name: String,
                  handler: Handler|
                  -> JsResult<bool> {
                let event = InputEventName::from_name(&name).ok_or_else(|| {
                    Exception::throw_type(
                        &ctx,
                        &format!(
                            "unknown input event `{name}`; expected one of: {}",
                            InputEventName::NAMES.join(", ")
                        ),
                    )
                })?;

                let saved = handler.0;
                let runtime = subscribe_runtime.clone();

                let subscribed = scope::with_current(|window, cx| {
                    entities::subscribe_input(
                        handle,
                        event,
                        window,
                        cx,
                        move |emitted, window, cx| {
                            let Some(runtime) = runtime.upgrade() else {
                                return;
                            };
                            runtime.dispatch_input_event(&saved, emitted, window, cx);
                        },
                    )
                })
                .ok_or_else(|| {
                    Exception::throw_type(
                        &ctx,
                        "on(...) needs a live host call; subscribe from init() or an event handler",
                    )
                })?;

                if !subscribed {
                    return Err(Exception::throw_type(
                        &ctx,
                        "this input state has been released",
                    ));
                }
                Ok(true)
            },
        ),
    )?;

    globals.set(
        "__input_release",
        Func::from(|handle: EntityHandle| entities::release(handle)),
    )?;

    globals.set(
        "__input_element",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<SpecId> {
                if entities::input(handle).is_none() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "this input state has been released and can no longer be rendered",
                    ));
                }
                let runtime = runtime
                    .upgrade()
                    .ok_or_else(|| Exception::throw_message(&ctx, "the runtime has shut down"))?;
                Ok(runtime.push_component(Component::Input(handle)))
            },
        ),
    )?;

    Ok(())
}

fn live(
    ctx: &Ctx<'_>,
    handle: EntityHandle,
) -> JsResult<gpui::Entity<gpui_base::input::InputState>> {
    entities::input(handle)
        .ok_or_else(|| Exception::throw_type(ctx, "this input state has been released"))
}

fn needs_call(ctx: &Ctx<'_>, what: &str) -> rquickjs::Error {
    Exception::throw_type(
        ctx,
        &format!("{what} needs a live host call; call it from render or an event handler"),
    )
}
