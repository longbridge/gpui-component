use std::{rc::Rc, time::Duration};

use gpui::{
    AnyElement, AnyView, App, Bounds, Context, Display, Div, Element, ElementId, GlobalElementId,
    Half, InspectorElementId, InteractiveElement, IntoElement, LayoutId, ParentElement, Pixels,
    Point, Position, Render, RenderOnce, Role, Size, Stateful, StatefulInteractiveElement, Style,
    Styled, Task, Window, deferred, div, point, px, prelude::FluentBuilder as _,
};

use crate::Placement;

const WINDOW_MARGIN: Pixels = px(4.);
const GRACE_PERIOD: Duration = Duration::from_millis(300);
const SHOW_DELAY: Duration = Duration::from_millis(500);

type TooltipBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyView>;
type TooltipRenderer = Rc<
    dyn Fn(AnyView, TooltipTransition, &mut Window, &mut App) -> AnyElement,
>;

/// An unstyled tooltip popup.
///
/// This corresponds to Base UI's `Tooltip.Popup`: it owns the accessible
/// tooltip role and accepts application-owned content and presentation.
#[derive(IntoElement)]
pub struct Tooltip {
    base: Stateful<Div>,
}

impl Tooltip {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id).role(Role::Tooltip),
        }
    }
}

impl Styled for Tooltip {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Tooltip {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(children);
    }
}

impl RenderOnce for Tooltip {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
    }
}

/// Content requested by a tooltip trigger.
#[derive(Clone)]
pub struct TooltipRequest {
    build: TooltipBuilder,
    trigger_bounds: Bounds<Pixels>,
    preferred_placement: Option<Placement>,
}

impl TooltipRequest {
    pub fn new(
        trigger_bounds: Bounds<Pixels>,
        build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        Self {
            build: Rc::new(build),
            trigger_bounds,
            preferred_placement: None,
        }
    }

    pub fn placement(mut self, placement: Placement) -> Self {
        self.preferred_placement = Some(placement);
        self
    }
}

/// Presentation transition requested by the Base tooltip lifecycle.
#[derive(Clone, Copy, Debug)]
pub enum TooltipTransition {
    Enter { epoch: usize },
    Switch {
        epoch: usize,
        previous: Bounds<Pixels>,
        current: Bounds<Pixels>,
    },
}

/// Per-window tooltip provider and overlay.
pub struct TooltipOverlay {
    content: Option<TooltipRequest>,
    previous_bounds: Option<Bounds<Pixels>>,
    epoch: usize,
    had_recent_tooltip: bool,
    animation_epoch: usize,
    is_switching: bool,
    show_task: Option<Task<()>>,
    hide_task: Option<Task<()>>,
    renderer: TooltipRenderer,
}

impl TooltipOverlay {
    pub fn new() -> Self {
        Self {
            content: None,
            previous_bounds: None,
            epoch: 0,
            had_recent_tooltip: false,
            animation_epoch: 0,
            is_switching: false,
            show_task: None,
            hide_task: None,
            renderer: Rc::new(|view, _, _, _| div().child(view).into_any_element()),
        }
    }

    pub fn render_with(
        mut self,
        renderer: impl Fn(AnyView, TooltipTransition, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.renderer = Rc::new(renderer);
        self
    }

    fn next_epoch(&mut self) -> usize {
        self.epoch += 1;
        self.epoch
    }

    pub fn request_show(
        &mut self,
        content: TooltipRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.hide_task = None;
        let was_visible = self.content.is_some();
        if was_visible || self.had_recent_tooltip {
            self.previous_bounds = self.content.as_ref().map(|content| content.trigger_bounds);
            self.content = Some(content);
            self.show_task = None;
            self.is_switching = was_visible;
            self.animation_epoch += 1;
            cx.notify();
            return;
        }

        let epoch = self.next_epoch();
        self.show_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(SHOW_DELAY).await;
            let _ = this.update_in(cx, |this, _, cx| {
                if this.epoch == epoch {
                    this.content = Some(content);
                    this.previous_bounds = None;
                    this.is_switching = false;
                    this.animation_epoch += 1;
                    cx.notify();
                }
            });
        }));
    }

    pub fn request_hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_task = None;
        if self.content.is_none() {
            return;
        }
        let epoch = self.next_epoch();
        self.had_recent_tooltip = true;
        self.hide_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(GRACE_PERIOD).await;
            let _ = this.update_in(cx, |this, _, cx| {
                if this.epoch == epoch {
                    this.content = None;
                    this.previous_bounds = None;
                    this.had_recent_tooltip = false;
                    cx.notify();
                }
            });
        }));
    }

    pub fn hide(&mut self, cx: &mut Context<Self>) {
        let changed = self.content.is_some()
            || self.previous_bounds.is_some()
            || self.had_recent_tooltip
            || self.show_task.is_some()
            || self.hide_task.is_some();
        self.content = None;
        self.previous_bounds = None;
        self.had_recent_tooltip = false;
        self.is_switching = false;
        self.show_task = None;
        self.hide_task = None;
        if changed {
            cx.notify();
        }
    }
}

impl Default for TooltipOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for TooltipOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(content) = self.content.as_ref() else {
            return div().into_any_element();
        };
        let view = (content.build)(window, cx);
        let transition = match (self.is_switching, self.previous_bounds) {
            (true, Some(previous)) => TooltipTransition::Switch {
                epoch: self.animation_epoch,
                previous,
                current: content.trigger_bounds,
            },
            _ => TooltipTransition::Enter {
                epoch: self.animation_epoch,
            },
        };
        let rendered = (self.renderer)(view, transition, window, cx);
        deferred(
            TooltipPositioner::new(content.trigger_bounds)
                .when_some(content.preferred_placement, |this, placement| this.placement(placement))
                .child(rendered),
        )
        .with_priority(2)
        .into_any_element()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TooltipPosition {
    bounds: Bounds<Pixels>,
    placement: Placement,
}

fn tooltip_position(
    trigger_bounds: Bounds<Pixels>,
    tooltip_size: Size<Pixels>,
    viewport_size: Size<Pixels>,
    margin: Pixels,
    preferred_placement: Option<Placement>,
) -> TooltipPosition {
    let right_limit = (viewport_size.width - margin).max(margin);
    let bottom_limit = (viewport_size.height - margin).max(margin);
    let available_left = (trigger_bounds.left() - margin).max(px(0.));
    let available_right = (right_limit - trigger_bounds.right()).max(px(0.));
    let available_above = (trigger_bounds.top() - margin).max(px(0.));
    let available_below = (bottom_limit - trigger_bounds.bottom()).max(px(0.));

    let placement = match preferred_placement {
        Some(Placement::Right) if tooltip_size.width <= available_right => Placement::Right,
        Some(Placement::Right) if tooltip_size.width <= available_left => Placement::Left,
        Some(Placement::Right) if available_right >= available_left => Placement::Right,
        Some(Placement::Right) => Placement::Left,
        Some(Placement::Left) if tooltip_size.width <= available_left => Placement::Left,
        Some(Placement::Left) if tooltip_size.width <= available_right => Placement::Right,
        Some(Placement::Left) if available_left >= available_right => Placement::Left,
        Some(Placement::Left) => Placement::Right,
        Some(Placement::Bottom) if tooltip_size.height <= available_below => Placement::Bottom,
        Some(Placement::Bottom) if tooltip_size.height <= available_above => Placement::Top,
        Some(Placement::Bottom) if available_below >= available_above => Placement::Bottom,
        Some(Placement::Bottom) => Placement::Top,
        Some(Placement::Top) | None if tooltip_size.height <= available_above => Placement::Top,
        Some(Placement::Top) | None if tooltip_size.height <= available_below => Placement::Bottom,
        Some(Placement::Top) | None if available_below >= available_above => Placement::Bottom,
        Some(Placement::Top) | None => Placement::Top,
    };

    let centered_x = trigger_bounds.center().x - tooltip_size.width.half();
    let centered_y = trigger_bounds.center().y - tooltip_size.height.half();
    let origin = match placement {
        Placement::Top => point(centered_x, trigger_bounds.top() - tooltip_size.height),
        Placement::Bottom => point(centered_x, trigger_bounds.bottom()),
        Placement::Left => point(trigger_bounds.left() - tooltip_size.width, centered_y),
        Placement::Right => point(trigger_bounds.right(), centered_y),
    };

    TooltipPosition {
        bounds: clamp_bounds(Bounds::new(origin, tooltip_size), viewport_size, margin),
        placement,
    }
}

fn clamp_bounds(
    mut bounds: Bounds<Pixels>,
    viewport_size: Size<Pixels>,
    margin: Pixels,
) -> Bounds<Pixels> {
    let right_limit = (viewport_size.width - margin).max(margin);
    let bottom_limit = (viewport_size.height - margin).max(margin);

    if bounds.right() > right_limit {
        bounds.origin.x -= bounds.right() - right_limit;
    }
    if bounds.left() < margin {
        bounds.origin.x = margin;
    }
    if bounds.bottom() > bottom_limit {
        bounds.origin.y -= bounds.bottom() - bottom_limit;
    }
    if bounds.top() < margin {
        bounds.origin.y = margin;
    }

    bounds
}

/// An unstyled tooltip positioner with viewport-aware flipping and clamping.
pub struct TooltipPositioner {
    trigger_bounds: Bounds<Pixels>,
    preferred_placement: Option<Placement>,
    children: Vec<AnyElement>,
}

#[doc(hidden)]
pub struct TooltipPositionerState {
    child_layout_ids: Vec<LayoutId>,
}

impl TooltipPositioner {
    pub fn new(trigger_bounds: Bounds<Pixels>) -> Self {
        Self {
            trigger_bounds,
            preferred_placement: None,
            children: Vec::new(),
        }
    }

    pub fn placement(mut self, placement: Placement) -> Self {
        self.preferred_placement = Some(placement);
        self
    }
}

impl ParentElement for TooltipPositioner {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Element for TooltipPositioner {
    type RequestLayoutState = TooltipPositionerState;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let child_layout_ids = self
            .children
            .iter_mut()
            .map(|child| child.request_layout(window, cx))
            .collect::<Vec<_>>();
        let layout_id = window.request_layout(
            Style {
                position: Position::Absolute,
                display: Display::Flex,
                ..Style::default()
            },
            child_layout_ids.iter().copied(),
            cx,
        );

        (
            layout_id,
            TooltipPositionerState { child_layout_ids },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if request_layout.child_layout_ids.is_empty() {
            return;
        }

        let mut child_min = point(Pixels::MAX, Pixels::MAX);
        let mut child_max = Point::default();
        for child_layout_id in &request_layout.child_layout_ids {
            let child_bounds = window.layout_bounds(*child_layout_id);
            child_min = child_min.min(&child_bounds.origin);
            child_max = child_max.max(&child_bounds.bottom_right());
        }

        let tooltip_size = (child_max - child_min).into();
        let client_inset = window.client_inset().unwrap_or(px(0.));
        let position = tooltip_position(
            self.trigger_bounds,
            tooltip_size,
            window.viewport_size(),
            WINDOW_MARGIN + client_inset,
            self.preferred_placement,
        );
        let offset = position.bounds.origin - bounds.origin;
        let offset = point(offset.x.round(), offset.y.round());

        window.with_element_offset(offset, |window| {
            for child in &mut self.children {
                child.prepaint(window, cx);
            }
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        for child in &mut self.children {
            child.paint(window, cx);
        }
    }
}

impl IntoElement for TooltipPositioner {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, size};

    fn bounds(x: f32, y: f32, width: f32, height: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(width), px(height)))
    }

    fn size_px(width: f32, height: f32) -> Size<Pixels> {
        size(px(width), px(height))
    }

    #[test]
    fn prefers_above_when_space_allows() {
        let trigger = bounds(100., 80., 80., 24.);
        let position = tooltip_position(
            trigger,
            size_px(120., 30.),
            size_px(300., 200.),
            WINDOW_MARGIN,
            None,
        );
        assert_eq!(position.placement, Placement::Top);
        assert_eq!(position.bounds.origin, point(px(80.), px(50.)));
    }

    #[test]
    fn flips_and_clamps_on_each_axis() {
        let top = tooltip_position(
            bounds(24., 4., 120., 32.),
            size_px(240., 32.),
            size_px(520., 260.),
            WINDOW_MARGIN,
            None,
        );
        assert_eq!(top.placement, Placement::Bottom);

        let right = tooltip_position(
            bounds(260., 60., 32., 32.),
            size_px(120., 30.),
            size_px(300., 200.),
            WINDOW_MARGIN,
            Some(Placement::Right),
        );
        assert_eq!(right.placement, Placement::Left);

        let left_edge = tooltip_position(
            bounds(4., 80., 24., 24.),
            size_px(120., 30.),
            size_px(300., 200.),
            WINDOW_MARGIN,
            None,
        );
        assert_eq!(left_edge.bounds.left(), WINDOW_MARGIN);
    }

    #[test]
    fn uses_larger_side_when_neither_vertical_side_fits() {
        let position = tooltip_position(
            bounds(120., 20., 40., 20.),
            size_px(160., 120.),
            size_px(300., 100.),
            WINDOW_MARGIN,
            None,
        );
        assert_eq!(position.placement, Placement::Bottom);
        assert_eq!(position.bounds.top(), WINDOW_MARGIN);
        assert_eq!(position.bounds.left(), px(60.));
    }

    #[test]
    fn places_tooltip_to_the_right() {
        let trigger = bounds(20., 60., 32., 32.);
        let position = tooltip_position(
            trigger,
            size_px(120., 30.),
            size_px(300., 200.),
            WINDOW_MARGIN,
            Some(Placement::Right),
        );
        assert_eq!(position.placement, Placement::Right);
        assert_eq!(position.bounds.left(), trigger.right());
        assert_eq!(position.bounds.center().y, trigger.center().y);
    }

    #[test]
    fn right_placement_clamps_vertical_edges() {
        let trigger = bounds(20., 2., 32., 20.);
        let position = tooltip_position(
            trigger,
            size_px(120., 40.),
            size_px(300., 200.),
            WINDOW_MARGIN,
            Some(Placement::Right),
        );
        assert_eq!(position.placement, Placement::Right);
        assert_eq!(position.bounds.top(), WINDOW_MARGIN);
        assert_eq!(position.bounds.left(), trigger.right());
    }

    #[gpui::test]
    fn provider_owns_grace_switch_and_dismiss(cx: &mut gpui::TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| TooltipOverlay::new()));
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            state.update(cx, |tooltip, cx| {
                tooltip.had_recent_tooltip = true;
                tooltip.request_show(
                    TooltipRequest::new(bounds(0., 0., 20., 20.), |_, _| {
                        panic!("content is not rendered by this lifecycle test")
                    }),
                    window,
                    cx,
                );
            });
        });
        cx.update(|_, cx| assert!(state.read(cx).content.is_some()));

        cx.update(|_, cx| {
            state.update(cx, |tooltip, cx| tooltip.hide(cx));
        });
        cx.update(|_, cx| assert!(state.read(cx).content.is_none()));
    }
}
