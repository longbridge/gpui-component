//! Turns element descriptions into real GPUI elements.
//!
//! This runs entirely in Rust: it never calls back into Lua, which is what
//! makes it possible to benchmark and snapshot-test the render path
//! independently of the VM.

use std::rc::{Rc, Weak};

use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, MouseButton, ParentElement, Refineable as _,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
};
use gpui_base::{
    Button, Checkbox, CheckboxState, Switch, h_flex,
    input::{Input, InputBase},
    v_flex,
};

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
    /// What a screen reader announces. An icon-only control has no text of its
    /// own, so without this it announces nothing.
    accessibility_label: Option<SharedString>,
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
pub fn materialize(
    runtime: &Rc<ShellRuntime>,
    id: SpecId,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let ambient = window.text_style().color;
    materialize_node(runtime, id, ambient, window, cx)
}

/// Materializes one node, carrying the text color down the description.
///
/// GPUI resolves inherited text color while painting, but an svg needs the
/// color on its *own* style before it will paint at all — and by then the
/// description is gone. So inheritance is resolved here, walking the tree the
/// script built: each node passes down its own `text_color` if it set one, and
/// the ambient color otherwise. That is what makes an icon inside a dark button
/// come out light without the script saying so twice.
#[allow(clippy::only_used_in_recursion)]
fn materialize_node(
    runtime: &Rc<ShellRuntime>,
    id: SpecId,
    inherited: gpui::Hsla,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let Some(node) = runtime.arena_mut().take(id) else {
        return div().into_any_element();
    };
    let Some(component) = node.component.clone() else {
        return div().into_any_element();
    };

    let (refinement, behavior, states) = resolve_ops(runtime, &node);
    let inherited = refinement.text.color.unwrap_or(inherited);

    let children: Vec<AnyElement> = node
        .children
        .iter()
        .map(|child| materialize_node(runtime, *child, inherited, window, cx))
        .collect();

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

            if let Some(label) = behavior.accessibility_label.clone() {
                button = button.accessibility_label(label);
            }

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

            if let Some(label) = behavior.accessibility_label.clone() {
                checkbox = checkbox.accessibility_label(label);
            }

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

            let checkbox = with_hover(checkbox, &states);
            let checkbox = with_active_and_focus(checkbox, &states);
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

            // `Switch` itself is not interactive — `SwitchTrack` is — so a
            // state style on the switch root has nowhere to land. Saying so is
            // better than dropping it without a word.
            if states.hover.is_some() || states.active.is_some() || states.focus.is_some() {
                tracing::warn!(
                    "state styles on a Switch are ignored; style the row around it instead"
                );
            }
            finish(switch, refinement, children)
        }
        Component::Svg(path) => {
            // GPUI paints an svg only when the element's own style carries a
            // text color, and an inherited color reaches children as a text
            // style rather than as this element's style — so an icon with no
            // color of its own silently draws nothing. `inherited` already
            // accounts for this node's own `text_color`, so an explicit color
            // still wins and an icon in a dark button comes out light.
            let mut image = gpui::svg().path(SharedString::from(path));
            image.style().refine(&refinement);
            image.style().text.color = Some(inherited);
            image.into_any_element()
        }
        Component::Input(handle) => {
            let Some(state) = crate::entities::input(handle) else {
                tracing::error!("input handle {handle} is no longer live");
                return div().into_any_element();
            };

            // `InputBase` rather than a bare `div`: it is the foundational input
            // frame, so it carries the input semantics, the focused state style
            // and the accessibility role that a `div` around a text run does
            // not. `Input` itself draws the text and nothing else.
            //
            // Three defaults are applied before the script's own styling, so a
            // script can override any of them but does not have to remember
            // them:
            //
            // * a centered row — otherwise the text sits at the top of whatever
            //   height the frame was given, which is what a missing `h` looks
            //   like on screen;
            // * full width, so the editable area is the frame rather than the
            //   width of the text already in it;
            // * a click anywhere in the frame focuses the input, because the
            //   padding is part of the control as far as a user is concerned.
            let focus_target = state.clone();
            let mut frame = InputBase::new(("gpui-shell-input", handle))
                .flex()
                .items_center()
                .w_full()
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    focus_target.update(cx, |state, cx| state.focus(window, cx));
                });

            frame.style().refine(&refinement);
            frame.extend(children);
            let frame = with_hover(frame, &states);
            let frame = with_active_and_focus(frame, &states);
            frame.child(Input::new(&state)).into_any_element()
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
        "accessibility_label" => {
            behavior.accessibility_label = args
                .first()
                .and_then(|value| value.as_str().ok())
                .map(SharedString::from);
        }
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
