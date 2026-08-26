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

use gpui_base::input::{InputBaseState, InputModeKind};
use rquickjs::{
    Ctx, Exception, FromJs, Function, Object, Persistent, Result as JsResult, Value, function::Func,
};

use crate::{
    entities::{EntityHandle, InputEventName},
    scope::{self, ScopePhase},
    spec::{Component, SpecId},
};

use super::{InputCallbackOwner, ShellRuntime};

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
/// into `InputState`, `TextareaState`, `FocusHandle` and the elements built
/// from them.
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
                refuse_creation_in_render(&ctx, "InputState.new(...)")?;

                let store = alive(&ctx, &create)?;
                scope::with_current(|window, cx| {
                    store.entities().create_input(
                        placeholder,
                        value,
                        scope::current_application_generation(),
                        window,
                        cx,
                    )
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
                read_value(&ctx, &live(&ctx, &read, handle)?)
            },
        ),
    )?;

    let write = runtime.clone();
    globals.set(
        "__input_set_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, value: String| -> JsResult<()> {
                write_value(&ctx, &live(&ctx, &write, handle)?, value)
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
                subscribe(&ctx, &subscribe_runtime, handle, &name, handler, "input")
            },
        ),
    )?;

    // Multi-line text. `TextareaState` is a different Rust type from
    // `InputState` — the same engine specialized on another mode — so it needs
    // its own creation and its own resolver. Everything the two states share
    // goes through the generic helpers below rather than being written twice.
    let create_textarea = runtime.clone();
    globals.set(
        "__textarea_state_new",
        Func::from(
            move |ctx: Ctx<'_>,
                  placeholder: Option<String>,
                  value: Option<String>,
                  rows: Option<usize>|
                  -> JsResult<EntityHandle> {
                refuse_creation_in_render(&ctx, "TextareaState.new(...)")?;

                let store = alive(&ctx, &create_textarea)?;
                scope::with_current(|window, cx| {
                    store.entities().create_textarea(
                        placeholder,
                        value,
                        rows,
                        scope::current_application_generation(),
                        window,
                        cx,
                    )
                })
                .ok_or_else(|| {
                    Exception::throw_type(
                        &ctx,
                        "TextareaState.new(...) needs a live host call; call it from init() \
                         or an event handler",
                    )
                })
            },
        ),
    )?;

    let read_textarea = runtime.clone();
    globals.set(
        "__textarea_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<String> {
                read_value(&ctx, &live_textarea(&ctx, &read_textarea, handle)?)
            },
        ),
    )?;

    let write_textarea = runtime.clone();
    globals.set(
        "__textarea_set_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, value: String| -> JsResult<()> {
                write_value(&ctx, &live_textarea(&ctx, &write_textarea, handle)?, value)
            },
        ),
    )?;

    let subscribe_textarea = runtime.clone();
    globals.set(
        "__textarea_on",
        Func::from(
            move |ctx: Ctx<'_>,
                  handle: EntityHandle,
                  name: String,
                  handler: Handler|
                  -> JsResult<bool> {
                subscribe(
                    &ctx,
                    &subscribe_textarea,
                    handle,
                    &name,
                    handler,
                    "textarea",
                )
            },
        ),
    )?;

    let rows_runtime = runtime.clone();
    globals.set(
        "__textarea_set_rows",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, rows: usize| -> JsResult<()> {
                let state = live_textarea(&ctx, &rows_runtime, handle)?;
                scope::with_current_app(|cx| {
                    state.update(cx, |state, cx| state.set_rows(rows, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_rows()"))
            },
        ),
    )?;

    let grow_runtime = runtime.clone();
    globals.set(
        "__textarea_set_auto_grow",
        Func::from(
            move |ctx: Ctx<'_>,
                  handle: EntityHandle,
                  min_rows: usize,
                  max_rows: usize|
                  -> JsResult<()> {
                let state = live_textarea(&ctx, &grow_runtime, handle)?;
                scope::with_current_app(|cx| {
                    state.update(cx, |state, cx| state.set_auto_grow(min_rows, max_rows, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_auto_grow()"))
            },
        ),
    )?;

    let wrap_runtime = runtime.clone();
    globals.set(
        "__textarea_set_soft_wrap",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, wrap: bool| -> JsResult<()> {
                let state = live_textarea(&ctx, &wrap_runtime, handle)?;
                // Needs the window as well as the app: turning wrapping on
                // re-measures against the width the last layout produced.
                scope::with_current(|window, cx| {
                    state.update(cx, |state, cx| state.set_soft_wrap(wrap, window, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_soft_wrap()"))
            },
        ),
    )?;

    let discard_textarea = runtime.clone();
    globals.set(
        "__textarea_release",
        Func::from(move |handle: EntityHandle| {
            discard_textarea
                .upgrade()
                .is_some_and(|runtime| runtime.entities().release(handle))
        }),
    )?;

    let textarea_element = runtime.clone();
    globals.set(
        "__textarea_element",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<SpecId> {
                let store = alive(&ctx, &textarea_element)?;
                if store.entities().textarea(handle).is_none() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "this textarea state has been released and can no longer be rendered",
                    ));
                }
                Ok(store.push_component(Component::Textarea(handle)))
            },
        ),
    )?;

    // Focus handles. A focus handle is not an input, but it is retained state
    // held by handle for the same reason, so it lives in the same store and
    // reaches the script the same way.
    let create_focus = runtime.clone();
    globals.set(
        "__focus_handle_new",
        Func::from(move |ctx: Ctx<'_>| -> JsResult<EntityHandle> {
            let phase = scope::current_phase();
            if matches!(phase, Some(ScopePhase::Render) | Some(ScopePhase::Layout)) {
                return Err(Exception::throw_type(
                    &ctx,
                    "FocusHandle.new() cannot run during render; a handle created there would \
                     be a new one every frame, so the focus it tracks would be dropped by the \
                     next repaint. Create it in init() or in an event handler and keep it on \
                     the view",
                ));
            }

            let store = alive(&ctx, &create_focus)?;
            scope::with_current_app(|cx| {
                store
                    .entities()
                    .create_focus(scope::current_application_generation(), cx)
            })
            .ok_or_else(|| {
                Exception::throw_type(
                    &ctx,
                    "FocusHandle.new() needs a live host call; call it from init() or an \
                     event handler",
                )
            })
        }),
    )?;

    let take_focus = runtime.clone();
    globals.set(
        "__focus_focus",
        Func::from(move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<()> {
            let focus = live_focus(&ctx, &take_focus, handle)?;
            scope::with_current(|window, cx| window.focus(&focus, cx))
                .ok_or_else(|| needs_call(&ctx, "focus()"))
        }),
    )?;

    let read_focus = runtime.clone();
    globals.set(
        "__focus_is_focused",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<bool> {
                let focus = live_focus(&ctx, &read_focus, handle)?;
                // Through the window rather than the handle alone: focus is a
                // fact about one window, and `is_focused` needs it to answer.
                scope::with_current(|window, _| focus.is_focused(window))
                    .ok_or_else(|| needs_call(&ctx, "is_focused()"))
            },
        ),
    )?;

    let discard_focus = runtime.clone();
    globals.set(
        "__focus_release",
        Func::from(move |handle: EntityHandle| {
            discard_focus
                .upgrade()
                .is_some_and(|runtime| runtime.entities().release(handle))
        }),
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

fn live_textarea(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
) -> JsResult<gpui::Entity<gpui_base::input::TextareaState>> {
    alive(ctx, runtime)?
        .entities()
        .textarea(handle)
        .ok_or_else(|| Exception::throw_type(ctx, "this textarea state has been released"))
}

/// Reads the text out of either state.
///
/// Generic over the mode marker because `value()` lives on the shared engine:
/// the single-line and multi-line states answer it the same way, and only the
/// resolver that produced the entity differs.
fn read_value<M: InputModeKind>(
    ctx: &Ctx<'_>,
    state: &gpui::Entity<InputBaseState<M>>,
) -> JsResult<String> {
    scope::with_current_app(|cx| state.read(cx).value().to_string())
        .ok_or_else(|| needs_call(ctx, "value()"))
}

fn write_value<M: InputModeKind>(
    ctx: &Ctx<'_>,
    state: &gpui::Entity<InputBaseState<M>>,
    value: String,
) -> JsResult<()> {
    scope::with_current(|window, cx| {
        state.update(cx, |state, cx| state.set_value(value, window, cx));
    })
    .ok_or_else(|| needs_call(ctx, "set_value()"))
}

/// Subscribes a script handler to one named event on either text state.
///
/// The store resolves the handle to whichever entity it names, so the whole of
/// this — the event-name check, the captured grant, the released-state report —
/// is shared; `what` only names the state in the error a script sees.
fn subscribe(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
    name: &str,
    handler: Handler,
    what: &str,
) -> JsResult<bool> {
    let event = InputEventName::from_name(name).ok_or_else(|| {
        Exception::throw_type(
            ctx,
            &format!(
                "unknown input event `{name}`; expected one of: {}",
                InputEventName::NAMES.join(", ")
            ),
        )
    })?;

    let saved = handler.0;
    let dispatch = runtime.clone();
    let store = alive(ctx, runtime)?;
    // Captured here, not read when the event arrives: this subscription
    // outlives the call that made it, and an input on a plugin's form must
    // dispatch under that plugin's grant rather than under whatever the default
    // policy happens to be.
    let owner = InputCallbackOwner {
        policy: scope::policy(),
        application: scope::current_application_generation(),
        view: scope::current_view().map(|view| view.downgrade()),
    };

    let subscribed = scope::with_current(|window, cx| {
        store
            .entities()
            .subscribe_input(handle, event, window, cx, move |emitted, window, cx| {
                let Some(runtime) = dispatch.upgrade() else {
                    return;
                };
                runtime.dispatch_input_event(&saved, &owner, emitted, window, cx);
            })
    })
    .ok_or_else(|| {
        Exception::throw_type(
            ctx,
            "on(...) needs a live host call; subscribe from init() or an event handler",
        )
    })?;

    if !subscribed {
        return Err(Exception::throw_type(
            ctx,
            &format!("this {what} state has been released"),
        ));
    }
    Ok(true)
}

/// Refuses to create retained state during a render pass.
///
/// State created there would be new on every frame, so what the script thought
/// it was keeping — the text typed into it, the focus on it — would be dropped
/// by the next repaint.
fn refuse_creation_in_render(ctx: &Ctx<'_>, constructor: &str) -> JsResult<()> {
    if matches!(
        scope::current_phase(),
        Some(ScopePhase::Render) | Some(ScopePhase::Layout)
    ) {
        return Err(Exception::throw_type(
            ctx,
            &format!(
                "{constructor} cannot run during render; create state in init() or in an \
                 event handler and keep it on the view"
            ),
        ));
    }
    Ok(())
}

fn live_focus(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
) -> JsResult<gpui::FocusHandle> {
    alive(ctx, runtime)?
        .entities()
        .focus(handle)
        .ok_or_else(|| Exception::throw_type(ctx, "this focus handle has been released"))
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
