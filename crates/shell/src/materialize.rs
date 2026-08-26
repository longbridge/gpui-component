//! Turns element descriptions into real GPUI elements.
//!
//! This runs entirely in Rust: it never calls back into the script, which is what
//! makes it possible to benchmark and snapshot-test the render path
//! independently of the VM — and, more importantly, what lets GPUI repaint a
//! script view as often as it likes without entering one.
//!
//! Reading a snapshot leaves it intact, so the same description is replayed by
//! every frame until script state replaces it. The runtime is still needed here,
//! but only to dispatch events: no path through this module calls into the
//! script while an element is being built.

use std::{
    rc::{Rc, Weak},
    time::Duration,
};

use smallvec::SmallVec;

use gpui::{
    AbsoluteLength, AnyElement, App, DefiniteLength, InteractiveElement, IntoElement, Length,
    MouseButton, ParentElement, Pixels, Refineable as _, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, div,
};
use gpui_base::{
    Button, Checkbox, CheckboxState, Link, Switch,
    animation::{ease_in_cubic, ease_in_out_cubic, ease_out_cubic},
    h_flex,
    input::{Input, InputBase},
    motion::{Spring, Transition, spring, transition},
    v_flex,
};

use crate::{
    engine::ShellRuntime,
    snapshot::RenderSnapshot,
    spec::{CallbackId, Component, SpecArena, SpecId, SpecNode, SpecOp},
    style,
    value::Bridged,
};

/// The children of one node, inline until there are more than a row's worth.
///
/// Eight because a quote row is six cells and a wrapper, which is the widest
/// ordinary shape; past that the spill is one allocation for a node that was
/// always going to be expensive.
type Children = SmallVec<[AnyElement; 8]>;

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
    /// A name the script gave this element, used as its GPUI identity.
    ///
    /// Without one, identity falls back to the node's address in the
    /// description — which is stable only while the script builds the same tree
    /// in the same order. A conditional child earlier in the tree shifts every
    /// address after it, and with it the active state, the focus, and anything
    /// else GPUI keys by id. `id("toolbar")` is how a script says which element
    /// this is, rather than where it happened to land.
    key: Option<SharedString>,
    /// What a screen reader announces. An icon-only control has no text of its
    /// own, so without this it announces nothing.
    accessibility_label: Option<SharedString>,
    href: Option<SharedString>,
    on_click: Option<CallbackId>,
    on_change: Option<CallbackId>,
    scroll_x: bool,
    scroll_y: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MotionProperty {
    Opacity,
    Width,
    Height,
    Left,
    Top,
}

impl MotionProperty {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "opacity" => Some(Self::Opacity),
            "width" => Some(Self::Width),
            "height" => Some(Self::Height),
            "left" => Some(Self::Left),
            "top" => Some(Self::Top),
            _ => None,
        }
    }

    fn channel(self) -> &'static str {
        match self {
            Self::Opacity => "opacity",
            Self::Width => "width",
            Self::Height => "height",
            Self::Left => "left",
            Self::Top => "top",
        }
    }
}

#[derive(Clone)]
struct Motion {
    property: MotionProperty,
    policy: MotionPolicy,
}

#[derive(Clone)]
enum MotionPolicy {
    Transition {
        duration: Duration,
        delay: Duration,
        easing: String,
    },
    Spring {
        response: Duration,
        damping: f32,
        epsilon: f32,
    },
}

/// Materializes a snapshot's root and every descendant.
///
/// Reading is non-destructive, so this may be called any number of times on the
/// same snapshot and produces the same interface each time. That is the whole
/// point: a hover, a cursor blink or an animation frame repaints through here
/// and never through the VM.
///
/// `window` and `cx` are threaded through even though only the recursion uses
/// them today: entity-backed components (Input, Tree, Table) and tooltips need
/// both at construction time, and they are part of this function's contract
/// rather than an oversight.
pub fn materialize(
    runtime: &Rc<ShellRuntime>,
    snapshot: &RenderSnapshot,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let ambient = window.text_style().color;
    // Counted and timed because this is the half that follows frames: the story
    // and the benchmark both read the two counters side by side, and the gap
    // between them is the architecture.
    let metrics = runtime.metrics();
    metrics.time_materialize(|| {
        materialize_node(
            runtime,
            snapshot.arena(),
            snapshot.root(),
            ambient,
            window,
            cx,
        )
    })
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
    arena: &SpecArena,
    id: SpecId,
    inherited: gpui::Hsla,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let Some(node) = arena.node(id) else {
        return div().into_any_element();
    };
    let Some(component) = node.component().cloned() else {
        return div().into_any_element();
    };

    let (mut refinement, behavior, states, motions) = resolve_ops(arena, node);
    let motion_identity = motion_element_id(id, behavior.key.clone(), &component);
    apply_motion(motion_identity, &motions, &mut refinement, window, cx);
    let inherited = refinement.text.color.unwrap_or(inherited);

    // `SmallVec` rather than `Vec`: this runs per node, per frame, and the
    // overwhelming majority of nodes have a handful of children or none. A
    // heap allocation for each of them is a cost the snapshot was supposed to
    // remove, arriving one layer down.
    let children: Children = node
        .children()
        .iter()
        .map(|child| materialize_node(runtime, arena, *child, inherited, window, cx))
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
            warn_ignored_key(&behavior, "Button");
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
        Component::Link(id) => {
            warn_ignored_key(&behavior, "Link");
            let mut link = Link::new(SharedString::from(id)).disabled(behavior.disabled);
            if let Some(label) = behavior.accessibility_label.clone() {
                link = link.accessibility_label(label);
            }
            if let Some(href) = behavior.href.clone() {
                link = link
                    .href(href)
                    .open_with(|href, _, _, cx| cx.open_url(href));
            }
            if let Some(callback) = behavior.on_click {
                let runtime = Rc::downgrade(runtime);
                link = link.on_activate(move |event, window, cx| {
                    dispatch_click(&runtime, callback, event, window, cx);
                });
            }
            let link = with_hover(link, &states);
            let link = with_active_and_focus(link, &states);
            finish(link, refinement, children)
        }
        Component::Checkbox(id) => {
            warn_ignored_key(&behavior, "Checkbox");
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
            warn_ignored_key(&behavior, "Switch");
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
            let Some(state) = runtime.entities().input(handle) else {
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

fn finish<E>(mut element: E, refinement: StyleRefinement, children: Children) -> AnyElement
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

/// A plain `div` becomes stateful when a state style needs an identity, or when
/// the script named it.
///
/// A script-given name wins, because it is the only identity that survives the
/// script reordering its own tree. Without one the identity is the node's
/// address in the description, which is stable for as long as the snapshot lives
/// and across rebuilds only while the tree keeps its shape.
fn flex_element(
    element: gpui::Div,
    id: SpecId,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    let element = with_hover(element, &states);
    if behavior.key.is_none()
        && !states.needs_identity()
        && !behavior.scroll_x
        && !behavior.scroll_y
    {
        return finish(element, refinement, children);
    }

    let scroll_x = behavior.scroll_x;
    let scroll_y = behavior.scroll_y;
    let stateful = element.id(element_id(id, behavior.key));
    let stateful = with_active_and_focus(stateful, &states);
    let stateful = match (scroll_x, scroll_y) {
        (true, true) => stateful.overflow_scroll(),
        (true, false) => stateful.overflow_x_scroll(),
        (false, true) => stateful.overflow_y_scroll(),
        (false, false) => stateful,
    };
    finish(stateful, refinement, children)
}

/// A control that already takes an identity from `new(id)` has nowhere to put a
/// second one. Saying so beats dropping it without a word.
fn warn_ignored_key(behavior: &Behavior, component: &str) {
    if let Some(key) = &behavior.key {
        tracing::warn!(
            "id(\"{key}\") is ignored on a {component}: it is already identified by the id \
             passed to {component}.new(...)"
        );
    }
}

/// The script's name for an element, or its address in the description.
fn element_id(id: SpecId, key: Option<SharedString>) -> gpui::ElementId {
    match key {
        Some(key) => gpui::ElementId::Name(key),
        None => gpui::ElementId::NamedInteger("gpui-shell".into(), id as u64),
    }
}

/// Resolves the identity used by native retained motion.
///
/// Controls and retained inputs already carry an identity that survives tree
/// reordering. Falling back to the snapshot position for them would make the
/// visual track jump to another control whenever a conditional sibling shifts
/// its `SpecId`.
fn motion_element_id(
    id: SpecId,
    key: Option<SharedString>,
    component: &Component,
) -> gpui::ElementId {
    match component {
        Component::Button(id)
        | Component::Link(id)
        | Component::Checkbox(id)
        | Component::Switch(id) => gpui::ElementId::Name(id.clone().into()),
        Component::Input(handle) => {
            gpui::ElementId::NamedInteger("gpui-shell-input".into(), u64::from(*handle))
        }
        _ => element_id(id, key),
    }
}

fn resolve_ops(
    arena: &SpecArena,
    node: &SpecNode,
) -> (
    StyleRefinement,
    Behavior,
    StateStyles,
    SmallVec<[Motion; 2]>,
) {
    let mut refinement = StyleRefinement::default();
    let mut behavior = Behavior::default();
    let mut states = StateStyles::default();
    let mut motions = SmallVec::new();

    for op in node.ops() {
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
            SpecOp::Method("transition", args) => {
                if let [
                    Bridged::Str(property),
                    Bridged::Number(duration),
                    Bridged::Number(delay),
                    Bridged::Str(easing),
                ] = args.as_slice()
                    && let Some(property) = MotionProperty::parse(property)
                {
                    set_motion(
                        &mut motions,
                        Motion {
                            property,
                            policy: MotionPolicy::Transition {
                                duration: Duration::from_secs_f64((*duration).max(0.0) / 1000.0),
                                delay: Duration::from_secs_f64((*delay).max(0.0) / 1000.0),
                                easing: easing.clone(),
                            },
                        },
                    );
                }
            }
            SpecOp::Method("spring", args) => {
                if let [
                    Bridged::Str(property),
                    Bridged::Number(response),
                    Bridged::Number(damping),
                    Bridged::Number(epsilon),
                ] = args.as_slice()
                    && let Some(property) = MotionProperty::parse(property)
                {
                    set_motion(
                        &mut motions,
                        Motion {
                            property,
                            policy: MotionPolicy::Spring {
                                response: Duration::from_secs_f64((*response).max(0.0) / 1000.0),
                                damping: *damping as f32,
                                epsilon: *epsilon as f32,
                            },
                        },
                    );
                }
            }
            SpecOp::Method(name, args) => apply_behavior(&mut behavior, name, args),
            SpecOp::StateStyle(name, node) => {
                let resolved = resolve_state(arena, *node);
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

    (refinement, behavior, states, motions)
}

fn set_motion(motions: &mut SmallVec<[Motion; 2]>, motion: Motion) {
    if let Some(existing) = motions
        .iter_mut()
        .find(|existing| existing.property == motion.property)
    {
        *existing = motion;
    } else {
        motions.push(motion);
    }
}

fn apply_motion(
    identity: gpui::ElementId,
    motions: &[Motion],
    refinement: &mut StyleRefinement,
    window: &mut Window,
    cx: &mut App,
) {
    for motion in motions {
        let channel = motion.property.channel();
        match motion.property {
            MotionProperty::Opacity => {
                if let Some(target) = refinement.opacity {
                    refinement.opacity = Some(sample_f32(
                        identity.clone(),
                        channel,
                        target,
                        &motion.policy,
                        window,
                        cx,
                    ));
                }
            }
            MotionProperty::Width => animate_length(
                &mut refinement.size.width,
                identity.clone(),
                channel,
                &motion.policy,
                window,
                cx,
            ),
            MotionProperty::Height => animate_length(
                &mut refinement.size.height,
                identity.clone(),
                channel,
                &motion.policy,
                window,
                cx,
            ),
            MotionProperty::Left => animate_length(
                &mut refinement.inset.left,
                identity.clone(),
                channel,
                &motion.policy,
                window,
                cx,
            ),
            MotionProperty::Top => animate_length(
                &mut refinement.inset.top,
                identity.clone(),
                channel,
                &motion.policy,
                window,
                cx,
            ),
        }
    }
}

fn animate_length(
    target: &mut Option<Length>,
    identity: gpui::ElementId,
    channel: &'static str,
    policy: &MotionPolicy,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(pixels)))) = *target
    else {
        return;
    };
    let sampled = sample_pixels(identity, channel, pixels, policy, window, cx);
    *target = Some(sampled.into());
}

fn transition_policy(duration: Duration, delay: Duration, easing: &str) -> Transition {
    let policy = Transition::new(duration).delay(delay);
    match easing {
        "linear" => policy.ease(|value| value),
        "ease-in" => policy.ease(ease_in_cubic),
        "ease-in-out" => policy.ease(ease_in_out_cubic),
        _ => policy.ease(ease_out_cubic),
    }
}

fn sample_f32(
    identity: gpui::ElementId,
    channel: &'static str,
    target: f32,
    policy: &MotionPolicy,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    match policy {
        MotionPolicy::Transition {
            duration,
            delay,
            easing,
        } => transition(
            (identity, channel),
            target,
            transition_policy(*duration, *delay, easing),
            window,
            cx,
        ),
        MotionPolicy::Spring {
            response,
            damping,
            epsilon,
        } => spring(
            (identity, channel),
            target,
            Spring::new(*response)
                .with_damping(*damping)
                .with_epsilon(*epsilon),
            window,
            cx,
        ),
    }
}

fn sample_pixels(
    identity: gpui::ElementId,
    channel: &'static str,
    target: Pixels,
    policy: &MotionPolicy,
    window: &mut Window,
    cx: &mut App,
) -> Pixels {
    match policy {
        MotionPolicy::Transition {
            duration,
            delay,
            easing,
        } => transition(
            (identity, channel),
            target,
            transition_policy(*duration, *delay, easing),
            window,
            cx,
        ),
        MotionPolicy::Spring {
            response,
            damping,
            epsilon,
        } => spring(
            (identity, channel),
            target,
            Spring::new(*response)
                .with_damping(*damping)
                .with_epsilon(*epsilon),
            window,
            cx,
        ),
    }
}

/// Resolves a detached state node into a refinement. Only style ops are
/// meaningful there; anything else is a script mistake already reported at the
/// call site.
fn resolve_state(arena: &SpecArena, node: SpecId) -> StyleRefinement {
    let Some(node) = arena.node(node) else {
        return StyleRefinement::default();
    };

    let mut refinement = StyleRefinement::default();
    for op in node.ops() {
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
        "id" => {
            behavior.key = args
                .first()
                .and_then(|value| value.as_str().ok())
                .map(SharedString::from);
        }
        "href" => {
            behavior.href = args
                .first()
                .and_then(|value| value.as_str().ok())
                .map(SharedString::from);
        }
        "disabled" => behavior.disabled = flag.unwrap_or(true),
        "selected" => behavior.selected = flag.unwrap_or(true),
        "checked" => behavior.checked = flag.unwrap_or(true),
        "overflow_scroll" => {
            behavior.scroll_x = true;
            behavior.scroll_y = true;
        }
        "overflow_x_scroll" => behavior.scroll_x = true,
        "overflow_y_scroll" => behavior.scroll_y = true,
        _ => tracing::error!("unhandled component method `{name}` reached materialize"),
    }
}

#[cfg(test)]
mod motion_identity_tests {
    use super::*;

    #[test]
    fn control_motion_uses_the_constructor_identity_instead_of_spec_position() {
        assert_eq!(
            motion_element_id(41, None, &Component::Button("save".into())),
            gpui::ElementId::Name("save".into())
        );
        assert_eq!(
            motion_element_id(7, None, &Component::Link("authorize".into())),
            gpui::ElementId::Name("authorize".into())
        );
        assert_eq!(
            motion_element_id(99, None, &Component::Checkbox("remember".into())),
            gpui::ElementId::Name("remember".into())
        );
    }

    #[test]
    fn retained_input_motion_uses_its_entity_handle() {
        assert_eq!(
            motion_element_id(41, None, &Component::Input(23)),
            gpui::ElementId::NamedInteger("gpui-shell-input".into(), 23)
        );
    }

    #[test]
    fn the_last_motion_policy_for_a_property_wins() {
        let mut motions = SmallVec::new();
        set_motion(
            &mut motions,
            Motion {
                property: MotionProperty::Left,
                policy: MotionPolicy::Transition {
                    duration: Duration::from_millis(100),
                    delay: Duration::ZERO,
                    easing: "linear".to_owned(),
                },
            },
        );
        set_motion(
            &mut motions,
            Motion {
                property: MotionProperty::Left,
                policy: MotionPolicy::Spring {
                    response: Duration::from_millis(300),
                    damping: 0.8,
                    epsilon: 0.001,
                },
            },
        );

        assert_eq!(motions.len(), 1);
        assert!(matches!(motions[0].policy, MotionPolicy::Spring { .. }));
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
