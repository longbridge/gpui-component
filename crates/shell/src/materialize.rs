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
    Button, Checkbox, CheckboxState, Link, ScrollbarAxis, Switch,
    animation::{ease_in_cubic, ease_in_out_cubic, ease_out_cubic},
    h_flex,
    input::{Input, InputBase},
    motion::{Spring, Transition, spring, transition},
    v_flex,
};

mod components;

use crate::{
    engine::ShellRuntime,
    scroll::Scrollable,
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

/// The named slots a node's ops filled, in the order the script filled them.
///
/// A list rather than a set of fields because the slot names belong to the
/// components, not to this module: a `Collapsible` fills one, a popover fills a
/// trigger and a content, a number input fills three. Two inline covers every
/// component bound today and every one on the way; a third is one allocation on
/// a node that already carries whole subtrees.
type Slots = SmallVec<[(&'static str, AnyElement); 2]>;

/// The same list before the slots are materialized.
type SlotSpecs = SmallVec<[(&'static str, SpecId); 2]>;

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
    /// A `Toggle`'s controlled state. Separate from `checked` because that is
    /// what base calls it, and because the two announce differently: a pressed
    /// toggle is a button in a toggled state, not a checked box.
    pressed: bool,
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
    /// Reports the open state of an anchored surface after something other than
    /// the script changed it — a click on the trigger, a click outside, Escape.
    on_open_change: Option<CallbackId>,
    /// Reports Enter on an open `Select` or `Combobox`. It carries no payload:
    /// the root holds no value, so what was confirmed is whatever the script
    /// had highlighted.
    on_confirm: Option<CallbackId>,
    /// Reports Escape on an open `Select` or `Combobox`, before the open state
    /// is asked to close — which is what lets a script commit a pending value
    /// on the way out.
    on_dismiss: Option<CallbackId>,
    scroll_x: bool,
    scroll_y: bool,
    scrollbar: bool,
    /// A `Scrollbar`'s show mode. `None` keeps the theme's own projection,
    /// which is what every bar the shell already paints follows.
    scrollbar_mode: Option<gpui_base::ScrollbarMode>,
    /// The content size a `Scrollbar` measures its thumb against, when the
    /// script knows it and the scroll area does not — a list that paints a
    /// window of rows rather than all of them.
    scroll_size: Option<gpui::Size<Pixels>>,
    /// Whether a `Scrollbar` takes its viewport from its own layout box rather
    /// than from the scroll area. The way to keep a bar off a fixed header.
    viewport_from_layout: bool,
    /// This item's one-based position in its collection, and the collection's
    /// size — "tab 2 of 5". Announced, never drawn.
    position_in_set: Option<(usize, usize)>,
    /// The announced progress percentage. `None` leaves base's own default,
    /// which is 0 — a bar the script never told how far along it is.
    value: Option<f32>,
    /// Withdraws the value from the accessibility tree instead of changing it:
    /// "still working, no idea how far" is a different announcement from
    /// "at 40%".
    indeterminate: bool,
    /// The semantic orientation a grouping container announces. `None` keeps
    /// the container's own default, which differs between them: a radio group
    /// is vertical, a toolbar horizontal.
    axis: Option<gpui::Axis>,
    /// The whole table's size, including rows and columns outside the range
    /// the script rendered. A table that draws every row it has needs neither.
    row_count: Option<usize>,
    column_count: Option<usize>,
    /// Whether a `Collapsible` renders the element in its `content` slot, or
    /// whether a `Popover` is showing.
    ///
    /// `None` means the script never said, which is not the same as `false`: a
    /// popover that was never given an `open` is uncontrolled and starts from
    /// its own `default_open`, while one told `open(false)` is controlled and
    /// stays shut until the script says otherwise.
    open: Option<bool>,
    /// A `Popover`'s starting state, used only while it is uncontrolled.
    default_open: bool,
    /// Whether pressing outside a `Popover` closes it. `None` keeps base's own
    /// default, which is that it does.
    overlay_closable: Option<bool>,
    /// Which corner of the surface is pinned to the trigger. `None` keeps the
    /// component's own default, and the two differ: a popover anchors top-left,
    /// a hover card top-center.
    anchor: Option<gpui::Anchor>,
    /// The pointer button that opens a `Popover`.
    mouse_button: Option<MouseButton>,
    /// How long the pointer rests on a `HoverCard`'s trigger before it appears,
    /// and how long it may leave before it goes away.
    open_delay: Option<Duration>,
    close_delay: Option<Duration>,
    /// The handle of a script-created `FocusHandle` this element tracks.
    ///
    /// Stored as the handle rather than the resolved `gpui::FocusHandle`
    /// because resolving it needs the runtime, which [`resolve_ops`] does not
    /// have — and because a released handle has to be reported once, where it
    /// is used, rather than turned into silence here.
    focus_handle: Option<crate::entities::EntityHandle>,
    /// The handle the keyboard moves to when a `Select` or `Combobox` opens.
    ///
    /// A second handle rather than a second use of [`Self::focus_handle`],
    /// because a combobox has two focus targets at once: the trigger, which
    /// holds the keyboard while it is shut, and the list, which takes it while
    /// it is open.
    content_focus_handle: Option<crate::entities::EntityHandle>,
    /// Where this element sits in the window's focus traversal order.
    tab_index: Option<isize>,
    /// Whether Tab can land on it at all. Separate from [`Self::tab_index`]
    /// because a container that keeps its place in the order without being
    /// reachable is the whole point of `tab_stop(false)`.
    tab_stop: Option<bool>,
    /// What this element announces itself as.
    role: Option<gpui::Role>,
    aria_selected: Option<bool>,
    /// Announces this element as the focused node while an ancestor holds real
    /// focus — the highlighted option of a combobox whose input keeps the
    /// keyboard.
    aria_active_descendant: bool,
}

impl Behavior {
    /// Whether focus or accessibility gave this element a reason to be
    /// identified.
    ///
    /// A tab stop that changes identity between renders is a tab stop the
    /// keyboard loses on the next repaint, and GPUI produces an accessibility
    /// node only for an element that has an id — so both halves need one.
    fn needs_identity(&self) -> bool {
        self.focus_handle.is_some()
            || self.tab_index.is_some()
            || self.tab_stop.is_some()
            || self.role.is_some()
            || self.aria_selected.is_some()
            || self.aria_active_descendant
    }
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

    let (mut refinement, behavior, states, motions, slot_specs) = resolve_ops(arena, node);
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

    // A slot's element is built here, beside the children, so it inherits the
    // same text color: where the component chooses to put it is the component's
    // business, but what it looks like should not depend on that choice.
    let slots: Slots = if materializes_slots(&component, &behavior) {
        slot_specs
            .iter()
            .map(|(name, slot)| {
                (
                    *name,
                    materialize_node(runtime, arena, *slot, inherited, window, cx),
                )
            })
            .collect()
    } else {
        SmallVec::new()
    };
    // Every component that reads a slot reports its own leftovers, because only
    // it knows which names it read.
    if !matches!(
        component,
        Component::Collapsible
            | Component::Popover(_)
            | Component::HoverCard(_)
            | Component::Popup(_)
    ) {
        warn_unread_slots(&slots, component.name());
    }

    materialize_component(
        runtime, node, id, component, inherited, refinement, behavior, states, children, slots,
        window, cx,
    )
}

/// Dispatches an already-materialized node to its concrete component.
///
/// Keeping this large, monomorphization-heavy match out of `materialize_node`
/// is not cosmetic: the latter is recursive, so every byte of its stack frame
/// is multiplied by the depth of the script's element tree. Components can
/// grow independently without making ordinary nested layouts exhaust Rust's
/// default test-thread stack.
#[allow(clippy::too_many_arguments)]
fn materialize_component(
    runtime: &Rc<ShellRuntime>,
    node: &SpecNode,
    id: SpecId,
    component: Component,
    inherited: gpui::Hsla,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
    slots: Slots,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    match component {
        Component::Div => flex_element(
            runtime,
            div(),
            id,
            refinement,
            behavior,
            states,
            children,
            window,
            cx,
        ),
        Component::HFlex => flex_element(
            runtime,
            h_flex(),
            id,
            refinement,
            behavior,
            states,
            children,
            window,
            cx,
        ),
        Component::VFlex => flex_element(
            runtime,
            v_flex(),
            id,
            refinement,
            behavior,
            states,
            children,
            window,
            cx,
        ),
        Component::Text(value) => {
            // Text, svg and image are the three components with no identity and
            // no interactivity, so nothing on them can carry a role or a tab
            // stop. Wrap them in a `div()` and put it there.
            warn_unhonoured_a11y(&behavior, "text", &[]);
            let mut element = div();
            element.style().refine(&refinement);
            element.extend(children);
            element.child(SharedString::from(value)).into_any_element()
        }
        Component::Button(id) => {
            warn_ignored_key(&behavior, "Button");
            warn_unhonoured_a11y(
                &behavior,
                "Button",
                &["track_focus", "tab_index", "tab_stop", "role"],
            );
            let mut button = Button::new(SharedString::from(id))
                .disabled(behavior.disabled)
                .selected(behavior.selected);

            if let Some(label) = behavior.accessibility_label.clone() {
                button = button.accessibility_label(label);
            }

            // Base's own builders, not GPUI's: `Button::render` builds the
            // focus handle it tracks from these three fields, so the trait
            // methods of the same name would be overwritten a moment later.
            if let Some(focus) = tracked_focus(runtime, &behavior, "Button") {
                button = button.track_focus(&focus);
            }
            if let Some(index) = behavior.tab_index {
                button = button.tab_index(index);
            }
            if let Some(stop) = behavior.tab_stop {
                button = button.tab_stop(stop);
            }
            // `Button::role` overrides the implicit `Role::Button`, which is
            // what makes a button that opens a menu announce itself as one.
            if let Some(role) = behavior.role {
                button = button.role(role);
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
            // No `track_focus`: base's `Link` builds its own keyed handle and
            // has no builder to replace it, so a script's handle would name a
            // focus target the link never focuses.
            warn_unhonoured_a11y(&behavior, "Link", &["tab_index", "tab_stop"]);
            let mut link = Link::new(SharedString::from(id)).disabled(behavior.disabled);
            if let Some(label) = behavior.accessibility_label.clone() {
                link = link.accessibility_label(label);
            }
            if let Some(index) = behavior.tab_index {
                link = link.tab_index(index);
            }
            if let Some(stop) = behavior.tab_stop {
                link = link.tab_stop(stop);
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
            warn_unhonoured_a11y(
                &behavior,
                "Checkbox",
                &["track_focus", "tab_index", "tab_stop", "role"],
            );
            let mut checkbox = Checkbox::new(SharedString::from(id))
                .disabled(behavior.disabled)
                .checked(behavior.checked);

            if let Some(label) = behavior.accessibility_label.clone() {
                checkbox = checkbox.accessibility_label(label);
            }

            if let Some(focus) = tracked_focus(runtime, &behavior, "Checkbox") {
                checkbox = checkbox.track_focus(&focus);
            }
            if let Some(index) = behavior.tab_index {
                checkbox = checkbox.tab_index(index);
            }
            if let Some(stop) = behavior.tab_stop {
                checkbox = checkbox.tab_stop(stop);
            }
            // The override is what turns a checkbox into a menu item that
            // carries a check, which is a different announcement.
            if let Some(role) = behavior.role {
                checkbox = checkbox.role(role);
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
            // Like `Link`, `Switch` builds its own keyed focus handle and
            // announces `Role::Switch` itself.
            warn_unhonoured_a11y(&behavior, "Switch", &["tab_index", "tab_stop"]);
            let mut switch = Switch::new(SharedString::from(id))
                .disabled(behavior.disabled)
                .checked(behavior.checked);

            if let Some(index) = behavior.tab_index {
                switch = switch.tab_index(index);
            }
            if let Some(stop) = behavior.tab_stop {
                switch = switch.tab_stop(stop);
            }

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
        Component::Tabs(id) => {
            components::tabs::tab_list(id, refinement, behavior, states, children)
        }
        Component::Tab(id) => {
            components::tabs::tab(runtime, id, refinement, behavior, states, children)
        }
        Component::Progress(id) => {
            components::progress::progress(id, refinement, behavior, states, children)
        }
        Component::ProgressTrack => {
            components::progress::progress_track(refinement, behavior, states, children)
        }
        Component::ProgressIndicator => {
            components::progress::progress_indicator(refinement, behavior, states, children)
        }
        Component::Radio(id) => {
            components::radio::radio(runtime, id, refinement, behavior, states, children)
        }
        Component::Toggle(id) => {
            components::toggle::toggle(runtime, id, refinement, behavior, states, children)
        }
        Component::RadioGroup(id) => {
            components::group::radio_group(id, refinement, behavior, states, children)
        }
        Component::ToggleGroup(id) => {
            components::group::toggle_group(id, refinement, behavior, states, children)
        }
        Component::Table(id) => {
            components::table::table(runtime, id, refinement, behavior, states, children)
        }
        Component::TableHeader(id) => {
            components::table::table_header(runtime, id, refinement, behavior, states, children)
        }
        Component::TableBody(id) => {
            components::table::table_body(runtime, id, refinement, behavior, states, children)
        }
        Component::TableRow(id, row_index) => components::table::table_row(
            runtime, id, row_index, refinement, behavior, states, children,
        ),
        Component::TableHead(id, column_index) => components::table::table_head(
            runtime,
            id,
            column_index,
            refinement,
            behavior,
            states,
            children,
        ),
        Component::TableCell(id, column_index) => components::table::table_cell(
            runtime,
            id,
            column_index,
            refinement,
            behavior,
            states,
            children,
        ),
        Component::TableCaption(id) => {
            components::table::table_caption(runtime, id, refinement, behavior, states, children)
        }
        Component::Collapsible => {
            components::collapsible::collapsible(refinement, behavior, states, slots, children)
        }
        Component::Popover(id) => {
            components::popover::popover(runtime, id, refinement, behavior, states, slots, children)
        }
        Component::HoverCard(id) => components::popover::hover_card(
            runtime, id, refinement, behavior, states, slots, children,
        ),
        Component::Popup(id) => {
            components::popover::popup(runtime, id, refinement, behavior, states, slots, children)
        }
        Component::Select(id) => {
            components::select::select(runtime, id, refinement, behavior, states, children)
        }
        Component::Combobox(id) => {
            components::select::combobox(runtime, id, refinement, behavior, states, children)
        }
        Component::DatePicker(id, focus) => components::select::date_picker(
            runtime, id, focus, refinement, behavior, states, children,
        ),
        Component::Svg(path) => {
            // GPUI paints an svg only when the element's own style carries a
            // text color, and an inherited color reaches children as a text
            // style rather than as this element's style — so an icon with no
            // color of its own silently draws nothing. `inherited` already
            // accounts for this node's own `text_color`, so an explicit color
            // still wins and an icon in a dark button comes out light.
            warn_unhonoured_a11y(&behavior, "svg", &[]);
            let mut image = gpui::svg().path(SharedString::from(path));
            image.style().refine(&refinement);
            image.style().text.color = Some(inherited);
            image.into_any_element()
        }
        Component::Image(path) => {
            warn_unhonoured_a11y(&behavior, "image", &[]);
            let mut image = gpui::img(SharedString::from(path));
            image.style().refine(&refinement);
            image.into_any_element()
        }
        Component::Path {
            fill,
            background,
            stroke_width,
        } => {
            warn_unhonoured_a11y(&behavior, "path", &[]);
            crate::path::NativePath::new(
                fill,
                background,
                stroke_width,
                node.ops().to_vec(),
                refinement,
            )
            .into_any_element()
        }
        Component::Scrollbar(id) => {
            components::scrollbar::scrollbar(id, refinement, behavior, states, children, window, cx)
        }
        Component::Input(handle) => {
            // An input's focus belongs to its `InputState`, which is what
            // `on_mouse_down` below hands it. A second handle on the frame
            // would be a focus target the caret never follows.
            warn_unhonoured_a11y(&behavior, "Input", &[]);
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
        Component::Textarea(handle) => {
            components::textarea::textarea(runtime, handle, refinement, behavior, states, children)
        }
    }
}

fn materializes_slots(component: &Component, behavior: &Behavior) -> bool {
    // A `Popover` or a `HoverCard` is not here, and not by oversight: its
    // `trigger` slot is what is on screen while it is shut, so skipping its
    // slots would remove the one element that has to stay. Skipping only the
    // content needs a per-slot predicate and, for the uncontrolled case, an
    // open state this side does not hold — base owns it.
    !matches!(component, Component::Collapsible) || behavior.open.unwrap_or(false)
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

/// The focus handle a script gave this element, if it is still live.
///
/// A released handle is reported rather than substituted: an element that
/// quietly fell back to base's own keyed focus would still be focusable, so
/// nothing on screen would say that `handle.is_focused()` is now asking about
/// somebody else.
fn tracked_focus(
    runtime: &Rc<ShellRuntime>,
    behavior: &Behavior,
    component: &str,
) -> Option<gpui::FocusHandle> {
    resolve_focus(runtime, behavior.focus_handle, component, "track_focus")
}

/// The handle a `Select` or `Combobox` moves the keyboard to when it opens.
fn content_focus(
    runtime: &Rc<ShellRuntime>,
    behavior: &Behavior,
    component: &str,
) -> Option<gpui::FocusHandle> {
    resolve_focus(
        runtime,
        behavior.content_focus_handle,
        component,
        "content_focus_handle",
    )
}

fn resolve_focus(
    runtime: &Rc<ShellRuntime>,
    handle: Option<crate::entities::EntityHandle>,
    component: &str,
    method: &str,
) -> Option<gpui::FocusHandle> {
    let handle = handle?;
    let resolved = runtime.entities().focus(handle);
    if resolved.is_none() {
        tracing::error!(
            "the focus handle {handle} given to `{method}` on a {component} has been released; \
             the element keeps its own focus instead"
        );
    }
    resolved
}

/// GPUI's own focus builders, for elements that carry no focus wiring of their
/// own.
///
/// Base's controls do — `Button`, `Checkbox`, `Radio`, `Toggle`, `Switch` and
/// `Link` each have an inherent `tab_index` and `tab_stop` feeding the handle
/// they build in their own `render` — so those call theirs instead. Using this
/// on one of them would set a field that `render` overwrites a moment later.
fn with_gpui_focus<E: InteractiveElement>(
    element: E,
    behavior: &Behavior,
    focus: Option<&gpui::FocusHandle>,
) -> E {
    let mut element = element;
    match focus {
        // Once an element tracks a handle, `Interactivity`'s own `tab_index`
        // and `tab_stop` are never read again: GPUI applies them only on the
        // path where it has to create a handle itself (`div.rs`, "ensure we
        // store a focus handle in our element state"). Setting them on the
        // element instead of the handle would leave a tab stop that is not one
        // — the exact silence this method exists to remove — so they go on the
        // handle, which is what base's own controls do with theirs.
        Some(handle) => {
            let mut handle = handle.clone();
            if let Some(index) = behavior.tab_index {
                // A tab index implies a tab stop, as GPUI's own does.
                handle = handle.tab_index(index).tab_stop(true);
            }
            // Second, so an explicit `tab_stop(false)` still wins.
            if let Some(stop) = behavior.tab_stop {
                handle = handle.tab_stop(stop);
            }
            element = element.track_focus(&handle);
        }
        None => {
            if let Some(index) = behavior.tab_index {
                element = element.tab_index(index);
            }
            if let Some(stop) = behavior.tab_stop {
                element = element.tab_stop(stop);
            }
        }
    }
    element
}

/// The accessibility semantics GPUI reads off the element itself.
fn with_aria<E: StatefulInteractiveElement>(element: E, behavior: &Behavior) -> E {
    let mut element = element;
    if let Some(role) = behavior.role {
        element = element.role(role);
    }
    if let Some(selected) = behavior.aria_selected {
        element = element.aria_selected(selected);
    }
    if behavior.aria_active_descendant {
        element = element.aria_active_descendant();
    }
    element
}

/// Reports focus and accessibility calls a component cannot honour.
///
/// Each bound component wires the subset base gives it a builder for; the rest
/// would be set on an `Interactivity` the component's own `render` overwrites,
/// which is a method that looks bound and does nothing. `honoured` lists what
/// this one does wire, so the warning names only what was actually lost.
fn warn_unhonoured_a11y(behavior: &Behavior, component: &str, honoured: &[&str]) {
    let asked = [
        ("track_focus", behavior.focus_handle.is_some()),
        (
            "content_focus_handle",
            behavior.content_focus_handle.is_some(),
        ),
        ("tab_index", behavior.tab_index.is_some()),
        ("tab_stop", behavior.tab_stop.is_some()),
        ("role", behavior.role.is_some()),
        ("aria_selected", behavior.aria_selected.is_some()),
        ("aria_active_descendant", behavior.aria_active_descendant),
    ];
    for (method, called) in asked {
        if called && !honoured.contains(&method) {
            tracing::warn!(
                "`{method}` is not wired on a {component}: base's {component} owns this part \
                 of its own focus and accessibility. Put it on an element around it"
            );
        }
    }
}

/// A plain `div` becomes stateful when a state style needs an identity, or when
/// the script named it.
///
/// A script-given name wins, because it is the only identity that survives the
/// script reordering its own tree. Without one the identity is the node's
/// address in the description, which is stable for as long as the snapshot lives
/// and across rebuilds only while the tree keeps its shape.
// Nine arguments, because a scroll area now also has to reach window element
// state for the position it shares with an explicit `Scrollbar`. Bundling them
// into a struct would move the same nine values one line up and no further.
#[allow(clippy::too_many_arguments)]
fn flex_element(
    runtime: &Rc<ShellRuntime>,
    element: gpui::Div,
    id: SpecId,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let element = with_hover(element, &states);
    if behavior.key.is_none()
        && !states.needs_identity()
        && !behavior.needs_identity()
        && behavior.on_click.is_none()
        && !behavior.scroll_x
        && !behavior.scroll_y
    {
        return finish(element, refinement, children);
    }

    let scroll_x = behavior.scroll_x;
    let scroll_y = behavior.scroll_y;
    // A plain element has no focus or role of its own, so GPUI's builders are
    // the whole of what it can carry — and they are exactly what a script
    // needs to build the thing base has no component for: a listbox option, a
    // toolbar, a composite whose keyboard focus stays on its container.
    let focus = tracked_focus(runtime, &behavior, "div");
    let identity = element_id(id, behavior.key.clone());
    let stateful = element.id(identity.clone());
    let stateful = with_gpui_focus(stateful, &behavior, focus.as_ref());
    let stateful = with_aria(stateful, &behavior);
    let mut stateful = with_active_and_focus(stateful, &states);
    if !behavior.disabled
        && let Some(callback) = behavior.on_click
    {
        let runtime = Rc::downgrade(runtime);
        stateful = stateful.on_click(move |event, window, cx| {
            dispatch_click(&runtime, callback, event, window, cx);
        });
    }
    if behavior.scrollbar {
        let mut stateful = stateful;
        stateful.style().refine(&refinement);
        stateful.extend(children);
        return match (scroll_x, scroll_y) {
            (true, true) => Scrollable::new(stateful, ScrollbarAxis::Both)
                .id(identity)
                .into_any_element(),
            (true, false) => Scrollable::new(stateful, ScrollbarAxis::Horizontal)
                .id(identity)
                .into_any_element(),
            (false, true) => Scrollable::new(stateful, ScrollbarAxis::Vertical)
                .id(identity)
                .into_any_element(),
            (false, false) => stateful.into_any_element(),
        };
    }
    // The scroll position lives in window element state under this element's
    // own identity, which is the slot an explicit `Scrollbar` elsewhere in the
    // tree looks in when the script gives it the same name. The
    // `overflow_*_scrollbar` path above is deliberately left out of this: its
    // `Scrollable` owns a position of its own, on the inner area it builds.
    let stateful = match (scroll_x, scroll_y) {
        (true, true) => {
            components::scrollbar::track_scroll_position(stateful, &identity, window, cx)
                .overflow_scroll()
        }
        (true, false) => {
            components::scrollbar::track_scroll_position(stateful, &identity, window, cx)
                .overflow_x_scroll()
        }
        (false, true) => {
            components::scrollbar::track_scroll_position(stateful, &identity, window, cx)
                .overflow_y_scroll()
        }
        (false, false) => stateful,
    };
    finish(stateful, refinement, children)
}

/// Takes the element filling `name`, leaving any other slot for its own reader.
fn take_slot(slots: &mut Slots, name: &str) -> Option<AnyElement> {
    slots
        .iter()
        .position(|(slot, _)| *slot == name)
        .map(|index| slots.remove(index).1)
}

/// Slots nothing rendered.
///
/// Filling a slot detaches the element from the tree, so a slot the component
/// does not read is not content in the wrong place — it is content that
/// disappears, which is worth more than silence.
fn warn_unread_slots(slots: &Slots, component: &str) {
    for (name, _) in slots.iter() {
        tracing::warn!(
            "{component} has no `{name}` slot, so the element given to it is not rendered \
             at all: a slot element is not drawn as an ordinary child"
        );
    }
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
        | Component::Switch(id)
        | Component::Tabs(id)
        | Component::Tab(id)
        | Component::Progress(id)
        | Component::Radio(id)
        | Component::Toggle(id)
        | Component::RadioGroup(id)
        | Component::ToggleGroup(id)
        | Component::Popover(id)
        | Component::HoverCard(id)
        | Component::Popup(id)
        | Component::Select(id)
        | Component::Combobox(id)
        // An or-pattern has to agree on the bindings it makes, not on how many
        // fields each variant carries, so the picker's focus handle costs it
        // nothing to sit here.
        | Component::DatePicker(id, _) => gpui::ElementId::Name(id.clone().into()),
        // One arm for all seven: an or-pattern only has to agree on the
        // bindings it makes, not on how many fields each variant carries, so
        // the indexed three sit here beside the rest.
        Component::Table(id)
        | Component::TableHeader(id)
        | Component::TableBody(id)
        | Component::TableCaption(id)
        | Component::TableRow(id, _)
        | Component::TableHead(id, _)
        | Component::TableCell(id, _) => gpui::ElementId::Name(id.clone().into()),
        Component::Scrollbar(id) => gpui::ElementId::Name(id.clone().into()),
        Component::Input(handle) => {
            gpui::ElementId::NamedInteger("gpui-shell-input".into(), *handle)
        }
        Component::Textarea(handle) => {
            gpui::ElementId::NamedInteger("gpui-shell-textarea".into(), *handle)
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
    SlotSpecs,
) {
    let mut refinement = StyleRefinement::default();
    let mut behavior = Behavior::default();
    let mut states = StateStyles::default();
    let mut motions = SmallVec::new();
    let mut slots = SlotSpecs::new();

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
            SpecOp::Method(name, args) => {
                if !matches!(
                    *name,
                    "move_to"
                        | "line_to"
                        | "curve_to"
                        | "cubic_bezier_to"
                        | "arc_to"
                        | "close"
                        | "dash_array"
                ) {
                    apply_behavior(&mut behavior, name, args)
                }
            }
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
                "on_open_change" => behavior.on_open_change = Some(*id),
                "on_confirm" => behavior.on_confirm = Some(*id),
                "on_dismiss" => behavior.on_dismiss = Some(*id),
                other => tracing::error!("unhandled callback `{other}` reached materialize"),
            },
            // Filling the same slot twice replaces it, the way a second
            // `open(...)` replaces the first: the last call in the chain is
            // what the script meant.
            SpecOp::Slot(name, slot) => match slots.iter_mut().find(|(held, _)| held == name) {
                Some(existing) => existing.1 = *slot,
                None => slots.push((*name, *slot)),
            },
        }
    }

    (refinement, behavior, states, motions, slots)
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

/// A count of rows or columns, clamped at zero.
///
/// A negative total is not a table one row shorter than it says; it is a
/// number the script computed wrongly, and announcing "row 3 of 0" is the
/// least confusing thing to do with it.
fn whole_count(args: &[Bridged]) -> Option<usize> {
    args.first()
        .and_then(|value| value.as_f32().ok())
        .map(|count| count.max(0.0) as usize)
}

/// A duration a script gave in milliseconds, clamped at zero.
///
/// Milliseconds because that is what `transition` and a toast's `timeout`
/// already take, and a script that has to remember which host call wants
/// seconds has been given two grammars for one idea.
fn milliseconds(args: &[Bridged]) -> Option<Duration> {
    args.first()
        .and_then(|value| value.as_f32().ok())
        .map(|ms| Duration::from_secs_f64(f64::from(ms.max(0.0)) / 1000.0))
}

/// Every anchor name a script may pass to `anchor(...)`, in the order
/// `gpui::Anchor` declares them.
///
/// Written out rather than derived: GPUI has no name table for `Anchor`, and
/// the script API is the snake_case spelling of the variant. One list serves
/// the parser below, the check the prelude makes at the call site, and the
/// union `gpui.d.ts` declares — so the three cannot drift.
pub(crate) const ANCHOR_NAMES: [&str; 8] = [
    "top_left",
    "top_right",
    "bottom_left",
    "bottom_right",
    "top_center",
    "bottom_center",
    "left_center",
    "right_center",
];

/// The anchor a script named, or `None` if no variant spells it.
fn anchor_from_name(name: &str) -> Option<gpui::Anchor> {
    match name {
        "top_left" => Some(gpui::Anchor::TopLeft),
        "top_right" => Some(gpui::Anchor::TopRight),
        "bottom_left" => Some(gpui::Anchor::BottomLeft),
        "bottom_right" => Some(gpui::Anchor::BottomRight),
        "top_center" => Some(gpui::Anchor::TopCenter),
        "bottom_center" => Some(gpui::Anchor::BottomCenter),
        "left_center" => Some(gpui::Anchor::LeftCenter),
        "right_center" => Some(gpui::Anchor::RightCenter),
        _ => None,
    }
}

/// Reports the styling and the children a component with no surface of its own
/// cannot take.
///
/// `Popover` and `HoverCard` are behavior, not boxes: neither implements
/// `Styled` or `ParentElement`, because what is on screen is the trigger and
/// the content and the script owns both. A style call here is not a style in
/// the wrong place — it is a style that disappears.
fn warn_without_surface(
    component: &str,
    refinement: &StyleRefinement,
    states: &StateStyles,
    children: &Children,
) {
    if *refinement != StyleRefinement::default() {
        tracing::warn!(
            "styles on a {component} are ignored: it has no box of its own. Style the element \
             you gave to `trigger`, or the one you gave to `content`"
        );
    }
    if states.hover.is_some() || states.active.is_some() || states.focus.is_some() {
        tracing::warn!(
            "state styles on a {component} are ignored; put them on the element you gave to \
             `trigger`"
        );
    }
    if !children.is_empty() {
        tracing::warn!(
            "a {component} renders its `trigger` and `content` slots and nothing else, so the \
             {} ordinary children given to it are dropped",
            children.len()
        );
    }
}

/// Reports builder methods a component does not have.
///
/// Every element shares one prototype, so a delay reaches a popover and an
/// overlay reaches a hover card. Dropping the call in silence would leave a
/// popover that never opens on the right button looking like a positioning bug.
fn warn_unsupported(component: &str, methods: &[(&str, bool)]) {
    for (method, called) in methods {
        if *called {
            tracing::warn!(
                "`{method}` is not a {component} method: base's {component} has no such builder, \
                 so the call is dropped"
            );
        }
    }
}

fn apply_behavior(behavior: &mut Behavior, name: &str, args: &[Bridged]) {
    let flag = args.first().map(Bridged::is_truthy);
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
        "value" => behavior.value = args.first().and_then(|value| value.as_f32().ok()),
        "indeterminate" => behavior.indeterminate = flag.unwrap_or(true),
        "pressed" => behavior.pressed = flag.unwrap_or(true),
        "overflow_scroll" => {
            behavior.scroll_x = true;
            behavior.scroll_y = true;
        }
        "overflow_x_scroll" => behavior.scroll_x = true,
        "overflow_y_scroll" => behavior.scroll_y = true,
        "overflow_scrollbar" => {
            behavior.scroll_x = true;
            behavior.scroll_y = true;
            behavior.scrollbar = true;
        }
        "overflow_x_scrollbar" => {
            behavior.scroll_x = true;
            behavior.scrollbar = true;
        }
        "overflow_y_scrollbar" => {
            behavior.scroll_y = true;
            behavior.scrollbar = true;
        }
        "mode" => {
            // Only an explicit mode is recorded: unset means the theme's own
            // projection, which is what the rest of the application follows.
            behavior.scrollbar_mode =
                args.first()
                    .and_then(|value| value.as_str().ok())
                    .and_then(|value| match value {
                        "scrolling" => Some(gpui_base::ScrollbarMode::Scrolling),
                        "hover" => Some(gpui_base::ScrollbarMode::Hover),
                        "always" => Some(gpui_base::ScrollbarMode::Always),
                        _ => None,
                    });
        }
        "scroll_size" => {
            // Both halves or neither: one axis sized by the script and the
            // other by the scroll area is a thumb that lies about one of them.
            let width = args.first().and_then(|value| value.as_f32().ok());
            let height = args.get(1).and_then(|value| value.as_f32().ok());
            if let (Some(width), Some(height)) = (width, height) {
                behavior.scroll_size = Some(gpui::size(gpui::px(width), gpui::px(height)));
            }
        }
        "viewport_from_layout" => behavior.viewport_from_layout = true,
        "set_position" => {
            // Both halves or neither: "tab 2 of" announces nothing a reader
            // can place, so a malformed pair is dropped rather than halved.
            let position = args.first().and_then(|value| value.as_f32().ok());
            let size = args.get(1).and_then(|value| value.as_f32().ok());
            if let (Some(position), Some(size)) = (position, size) {
                behavior.position_in_set = Some((position as usize, size as usize));
            }
        }
        "axis" => {
            // Only an explicit value is recorded: each grouping container has
            // its own default orientation, and substituting one of ours would
            // be a choice base did not make.
            behavior.axis = args
                .first()
                .and_then(|value| value.as_str().ok())
                .and_then(|value| match value {
                    "horizontal" => Some(gpui::Axis::Horizontal),
                    "vertical" => Some(gpui::Axis::Vertical),
                    _ => None,
                });
        }
        // A handle crosses the bridge as a JavaScript number, which is exact
        // to 53 bits — the layout `EntityHandle` was chosen for. Anything else
        // reaching here is a host bug, and dropping it is what makes the
        // element fall back to base's own keyed focus rather than track a
        // handle nobody named.
        "track_focus" => {
            behavior.focus_handle = match args.first() {
                Some(Bridged::Number(handle)) if *handle >= 0.0 && handle.fract() == 0.0 => {
                    Some(*handle as crate::entities::EntityHandle)
                }
                _ => None,
            };
        }
        "content_focus_handle" => {
            behavior.content_focus_handle = match args.first() {
                Some(Bridged::Number(handle)) if *handle >= 0.0 && handle.fract() == 0.0 => {
                    Some(*handle as crate::entities::EntityHandle)
                }
                _ => None,
            };
        }
        "tab_index" => {
            behavior.tab_index = args
                .first()
                .and_then(|value| value.as_f32().ok())
                .map(|index| index as isize);
        }
        "tab_stop" => behavior.tab_stop = Some(flag.unwrap_or(true)),
        "role" => {
            behavior.role = args
                .first()
                .and_then(|value| value.as_str().ok())
                .and_then(crate::a11y::role_from_name);
        }
        "aria_selected" => behavior.aria_selected = Some(flag.unwrap_or(true)),
        "aria_active_descendant" => behavior.aria_active_descendant = true,
        "row_count" => behavior.row_count = whole_count(args),
        "column_count" => behavior.column_count = whole_count(args),
        "open" => behavior.open = Some(flag.unwrap_or(true)),
        "default_open" => behavior.default_open = flag.unwrap_or(true),
        "overlay_closable" => behavior.overlay_closable = Some(flag.unwrap_or(true)),
        // Only an explicit value is recorded, as with `axis`: a popover anchors
        // top-left and a hover card top-center, and substituting one of ours
        // would be a choice base did not make.
        "anchor" => {
            behavior.anchor = args
                .first()
                .and_then(|value| value.as_str().ok())
                .and_then(|value| anchor_from_name(value));
        }
        "mouse_button" => {
            behavior.mouse_button =
                args.first()
                    .and_then(|value| value.as_str().ok())
                    .and_then(|value| match value {
                        "left" => Some(MouseButton::Left),
                        "right" => Some(MouseButton::Right),
                        "middle" => Some(MouseButton::Middle),
                        _ => None,
                    });
        }
        "open_delay" => behavior.open_delay = milliseconds(args),
        "close_delay" => behavior.close_delay = milliseconds(args),
        _ => tracing::error!("unhandled component method `{name}` reached materialize"),
    }
}

#[cfg(test)]
mod motion_identity_tests {
    use super::*;

    #[test]
    fn a_closed_collapsible_does_not_materialize_its_content_slot() {
        let mut behavior = Behavior::default();
        assert!(!materializes_slots(&Component::Collapsible, &behavior));

        behavior.open = Some(true);
        assert!(materializes_slots(&Component::Collapsible, &behavior));
        assert!(materializes_slots(&Component::Div, &Behavior::default()));
    }

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

    /// The accessibility calls land on the fields GPUI reads them back from.
    ///
    /// Asserted on the concrete element rather than end to end, because
    /// `AnyElement` does not forward `a11y_role` or `write_a11y_info` — it
    /// captures them while prepainting into a tree a test outside GPUI cannot
    /// reach. This is the level base tests its own controls at, and for the
    /// same reason.
    #[test]
    fn accessibility_calls_land_where_gpui_reads_them() {
        use gpui::Element as _;

        let mut behavior = Behavior::default();
        apply_behavior(
            &mut behavior,
            "role",
            &[Bridged::Str("list_box_option".to_owned())],
        );
        apply_behavior(&mut behavior, "aria_selected", &[Bridged::Bool(true)]);
        apply_behavior(&mut behavior, "aria_active_descendant", &[]);

        let element = with_aria(div().id("option"), &behavior);
        assert_eq!(element.a11y_role(), Some(gpui::Role::ListBoxOption));

        let mut node = gpui::accesskit::Node::new(gpui::Role::Unknown);
        element.write_a11y_info(&mut node);
        assert_eq!(node.is_selected(), Some(true));
    }

    /// A tracked handle carries the tab order, because the element stops
    /// carrying it the moment there is a handle to carry it instead.
    #[gpui::test]
    fn a_tracked_handle_carries_the_tab_order_the_element_would_have(
        cx: &mut gpui::TestAppContext,
    ) {
        let handle = cx.update(|cx| cx.focus_handle());

        let mut behavior = Behavior::default();
        apply_behavior(&mut behavior, "tab_index", &[Bridged::Number(3.0)]);
        let _ = with_gpui_focus(div().id("custom"), &behavior, Some(&handle));
        // Cloning re-reads the window's focus map, which is where the builders
        // write and where the tab order is read from.
        assert_eq!(handle.clone().tab_index, 3);
        assert!(
            handle.clone().tab_stop,
            "a tab index implies a tab stop, as GPUI's own does"
        );

        // And an explicit refusal still wins over that implication.
        let mut behavior = Behavior::default();
        apply_behavior(&mut behavior, "tab_index", &[Bridged::Number(3.0)]);
        apply_behavior(&mut behavior, "tab_stop", &[Bridged::Bool(false)]);
        let _ = with_gpui_focus(div().id("custom"), &behavior, Some(&handle));
        assert!(!handle.clone().tab_stop);
    }

    /// One written list of anchors serves the parser, the check the prelude
    /// makes at the call site and the union in `gpui.d.ts`. A declared name
    /// that does not parse is a name a script could type and the runtime would
    /// silently drop.
    #[test]
    fn every_declared_anchor_name_parses_back_to_a_variant() {
        for name in ANCHOR_NAMES {
            assert!(
                anchor_from_name(name).is_some(),
                "`{name}` is declared but does not parse"
            );
        }
        assert_eq!(anchor_from_name("top_left"), Some(gpui::Anchor::TopLeft));
        assert_eq!(
            anchor_from_name("right_center"),
            Some(gpui::Anchor::RightCenter)
        );
        assert_eq!(anchor_from_name("topLeft"), None);
        assert_eq!(anchor_from_name(""), None);
    }

    /// `open` is three-valued because a popover is uncontrolled until the
    /// script says otherwise, and "never said" has to be distinguishable from
    /// "said false" for `default_open` to mean anything.
    #[test]
    fn an_unsaid_open_state_is_not_a_closed_one() {
        let mut behavior = Behavior::default();
        assert_eq!(behavior.open, None);

        apply_behavior(&mut behavior, "open", &[Bridged::Bool(false)]);
        assert_eq!(behavior.open, Some(false));
    }

    #[test]
    fn a_delay_crosses_the_bridge_in_milliseconds() {
        let mut behavior = Behavior::default();
        apply_behavior(&mut behavior, "open_delay", &[Bridged::Number(250.0)]);
        assert_eq!(behavior.open_delay, Some(Duration::from_millis(250)));

        // A negative delay is a number the script computed wrongly, not a
        // request to open before the pointer arrives.
        apply_behavior(&mut behavior, "close_delay", &[Bridged::Number(-5.0)]);
        assert_eq!(behavior.close_delay, Some(Duration::ZERO));
    }

    #[test]
    fn boolean_behaviors_use_javascript_truthiness() {
        for falsy in [
            Bridged::Nil,
            Bridged::Bool(false),
            Bridged::Number(0.0),
            Bridged::Number(f64::NAN),
            Bridged::Str(String::new()),
        ] {
            let mut behavior = Behavior::default();
            apply_behavior(&mut behavior, "disabled", &[falsy]);
            assert!(!behavior.disabled);
        }

        for truthy in [
            Bridged::Bool(true),
            Bridged::Number(1.0),
            Bridged::Str("false".to_owned()),
        ] {
            let mut behavior = Behavior::default();
            apply_behavior(&mut behavior, "disabled", &[truthy]);
            assert!(behavior.disabled);
        }
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

/// A callback with nothing to report but that it happened.
///
/// `on_confirm` and `on_dismiss` carry no payload: the combobox root holds no
/// value, so the only news is the action itself. The script still receives the
/// `(payload, cx)` pair every other handler takes, with an empty payload, so
/// that a handler written as `(_, cx) => …` reads the same everywhere.
fn dispatch_signal(
    runtime: &Weak<ShellRuntime>,
    callback: CallbackId,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(runtime) = runtime.upgrade() else {
        return;
    };
    runtime.dispatch_signal(callback, window, cx);
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
