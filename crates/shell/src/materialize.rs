//! Turns element descriptions into real GPUI elements.
//!
//! This runs entirely in Rust: it never calls back into Lua, which is what
//! makes it possible to benchmark and snapshot-test the render path
//! independently of the VM.

use std::rc::{Rc, Weak};

use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, Refineable as _, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
};
use gpui_base::{Button, Checkbox, CheckboxState, Switch, h_flex, v_flex};

use crate::{
    engine::ShellRuntime,
    spec::{CallbackId, Component, SpecId, SpecNode, SpecOp},
    style,
    value::Bridged,
};

/// Behavior collected from a node's ops, applied after styling.
/// Style refinements that apply only in a runtime state.
#[derive(Default)]
struct StateStyles {
    hover: Option<StyleRefinement>,
    active: Option<StyleRefinement>,
    focus: Option<StyleRefinement>,
}

impl StateStyles {
    /// `active` and `focus` need a stable element identity; `hover` does not.
    fn needs_identity(&self) -> bool {
        self.active.is_some() || self.focus.is_some()
    }
}

#[derive(Default)]
struct Behavior {
    disabled: bool,
    selected: bool,
    checked: bool,
    on_click: Option<CallbackId>,
    on_change: Option<CallbackId>,
}

/// Materializes `id` and every descendant. Nodes are taken out of the arena as
/// they are consumed, so a description can only ever be materialized once.
///
/// `window` and `cx` are threaded through even though only the recursion uses
/// them today: entity-backed components (Input, Tree, Table) and tooltips need
/// both at construction time, and they are part of this function's contract
/// rather than an oversight.
#[allow(clippy::only_used_in_recursion)]
pub fn materialize(
    runtime: &Rc<ShellRuntime>,
    id: SpecId,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let Some(node) = runtime.arena_mut().take(id) else {
        return div().into_any_element();
    };
    let Some(component) = node.component.clone() else {
        return div().into_any_element();
    };

    let children: Vec<AnyElement> = node
        .children
        .iter()
        .map(|child| materialize(runtime, *child, window, cx))
        .collect();

    let (refinement, behavior, states) = resolve_ops(runtime, &node);

    match component {
        Component::Div => flex_element(div(), id, refinement, behavior, states, children),
        Component::HFlex => flex_element(h_flex(), id, refinement, behavior, states, children),
        Component::VFlex => flex_element(v_flex(), id, refinement, behavior, states, children),
        Component::Text(value) => {
            let mut element = div();
            element.style().refine(&refinement);
            element.extend(children);
            element.child(SharedString::from(value)).into_any_element()
        }
        Component::Button(id) => {
            let mut button = Button::new(SharedString::from(id))
                .disabled(behavior.disabled)
                .selected(behavior.selected);

            if let Some(callback) = behavior.on_click {
                let runtime = Rc::downgrade(runtime);
                button = button.on_click(move |event, window, cx| {
                    dispatch_click(&runtime, callback, event, window, cx);
                });
            }

            let button = with_hover(button, &states);
            let button = with_active_and_focus(button, &states);
            finish(button, refinement, children)
        }
        Component::Checkbox(id) => {
            let mut checkbox = Checkbox::new(SharedString::from(id))
                .disabled(behavior.disabled)
                .checked(behavior.checked);

            if let Some(callback) = behavior.on_change {
                let runtime = Rc::downgrade(runtime);
                checkbox = checkbox.on_change(move |state, _, window, cx| {
                    dispatch_change(
                        &runtime,
                        callback,
                        matches!(state, CheckboxState::Checked),
                        window,
                        cx,
                    );
                });
            }

            finish(checkbox, refinement, children)
        }
        Component::Switch(id) => {
            let mut switch = Switch::new(SharedString::from(id))
                .disabled(behavior.disabled)
                .checked(behavior.checked);

            if let Some(callback) = behavior.on_change {
                let runtime = Rc::downgrade(runtime);
                switch = switch.on_change(move |checked, _, window, cx| {
                    dispatch_change(&runtime, callback, checked, window, cx);
                });
            }

            finish(switch, refinement, children)
        }
    }
}

fn finish<E>(mut element: E, refinement: StyleRefinement, children: Vec<AnyElement>) -> AnyElement
where
    E: Styled + ParentElement + IntoElement + 'static,
{
    element.style().refine(&refinement);
    element.extend(children);
    element.into_any_element()
}

/// Applies the state refinements a component supports.
///
/// `active` needs a stable element identity, which only a `Stateful` element
/// has; `div()` becomes stateful lazily and Button already is, so the two arms
/// differ. Components that are not interactive silently ignore state styles
/// rather than failing the render — the script gets a `tracing` warning.
fn with_hover<E: InteractiveElement>(element: E, states: &StateStyles) -> E {
    match states.hover.clone() {
        Some(hover) => element.hover(move |mut style| {
            style.refine(&hover);
            style
        }),
        None => element,
    }
}

fn with_active_and_focus<E: StatefulInteractiveElement>(element: E, states: &StateStyles) -> E {
    let mut element = element;
    if let Some(active) = states.active.clone() {
        element = element.active(move |mut style| {
            style.refine(&active);
            style
        });
    }
    if let Some(focus) = states.focus.clone() {
        element = element.focus(move |mut style| {
            style.refine(&focus);
            style
        });
    }
    element
}

/// A plain `div` becomes stateful only when a state style needs an identity.
///
/// The identity is derived from the node's position in the description, which
/// is stable across renders for a stable tree — the same property GPUI relies
/// on for its own element ids.
fn flex_element(
    element: gpui::Div,
    id: SpecId,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Vec<AnyElement>,
) -> AnyElement {
    let _ = behavior;
    let element = with_hover(element, &states);
    if !states.needs_identity() {
        return finish(element, refinement, children);
    }

    let stateful = element.id(gpui::ElementId::NamedInteger(
        "gpui-shell".into(),
        id as u64,
    ));
    finish(
        with_active_and_focus(stateful, &states),
        refinement,
        children,
    )
}

fn resolve_ops(
    runtime: &Rc<ShellRuntime>,
    node: &SpecNode,
) -> (StyleRefinement, Behavior, StateStyles) {
    let mut refinement = StyleRefinement::default();
    let mut behavior = Behavior::default();
    let mut states = StateStyles::default();

    for op in &node.ops {
        match op {
            SpecOp::NullaryStyle(index) => {
                refinement = style::apply_nullary(*index, refinement);
            }
            SpecOp::ParamStyle(name, args) => {
                match style::apply_param(name, args, refinement.clone()) {
                    Ok(next) => refinement = next,
                    Err(error) => {
                        // Argument coercion already ran when the op was
                        // recorded, so reaching here means a host bug rather
                        // than a script error. Keep the frame renderable.
                        tracing::error!("style `{name}` failed during materialize: {error}");
                    }
                }
            }
            SpecOp::Method(name, args) => apply_behavior(&mut behavior, name, args),
            SpecOp::StateStyle(name, node) => {
                let resolved = resolve_state(runtime, *node);
                match *name {
                    "hover" => states.hover = Some(resolved),
                    "active" => states.active = Some(resolved),
                    "focus" => states.focus = Some(resolved),
                    other => tracing::error!("unhandled state style `{other}`"),
                }
            }
            SpecOp::Callback(name, id) => match *name {
                "on_click" => behavior.on_click = Some(*id),
                "on_change" => behavior.on_change = Some(*id),
                other => tracing::error!("unhandled callback `{other}` reached materialize"),
            },
        }
    }

    (refinement, behavior, states)
}

/// Resolves a detached state node into a refinement. Only style ops are
/// meaningful there; anything else is a script mistake already reported at the
/// call site.
fn resolve_state(runtime: &Rc<ShellRuntime>, node: SpecId) -> StyleRefinement {
    let Some(node) = runtime.arena_mut().take(node) else {
        return StyleRefinement::default();
    };

    let mut refinement = StyleRefinement::default();
    for op in &node.ops {
        match op {
            SpecOp::NullaryStyle(index) => refinement = style::apply_nullary(*index, refinement),
            SpecOp::ParamStyle(name, args) => {
                if let Ok(next) = style::apply_param(name, args, refinement.clone()) {
                    refinement = next;
                }
            }
            _ => {}
        }
    }
    refinement
}

fn apply_behavior(behavior: &mut Behavior, name: &str, args: &[Bridged]) {
    let flag = args.first().map(|arg| arg.as_bool().unwrap_or(true));
    match name {
        "disabled" => behavior.disabled = flag.unwrap_or(true),
        "selected" => behavior.selected = flag.unwrap_or(true),
        "checked" => behavior.checked = flag.unwrap_or(true),
        _ => tracing::error!("unhandled component method `{name}` reached materialize"),
    }
}

fn dispatch_change(
    runtime: &Weak<ShellRuntime>,
    callback: CallbackId,
    checked: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(runtime) = runtime.upgrade() else {
        return;
    };
    runtime.dispatch_change(callback, checked, window, cx);
}

fn dispatch_click(
    runtime: &Weak<ShellRuntime>,
    callback: CallbackId,
    event: &gpui::ClickEvent,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(runtime) = runtime.upgrade() else {
        return;
    };
    runtime.dispatch_click(callback, event, window, cx);
}
