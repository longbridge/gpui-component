use std::{cell::Cell, rc::Rc};

use gpui::{
    Anchor, AnyElement, App, Bounds, Context, DismissEvent, ElementId, EventEmitter, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement as _,
    PathBuilder, Pixels, Point, Render, RenderOnce, Role, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Subscription, Window, canvas, div, point, prelude::FluentBuilder as _,
    px,
};

use crate::{
    DeferredPopover, GlobalState, Placement, Popup, ResolvedPosition, Selectable, StyledExt as _,
    actions::{Cancel, Confirm},
};

const CONTEXT: &str = "Popover";
const CONTENT_CONTEXT: &str = "PopoverContent";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("space", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTENT_CONTEXT)),
        // The content is still a descendant of the trigger's dispatch node.
        // Leave activation to its controls, not the trigger or an outer dialog.
        // More deeply scoped input bindings retain precedence.
        KeyBinding::new("enter", gpui::NoAction, Some(CONTENT_CONTEXT)),
        KeyBinding::new("space", gpui::NoAction, Some(CONTENT_CONTEXT)),
    ]);
}

type OpenChangeHandler = Rc<dyn Fn(&bool, &mut Window, &mut App)>;

/// State and focus lifecycle for an unstyled popover.
///
/// Applications own trigger/content construction, positioning, appearance, and
/// animation. This state owns the controlled open lifecycle, dismissal, focus
/// capture and restoration, and deferred-popup registration.
pub struct PopoverState {
    focus_handle: FocusHandle,
    tracked_focus_handle: Option<FocusHandle>,
    previous_focus_handle: Option<FocusHandle>,
    open: bool,
    on_open_change: Option<OpenChangeHandler>,
    dismiss_subscription: Option<Subscription>,
    /// Held while open, so that a state collected without being closed — this
    /// lives in element state — takes its registration with it.
    deferred_context: Option<DeferredPopover>,
}

impl PopoverState {
    pub fn new(default_open: bool, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            tracked_focus_handle: None,
            previous_focus_handle: None,
            open: default_open,
            on_open_change: None,
            dismiss_subscription: None,
            deferred_context: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.toggle_open(window, cx);
        }
    }

    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.toggle_open(window, cx);
        }
    }

    #[doc(hidden)]
    pub fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        self.open = open;
        self.deferred_context = open.then(|| GlobalState::register_deferred_popover(cx));
    }

    #[doc(hidden)]
    pub fn sync_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.transition_to(open, false, window, cx);
    }

    #[doc(hidden)]
    pub fn toggle_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.transition_to(!self.open, true, window, cx);
    }

    fn transition_to(
        &mut self,
        opening: bool,
        announce: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.open == opening {
            return;
        }
        if opening {
            self.previous_focus_handle = window.focused(cx);
        }
        self.set_open(opening, cx);

        if self.open {
            // Weak: the subscription is stored on this state, so a strong
            // handle would keep the state, and its deferred-popover
            // registration, alive after its trigger is gone.
            let state = cx.entity().downgrade();
            self.tracked_focus_handle
                .clone()
                .unwrap_or_else(|| self.focus_handle.clone())
                .focus(window, cx);

            self.dismiss_subscription =
                Some(
                    window.subscribe(&cx.entity(), cx, move |_, _: &DismissEvent, window, cx| {
                        _ = state.update(cx, |state, cx| state.dismiss(window, cx));
                        window.refresh();
                    }),
                );
        } else {
            self.dismiss_subscription = None;
            if let Some(previous) = self.previous_focus_handle.take() {
                if self.focus_handle.contains_focused(window, cx) {
                    previous.focus(window, cx);
                }
            }
        }

        if announce && let Some(callback) = self.on_open_change.as_ref() {
            callback(&opening, window, cx);
        }
        cx.notify();
    }

    #[doc(hidden)]
    pub fn track_focus(&mut self, focus_handle: Option<FocusHandle>) {
        self.tracked_focus_handle = focus_handle;
    }

    #[doc(hidden)]
    pub fn set_on_open_change(&mut self, handler: Option<OpenChangeHandler>) {
        self.on_open_change = handler;
    }

    #[doc(hidden)]
    pub fn on_action_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss(window, cx);
    }
}

impl Focusable for PopoverState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopoverState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl EventEmitter<DismissEvent> for PopoverState {}

type TriggerBuilder = Box<dyn FnOnce(bool, &Window, &App) -> AnyElement>;
type ContentBuilder =
    Box<dyn FnOnce(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> AnyElement>;

/// An unstyled popover owning trigger interaction, open state, dismissal, and focus.
///
/// Enter and Space on the trigger toggle the popover. Inside its content, those
/// keys belong to the focused control instead; Escape dismisses the popover and
/// restores the previous focus.
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    style: StyleRefinement,
    anchor: Anchor,
    offset: gpui::Pixels,
    on_position: Option<Box<dyn Fn(ResolvedPosition, gpui::Bounds<gpui::Pixels>)>>,
    default_open: bool,
    open: Option<bool>,
    tracked_focus_handle: Option<FocusHandle>,
    trigger: Option<TriggerBuilder>,
    content: Option<ContentBuilder>,
    mouse_button: MouseButton,
    overlay_closable: bool,
    disabled: bool,
    arrow: Option<PopupArrow>,
    on_open_change: Option<OpenChangeHandler>,
}

impl Popover {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            anchor: Anchor::TopLeft,
            offset: gpui::px(0.),
            on_position: None,
            default_open: false,
            open: None,
            tracked_focus_handle: None,
            trigger: None,
            content: None,
            mouse_button: MouseButton::Left,
            overlay_closable: true,
            disabled: false,
            arrow: None,
            on_open_change: None,
        }
    }

    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Gap from the trigger along the anchor's outward direction, zero by default.
    pub fn offset(mut self, offset: gpui::Pixels) -> Self {
        self.offset = offset;
        self
    }

    /// Observe geometry to supply presentation such as a pointer arrow.
    pub fn on_position(
        mut self,
        callback: impl Fn(ResolvedPosition, gpui::Bounds<gpui::Pixels>) + 'static,
    ) -> Self {
        self.on_position = Some(Box::new(callback));
        self
    }

    pub fn mouse_button(mut self, mouse_button: MouseButton) -> Self {
        self.mouse_button = mouse_button;
        self
    }

    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: Selectable + IntoElement + 'static,
    {
        self.trigger = Some(Box::new(|is_open, _, _| {
            let open = trigger.is_open();
            trigger.open(open || is_open).into_any_element()
        }));
        self
    }

    /// Supplies a trigger builder for higher-level presentation facades.
    #[doc(hidden)]
    pub fn trigger_with(
        mut self,
        trigger: impl FnOnce(bool, &Window, &App) -> AnyElement + 'static,
    ) -> Self {
        self.trigger = Some(Box::new(trigger));
        self
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    pub fn track_focus(mut self, handle: &FocusHandle) -> Self {
        self.tracked_focus_handle = Some(handle.clone());
        self
    }

    pub fn overlay_closable(mut self, closable: bool) -> Self {
        self.overlay_closable = closable;
        self
    }

    /// Ignores the trigger's presses and keys and keeps the popover closed,
    /// even when [`Popover::open`] asks for it. Disabling the trigger itself
    /// stays the composer's.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Draws `arrow` on the content's edge, aimed at the point of the trigger
    /// the anchor names. The arrow sits outside the content, so include its
    /// depth in [`Popover::offset`].
    pub fn arrow(mut self, arrow: PopupArrow) -> Self {
        self.arrow = Some(arrow);
        self
    }

    pub fn on_open_change(
        mut self,
        callback: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    pub fn content<F, E>(mut self, content: F) -> Self
    where
        E: IntoElement,
        F: FnOnce(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> E + 'static,
    {
        self.content = Some(Box::new(move |state, window, cx| {
            content(state, window, cx).into_any_element()
        }));
        self
    }
}

/// Styles the trigger container: the element that takes part in the parent
/// layout and whose bounds the popup is anchored to.
impl Styled for Popover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| {
            PopoverState::new(self.default_open, cx)
        });
        state.update(cx, |state, cx| {
            state.track_focus(self.tracked_focus_handle);
            state.set_on_open_change(self.on_open_change);
            if let Some(open) = self.open {
                state.sync_open(open, window, cx);
            }
            if self.disabled {
                state.sync_open(false, window, cx);
            }
        });

        let open = state.read(cx).is_open();
        let focus_handle = state.read(cx).focus_handle(cx);
        let Some(trigger) = self.trigger else {
            return div().id("empty").into_any_element();
        };
        let parent_view_id = window.current_view();
        let trigger_bounds = Rc::new(Cell::new(None));
        let on_position = self.on_position;
        let popup = Popup::new(self.id, trigger(open, window, cx))
            .refine_style(&self.style)
            .anchor(self.anchor)
            .offset(self.offset)
            .when(on_position.is_some() || self.arrow.is_some(), |this| {
                let trigger_bounds = trigger_bounds.clone();
                this.on_position(move |position, trigger| {
                    trigger_bounds.set(Some(trigger));
                    if let Some(callback) = &on_position {
                        callback(position, trigger);
                    }
                })
            })
            .key_context(CONTEXT)
            .when(!self.disabled, |this| {
                this.on_action({
                    let state = state.clone();
                    move |_: &Confirm, window, cx| {
                        state.update(cx, |state, cx| state.toggle_open(window, cx));
                        cx.notify(parent_view_id);
                    }
                })
                .on_mouse_down(self.mouse_button, {
                    let state = state.clone();
                    move |_, window, cx| {
                        cx.stop_propagation();
                        state.update(cx, |state, cx| {
                            if state.is_open() == open {
                                state.toggle_open(window, cx);
                            }
                        });
                        cx.notify(parent_view_id);
                    }
                })
            });
        if !open {
            return popup.into_any_element();
        }

        let content = div()
            .id("content")
            // A popover surface is a non-modal dialog: it takes focus and is
            // dismissed with Escape, which is what this role tells assistive
            // technology to expect.
            .role(Role::Dialog)
            .occlude()
            .tab_group()
            .track_focus(&focus_handle)
            .key_context(CONTENT_CONTEXT)
            .on_action(window.listener_for(&state, PopoverState::on_action_cancel))
            .when_some(self.content, |this, content| {
                this.child(state.update(cx, |state, cx| (content)(state, window, cx)))
            })
            .when_some(self.arrow, |this, arrow| {
                this.child(arrow.aimed(self.anchor, trigger_bounds))
            })
            .when(self.overlay_closable, |this| {
                this.on_mouse_down_out({
                    let state = state.clone();
                    move |_, window, cx| {
                        state.update(cx, |state, cx| state.dismiss(window, cx));
                        cx.notify(parent_view_id);
                    }
                })
            });
        popup.content(content).into_any_element()
    }
}

/// Where a popup's arrow sits: on which edge of the content, and the three
/// points of its triangle.
///
/// The arrow aims at a point of the trigger. Its base keeps clear of the
/// content's rounded corners, so on a content wider than the trigger the tip
/// may stop short of the point it aims at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowGeometry {
    side: Placement,
    points: [Point<Pixels>; 3],
}

impl ArrowGeometry {
    /// The arrow of content on the `side` of the trigger (its tip points back
    /// at it), aimed at `target`, `depth` deep, whose base keeps
    /// `corner_radius` from the corners of `content`.
    pub fn new(
        side: Placement,
        content: Bounds<Pixels>,
        target: Point<Pixels>,
        depth: Pixels,
        corner_radius: Pixels,
    ) -> Self {
        let (start, end, target) = if side.is_horizontal() {
            (content.top(), content.bottom(), target.y)
        } else {
            (content.left(), content.right(), target.x)
        };
        let half = depth.min((end - start) * 0.5);
        let inset = (corner_radius + half).min((end - start) * 0.5);
        let center = target.clamp(start + inset, end - inset);
        let points = match side {
            Placement::Bottom => [
                point(center - half, content.top()),
                point(center, content.top() - depth),
                point(center + half, content.top()),
            ],
            Placement::Top => [
                point(center - half, content.bottom()),
                point(center, content.bottom() + depth),
                point(center + half, content.bottom()),
            ],
            Placement::Right => [
                point(content.left(), center - half),
                point(content.left() - depth, center),
                point(content.left(), center + half),
            ],
            Placement::Left => [
                point(content.right(), center - half),
                point(content.right() + depth, center),
                point(content.right(), center + half),
            ],
        };
        Self { side, points }
    }

    /// The arrow of content anchored by `anchor` (as [`Popover::anchor`] and
    /// [`crate::HoverCard::anchor`] name it) to `trigger`: it aims at the
    /// point of the trigger the anchor names, not at the trigger's center.
    pub fn anchored(
        anchor: Anchor,
        content: Bounds<Pixels>,
        trigger: Bounds<Pixels>,
        depth: Pixels,
        corner_radius: Pixels,
    ) -> Self {
        let (side, target) = Self::aim(anchor, trigger);
        Self::new(side, content, target, depth, corner_radius)
    }

    /// The side of the trigger content anchored by `anchor` opens on, and the
    /// point of the trigger its arrow aims at.
    fn aim(anchor: Anchor, trigger: Bounds<Pixels>) -> (Placement, Point<Pixels>) {
        match anchor {
            Anchor::TopLeft => (Placement::Bottom, trigger.bottom_left()),
            Anchor::TopCenter => (Placement::Bottom, trigger.bottom_center()),
            Anchor::TopRight => (Placement::Bottom, trigger.bottom_right()),
            Anchor::BottomLeft => (Placement::Top, trigger.origin),
            Anchor::BottomCenter => (Placement::Top, trigger.top_center()),
            Anchor::BottomRight => (Placement::Top, trigger.top_right()),
            Anchor::LeftCenter => (Placement::Right, trigger.right_center()),
            Anchor::RightCenter => (Placement::Left, trigger.left_center()),
        }
    }

    /// The side of the trigger the content is on; the arrow sits on the
    /// content's opposite edge (on its top edge for [`Placement::Bottom`]).
    pub fn side(&self) -> Placement {
        self.side
    }

    /// The base's first corner, the tip, and the base's second corner.
    pub fn points(&self) -> [Point<Pixels>; 3] {
        self.points
    }

    /// The tip.
    pub fn tip(&self) -> Point<Pixels> {
        self.points[1]
    }

    /// The bounds of the triangle.
    pub fn bounds(&self) -> Bounds<Pixels> {
        let [a, b, c] = self.points;
        Bounds::from_corners(a.min(&b).min(&c), a.max(&b).max(&c))
    }

    /// A patch over the join of the base and the content's edge, `stroke`
    /// past the edge on both sides and inset by it from the slopes. Filled
    /// with the arrow's color, it hides the content's border under the base.
    fn join_bounds(&self, stroke: Pixels) -> Bounds<Pixels> {
        let points = self.points;
        if self.side.is_horizontal() {
            let inset = stroke.min((points[2].y - points[0].y) * 0.5);
            Bounds::from_corners(
                point(points[0].x - stroke, points[0].y + inset),
                point(points[2].x + stroke, points[2].y - inset),
            )
        } else {
            let inset = stroke.min((points[2].x - points[0].x) * 0.5);
            Bounds::from_corners(
                point(points[0].x + inset, points[0].y - stroke),
                point(points[2].x - inset, points[2].y + stroke),
            )
        }
    }
}

/// An unstyled arrow from a popup's content to its trigger, given to
/// [`Popover::arrow`] or [`crate::HoverCard::arrow`].
///
/// Base owns its geometry ([`ArrowGeometry`]); it draws nothing until the
/// composer styles it. Its background fills the triangle, joined to the
/// content's edge so the content's border does not cross the base, and its
/// border color strokes the two slopes one pixel wide. Other styles are
/// ignored.
pub struct PopupArrow {
    depth: Pixels,
    corner_radius: Pixels,
    style: StyleRefinement,
}

impl PopupArrow {
    /// An arrow reaching `depth` out of the content, its base twice as wide.
    pub fn new(depth: Pixels) -> Self {
        Self {
            depth,
            corner_radius: px(0.),
            style: StyleRefinement::default(),
        }
    }

    /// The radius of the content's corners, which the base keeps clear of.
    pub fn corner_radius(mut self, radius: Pixels) -> Self {
        self.corner_radius = radius;
        self
    }

    /// The element drawing the arrow over its parent, the content's
    /// container, aimed by `anchor` at the trigger bounds the popup reports.
    pub(crate) fn aimed(
        self,
        anchor: Anchor,
        trigger: Rc<Cell<Option<Bounds<Pixels>>>>,
    ) -> impl IntoElement {
        let Self {
            depth,
            corner_radius,
            style,
        } = self;
        let fill = style.background.as_ref().and_then(gpui::Fill::color);
        let stroke = style.border_color;
        canvas(
            |_, _, _| {},
            move |content, _, window, _| {
                let Some(trigger) = trigger.get() else {
                    return;
                };
                let geometry =
                    ArrowGeometry::anchored(anchor, content, trigger, depth, corner_radius);
                let [base_start, tip, base_end] = geometry.points();
                if let Some(fill) = fill {
                    let mut path = PathBuilder::fill();
                    path.move_to(base_start);
                    path.line_to(tip);
                    path.line_to(base_end);
                    path.close();
                    if let Ok(path) = path.build() {
                        window.paint_path(path, fill);
                    }
                    // The triangle ends exactly at the content's edge: cover
                    // its border and the antialiased base on both sides.
                    window.paint_quad(gpui::fill(geometry.join_bounds(px(1.)), fill));
                }
                if let Some(color) = stroke {
                    let mut path = PathBuilder::stroke(px(1.));
                    path.move_to(base_start);
                    path.line_to(tip);
                    path.line_to(base_end);
                    if let Ok(path) = path.build() {
                        window.paint_path(path, color);
                    }
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    }
}

/// Styles the arrow: its background and border color.
impl Styled for PopupArrow {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, Context, Render, point, px};
    use std::{cell::RefCell, rc::Rc};

    /// Popover state lives in element state, which is collected as soon as it
    /// stops being rendered — a trigger scrolled out of a virtual list, a panel
    /// closed with its menu open. A registration that outlived it would leave
    /// the application believing a popup is open for the rest of the session.
    #[gpui::test]
    fn a_state_dropped_while_open_closes_the_deferred_context(cx: &mut gpui::TestAppContext) {
        let state = cx.update(|cx| {
            GlobalState::init(cx);
            let state = cx.new(|cx| PopoverState::new(false, cx));
            state.update(cx, |state, cx| state.set_open(true, cx));
            assert!(GlobalState::is_in_deferred_context(cx));
            state
        });

        cx.update(|_| drop(state));
        cx.update(|cx| assert!(!GlobalState::is_in_deferred_context(cx)));
    }

    #[gpui::test]
    fn open_state_registers_and_unregisters_deferred_context(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            GlobalState::init(cx);
            let state = cx.new(|cx| PopoverState::new(false, cx));

            state.update(cx, |state, cx| state.set_open(true, cx));
            assert!(state.read(cx).is_open());
            assert!(GlobalState::is_in_deferred_context(cx));

            state.update(cx, |state, cx| state.set_open(false, cx));
            assert!(!state.read(cx).is_open());
            assert!(!GlobalState::is_in_deferred_context(cx));
        });
    }

    struct PopoverHarness {
        changes: Rc<RefCell<Vec<bool>>>,
        default_open: bool,
    }

    struct KeyboardPopoverHarness {
        trigger_focus: FocusHandle,
    }

    impl Render for KeyboardPopoverHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Popover::new("keyboard-popover")
                .trigger(
                    crate::Button::new("keyboard-trigger")
                        .track_focus(&self.trigger_focus)
                        .child("Open"),
                )
                .content(|_, _, _| {
                    div()
                        .debug_selector(|| "keyboard-popover-content".into())
                        .size(px(40.))
                })
        }
    }

    impl Render for PopoverHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let changes = self.changes.clone();
            Popover::new("base-popover")
                .default_open(self.default_open)
                .trigger_with(|_, _, _| div().child("Open").into_any_element())
                .content(|_, _, _| {
                    div()
                        .debug_selector(|| "base-popover-content".into())
                        .size(px(40.))
                })
                .on_open_change(move |open, _, _| changes.borrow_mut().push(*open))
        }
    }

    #[gpui::test]
    fn unstyled_popover_owns_pointer_open_and_outside_dismiss(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let changes = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let changes = changes.clone();
            move |_, _| PopoverHarness {
                changes,
                default_open: false,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        cx.simulate_click(point(px(20.), px(10.)), Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("base-popover-content").is_some());

        cx.simulate_click(point(px(300.), px(300.)), Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("base-popover-content").is_none());
        assert_eq!(&*changes.borrow(), &[true, false]);
    }

    #[gpui::test]
    fn default_open_renders_content_without_activation(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, _| PopoverHarness {
            changes: Rc::new(RefCell::new(Vec::new())),
            default_open: true,
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("base-popover-content").is_some());
    }

    /// A trigger that keeps "open" and "selected" apart, the way a downstream
    /// sidebar row does: it is selected when it is the current view, and open
    /// only while its popover is showing.
    #[derive(IntoElement)]
    struct RecordingTrigger {
        calls: Rc<RefCell<Vec<(&'static str, bool)>>>,
        selected: bool,
        open: bool,
    }

    impl Selectable for RecordingTrigger {
        fn selected(mut self, selected: bool) -> Self {
            self.calls.borrow_mut().push(("selected", selected));
            self.selected = selected;
            self
        }

        fn is_selected(&self) -> bool {
            self.selected
        }

        fn open(mut self, open: bool) -> Self {
            self.calls.borrow_mut().push(("open", open));
            self.open = open;
            self
        }

        fn is_open(&self) -> bool {
            self.open
        }
    }

    impl RenderOnce for RecordingTrigger {
        fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
            div().size(px(40.)).child("Open")
        }
    }

    struct RecordingTriggerHarness {
        calls: Rc<RefCell<Vec<(&'static str, bool)>>>,
    }

    impl Render for RecordingTriggerHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Popover::new("recording-popover")
                .trigger(RecordingTrigger {
                    calls: self.calls.clone(),
                    selected: false,
                    open: false,
                })
                .content(|_, _, _| div().size(px(40.)))
        }
    }

    #[gpui::test]
    fn an_open_popover_tells_its_trigger_it_is_open_not_selected(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let calls = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let calls = calls.clone();
            move |_, _| RecordingTriggerHarness { calls }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        calls.borrow_mut().clear();

        cx.simulate_click(point(px(20.), px(10.)), Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let calls = calls.borrow();
        assert!(
            calls.contains(&("open", true)),
            "an open popover marks its trigger open, got {calls:?}"
        );
        assert!(
            !calls.iter().any(|(name, _)| *name == "selected"),
            "opening must not touch the trigger's own selection, got {calls:?}"
        );
    }

    #[gpui::test]
    fn keyboard_activation_opens_the_popover(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|_, cx| KeyboardPopoverHarness {
            trigger_focus: cx.focus_handle(),
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        cx.update(|window, cx| {
            let focus = view.read(cx).trigger_focus.clone();
            focus.focus(window, cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| window.draw(cx).clear(cx));

        assert!(cx.debug_bounds("keyboard-popover-content").is_some());
    }

    struct DisabledHarness {
        open: Option<bool>,
        changes: Rc<RefCell<Vec<bool>>>,
    }

    impl Render for DisabledHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let changes = self.changes.clone();
            Popover::new("disabled-popover")
                .disabled(true)
                .when_some(self.open, |this, open| this.open(open))
                .trigger_with(|_, _, _| div().size(px(40.)).child("Open").into_any_element())
                .content(|_, _, _| {
                    div()
                        .debug_selector(|| "disabled-popover-content".into())
                        .size(px(40.))
                })
                .on_open_change(move |open, _, _| changes.borrow_mut().push(*open))
        }
    }

    #[gpui::test]
    fn a_disabled_popover_ignores_its_trigger_and_stays_closed(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let changes = Rc::new(RefCell::new(Vec::new()));
        let (view, cx) = cx.add_window_view({
            let changes = changes.clone();
            move |_, _| DisabledHarness {
                open: None,
                changes,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_click(point(px(20.), px(10.)), Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("disabled-popover-content").is_none());
        assert!(changes.borrow().is_empty());

        // Even the owner cannot open it.
        cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.open = Some(true);
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("disabled-popover-content").is_none());
    }

    struct ArrowHarness {
        positions: Rc<RefCell<Vec<gpui::Bounds<gpui::Pixels>>>>,
    }

    impl Render for ArrowHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let positions = self.positions.clone();
            // A flex row keeps the trigger container at the trigger's size.
            div().flex().items_start().child(
                Popover::new("arrow-popover")
                    .default_open(true)
                    .offset(px(6.))
                    .arrow(PopupArrow::new(px(6.)).corner_radius(px(4.)))
                    .on_position(move |_, trigger| positions.borrow_mut().push(trigger))
                    .trigger_with(|_, _, _| div().size(px(40.)).child("Open").into_any_element())
                    .content(|_, _, _| {
                        div()
                            .debug_selector(|| "arrow-popover-content".into())
                            .size(px(40.))
                    }),
            )
        }
    }

    /// The arrow observes the popup's position without taking the owner's
    /// observer away.
    #[gpui::test]
    fn an_arrow_keeps_the_owners_position_observer(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let positions = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let positions = positions.clone();
            move |_, _| ArrowHarness { positions }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let content = cx.debug_bounds("arrow-popover-content").unwrap();
        let trigger = *positions.borrow().last().expect("the position is observed");
        assert_eq!(content.top(), trigger.bottom() + px(6.));
    }

    fn bounds(x: f32, y: f32, width: f32, height: f32) -> gpui::Bounds<gpui::Pixels> {
        gpui::Bounds::new(point(px(x), px(y)), gpui::size(px(width), px(height)))
    }

    #[test]
    fn the_arrow_aims_at_the_point_the_anchor_names() {
        let content = bounds(0., 0., 400., 400.);
        let trigger = bounds(120., 120., 40., 20.);
        for (anchor, side, tip) in [
            (Anchor::TopLeft, Placement::Bottom, point(px(120.), px(-6.))),
            (
                Anchor::TopCenter,
                Placement::Bottom,
                point(px(140.), px(-6.)),
            ),
            (
                Anchor::TopRight,
                Placement::Bottom,
                point(px(160.), px(-6.)),
            ),
            (
                Anchor::BottomLeft,
                Placement::Top,
                point(px(120.), px(406.)),
            ),
            (
                Anchor::BottomCenter,
                Placement::Top,
                point(px(140.), px(406.)),
            ),
            (
                Anchor::BottomRight,
                Placement::Top,
                point(px(160.), px(406.)),
            ),
            (
                Anchor::LeftCenter,
                Placement::Right,
                point(px(-6.), px(130.)),
            ),
            (
                Anchor::RightCenter,
                Placement::Left,
                point(px(406.), px(130.)),
            ),
        ] {
            let arrow = ArrowGeometry::anchored(anchor, content, trigger, px(6.), px(4.));
            assert_eq!((arrow.side(), arrow.tip()), (side, tip), "{anchor:?}");
        }
    }

    #[test]
    fn the_arrow_points_back_at_the_trigger_on_every_side() {
        let content = bounds(100., 100., 80., 60.);
        for (side, target, tip) in [
            (
                Placement::Bottom,
                point(px(140.), px(70.)),
                point(px(140.), px(94.)),
            ),
            (
                Placement::Top,
                point(px(140.), px(180.)),
                point(px(140.), px(166.)),
            ),
            (
                Placement::Right,
                point(px(80.), px(130.)),
                point(px(94.), px(130.)),
            ),
            (
                Placement::Left,
                point(px(200.), px(130.)),
                point(px(186.), px(130.)),
            ),
        ] {
            let arrow = ArrowGeometry::new(side, content, target, px(6.), px(4.));
            assert_eq!(arrow.tip(), tip, "{side:?}");
        }
        // Aimed past a corner, the base keeps clear of the corner radius.
        let clamped = ArrowGeometry::new(
            Placement::Bottom,
            content,
            point(px(10.), px(70.)),
            px(6.),
            px(4.),
        );
        assert_eq!(
            clamped.points(),
            [
                point(px(104.), px(100.)),
                point(px(110.), px(94.)),
                point(px(116.), px(100.)),
            ]
        );
        assert_eq!(clamped.bounds(), bounds(104., 94., 12., 6.));
    }

    #[test]
    fn the_arrow_join_covers_both_sides_of_the_content_edge() {
        let content = bounds(100., 100., 80., 60.);
        let target = point(px(140.), px(130.));
        for (side, expected) in [
            (Placement::Bottom, bounds(135., 99., 10., 2.)),
            (Placement::Top, bounds(135., 159., 10., 2.)),
            (Placement::Right, bounds(99., 125., 2., 10.)),
            (Placement::Left, bounds(179., 125., 2., 10.)),
        ] {
            let arrow = ArrowGeometry::new(side, content, target, px(6.), px(4.));
            assert_eq!(arrow.join_bounds(px(1.)), expected, "{side:?}");
        }
    }
}
