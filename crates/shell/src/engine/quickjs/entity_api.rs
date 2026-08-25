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
    entities::{EntityHandle, InputEventName},
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

    // Every entity call reaches its store through the runtime, because the
    // store belongs to the runtime rather than to the thread — see
    // `crate::entities`. Each closure carries its own `Weak`, since a `Func`
    // owns what it captures.
    let create = runtime.clone();
    globals.set(
        "__input_state_new",
        Func::from(
            move |ctx: Ctx<'_>,
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

                let store = alive(&ctx, &create)?;
                scope::with_current(|window, cx| {
                    store
                        .entities()
                        .create_input(placeholder, value, window, cx)
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

    let read = runtime.clone();
    globals.set(
        "__input_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<String> {
                let state = live(&ctx, &read, handle)?;
                scope::with_current_app(|cx| state.read(cx).value().to_string())
                    .ok_or_else(|| needs_call(&ctx, "value()"))
            },
        ),
    )?;

    let write = runtime.clone();
    globals.set(
        "__input_set_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, value: String| -> JsResult<()> {
                let state = live(&ctx, &write, handle)?;
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
                let dispatch = subscribe_runtime.clone();
                let store = alive(&ctx, &subscribe_runtime)?;
                // Captured here, not read when the event arrives: this
                // subscription outlives the call that made it, and an input on a
                // plugin's form must dispatch under that plugin's grant rather
                // than under whatever the default policy happens to be.
                let policy = scope::policy();

                let subscribed = scope::with_current(|window, cx| {
                    store.entities().subscribe_input(
                        handle,
                        event,
                        window,
                        cx,
                        move |emitted, window, cx| {
                            let Some(runtime) = dispatch.upgrade() else {
                                return;
                            };
                            runtime.dispatch_input_event(&saved, &policy, emitted, window, cx);
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

    let discard = runtime.clone();
    globals.set(
        "__input_release",
        Func::from(move |handle: EntityHandle| {
            discard
                .upgrade()
                .is_some_and(|runtime| runtime.entities().release(handle))
        }),
    )?;

    globals.set(
        "__input_element",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<SpecId> {
                let store = alive(&ctx, &runtime)?;
                if store.entities().input(handle).is_none() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "this input state has been released and can no longer be rendered",
                    ));
                }
                Ok(store.push_component(Component::Input(handle)))
            },
        ),
    )?;

    Ok(())
}

fn live(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
) -> JsResult<gpui::Entity<gpui_base::input::InputState>> {
    alive(ctx, runtime)?
        .entities()
        .input(handle)
        .ok_or_else(|| Exception::throw_type(ctx, "this input state has been released"))
}

/// The runtime this handle's store belongs to, or a clear failure.
///
/// A `Weak` that no longer upgrades means the VM is being torn down while a
/// script call is still on the stack, which is a host bug rather than anything
/// the author wrote.
fn alive(ctx: &Ctx<'_>, runtime: &Weak<ShellRuntime>) -> JsResult<std::rc::Rc<ShellRuntime>> {
    runtime
        .upgrade()
        .ok_or_else(|| Exception::throw_message(ctx, "the runtime has shut down"))
}

fn needs_call(ctx: &Ctx<'_>, what: &str) -> rquickjs::Error {
    Exception::throw_type(
        ctx,
        &format!("{what} needs a live host call; call it from render or an event handler"),
    )
}
