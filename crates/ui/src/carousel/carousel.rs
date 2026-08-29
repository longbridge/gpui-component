use std::{panic::Location, sync::Arc, time::Duration};

use gpui::{
    AnyElement, App, Axis, Bounds, ElementId, Entity, InteractiveElement as _, IntoElement,
    ParentElement, Pixels, Point, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Subscription, Window, div, prelude::FluentBuilder as _, px, relative,
};
use gpui_base::{Spring, spring};
use rust_i18n::t;

use super::{CONTEXT, scroll_mask::CarouselScrollMask, state::CarouselState};
use crate::{
    AxisExt as _, Disableable as _, ElementExt as _, Selectable as _, Sizable as _, Size,
    StyledExt as _, ThemeStyled as _,
    actions::{SelectDown, SelectFirst, SelectLast, SelectLeft, SelectRight, SelectUp},
    button::Button,
    icon::IconName,
};

const SNAP_SPRING: Spring = Spring::new(Duration::from_millis(250)).with_epsilon(0.5);

/// A composable carousel root.
///
/// Add one [`CarouselContent`] and any optional controls as children. Every
/// part must share the same [`CarouselState`].
#[derive(IntoElement)]
pub struct Carousel {
    id: ElementId,
    state: Entity<CarouselState>,
    style: StyleRefinement,
    accessibility_label: SharedString,
    children: Vec<AnyElement>,
}

struct CarouselStateObserver {
    _subscription: Subscription,
}

impl Carousel {
    /// Creates a Carousel bound to `state`.
    pub fn new(id: impl Into<ElementId>, state: &Entity<CarouselState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            style: StyleRefinement::default(),
            accessibility_label: t!("Carousel.label").into(),
            children: Vec::new(),
        }
    }

    /// Sets the name announced for the carousel region.
    pub fn with_accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = label.into();
        self
    }
}

impl ParentElement for Carousel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Carousel {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Carousel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let observed_state = self.state.clone();
        let _observer = window.use_keyed_state(
            ("carousel-state-observer", self.state.entity_id()),
            cx,
            move |_, cx| CarouselStateObserver {
                _subscription: cx.observe(&observed_state, |_, _, cx| cx.notify()),
            },
        );
        let axis = self.state.read(cx).axis();
        let focus_handle = window
            .use_keyed_state(("carousel-focus", self.state.entity_id()), cx, |_, cx| {
                cx.focus_handle()
            })
            .read(cx)
            .clone();
        let previous_state = self.state.clone();
        let next_state = self.state.clone();
        let first_state = self.state.clone();
        let last_state = self.state.clone();

        div()
            .id(self.id)
            .relative()
            .role(Role::Region)
            .aria_label(self.accessibility_label)
            .track_focus(&focus_handle.tab_stop(true))
            .key_context(CONTEXT)
            .on_action(
                window.listener_for(&previous_state, move |state, _: &SelectLeft, _, cx| {
                    if axis.is_horizontal() && state.select_previous(cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_action(
                window.listener_for(&next_state, move |state, _: &SelectRight, _, cx| {
                    if axis.is_horizontal() && state.select_next(cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_action(
                window.listener_for(&previous_state, move |state, _: &SelectUp, _, cx| {
                    if axis.is_vertical() && state.select_previous(cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_action(
                window.listener_for(&next_state, move |state, _: &SelectDown, _, cx| {
                    if axis.is_vertical() && state.select_next(cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_action(
                window.listener_for(&first_state, |state, _: &SelectFirst, _, cx| {
                    if state.select_first(cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_action(
                window.listener_for(&last_state, |state, _: &SelectLast, _, cx| {
                    if state.select_last(cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .children(self.children)
            .refine_style(&self.style)
    }
}

#[derive(Default, PartialEq)]
struct CarouselGeometry {
    viewport: Bounds<Pixels>,
    items: Vec<Bounds<Pixels>>,
    revision: usize,
}

impl CarouselGeometry {
    fn read(state: &CarouselState) -> Self {
        let handle = state.scroll_handle();
        Self {
            viewport: handle.bounds(),
            items: (0..state.item_count())
                .filter_map(|ix| handle.bounds_for_item(ix))
                .collect(),
            revision: 0,
        }
    }

    fn same_layout(&self, other: &Self) -> bool {
        self.viewport == other.viewport && self.items == other.items
    }
}

/// The clipped viewport and snap track for Carousel items.
#[derive(IntoElement)]
pub struct CarouselContent {
    state: Entity<CarouselState>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl CarouselContent {
    /// Creates content bound to `state`.
    pub fn new(state: &Entity<CarouselState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl ParentElement for CarouselContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CarouselContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CarouselContent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let entity_id = self.state.entity_id();
        let snapshot = self.state.read(cx);
        let axis = snapshot.axis();
        let selected_ix = snapshot.selected_index();
        let item_count = snapshot.item_count();
        let handle = snapshot.scroll_handle().clone();
        let interacting = snapshot.is_interacting();
        let motion_revision = snapshot.motion_revision();
        let state_snap_target = selected_ix.and_then(|ix| snapshot.snap_target_for(ix));

        let geometry = window.use_keyed_state(
            ElementId::NamedChild(
                Arc::new(("carousel-geometry", entity_id).into()),
                "content".into(),
            ),
            cx,
            |_, _| CarouselGeometry::default(),
        );
        let geometry_revision = geometry.read(cx).revision;

        let current = axis_value(handle.offset(), axis);
        let target = state_snap_target
            .map(|target| axis_value(target, axis))
            .or_else(|| selected_ix.and_then(|ix| snap_offset(&handle, axis, ix)))
            .unwrap_or(current);
        let target = if interacting { current } else { target };
        let animated = spring(
            (
                ("carousel-content", entity_id),
                SharedString::from(format!("offset-{motion_revision}-{geometry_revision}")),
            ),
            target.as_f32(),
            SNAP_SPRING.with_travel(!interacting),
            window,
            cx,
        );
        let mut offset = handle.offset();
        set_axis_value(&mut offset, axis, px(animated));
        set_axis_value(
            &mut offset,
            if axis.is_horizontal() {
                Axis::Vertical
            } else {
                Axis::Horizontal
            },
            Pixels::ZERO,
        );
        handle.set_offset(offset);

        let geometry_state = self.state.clone();
        let viewport_id: ElementId = ("carousel-content", entity_id).into();

        div()
            .relative()
            .w_full()
            .child(
                div()
                    .id(viewport_id.clone())
                    .w_full()
                    .flex()
                    .gap_4()
                    .when(axis.is_horizontal(), |this| this.flex_row())
                    .when(axis.is_vertical(), |this| this.flex_col())
                    .overflow_hidden()
                    .track_scroll(&handle)
                    .children(self.children)
                    .refine_style(&self.style),
            )
            .child(CarouselScrollMask::new(axis, &self.state).id(viewport_id))
            .on_prepaint(move |_, _, cx| {
                let next = CarouselGeometry::read(geometry_state.read(cx));
                if !geometry.read(cx).same_layout(&next) {
                    geometry_state.update(cx, |state, _| {
                        state.set_geometry(next.viewport, next.items.clone());
                    });
                    geometry.update(cx, |current, cx| {
                        current.viewport = next.viewport;
                        current.items = next.items;
                        current.revision = current.revision.wrapping_add(1);
                        cx.notify();
                    });
                }
            })
            .when(item_count == 0, |this| this.invisible())
    }
}

/// One logical slide in a [`CarouselContent`].
#[derive(IntoElement)]
pub struct CarouselItem {
    id: ElementId,
    index: usize,
    state: Entity<CarouselState>,
    style: StyleRefinement,
    accessibility_label: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl CarouselItem {
    /// Creates the item at the zero-based `index` used by `state`.
    pub fn new(id: impl Into<ElementId>, index: usize, state: &Entity<CarouselState>) -> Self {
        Self {
            id: id.into(),
            index,
            state: state.clone(),
            style: StyleRefinement::default(),
            accessibility_label: None,
            children: Vec::new(),
        }
    }

    /// Replaces the generated "Slide N of M" accessibility label.
    pub fn with_accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl ParentElement for CarouselItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CarouselItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CarouselItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let axis = state.axis();
        let count = state.item_count();
        let label = self.accessibility_label.unwrap_or_else(|| {
            t!(
                "Carousel.slide",
                current = self.index.saturating_add(1),
                total = count
            )
            .into()
        });

        div()
            .id(self.id)
            .role(Role::Group)
            .aria_label(label)
            .aria_position_in_set(self.index.saturating_add(1))
            .aria_size_of_set(count)
            .min_w_0()
            .min_h_0()
            .flex_none()
            .when(axis.is_horizontal(), |this| this.w_full())
            .when(axis.is_vertical(), |this| this.h_full())
            .children(self.children)
            .refine_style(&self.style)
    }
}

/// A previous-slide control positioned around the Carousel viewport.
#[derive(IntoElement)]
pub struct CarouselPrevious {
    state: Entity<CarouselState>,
    size: Size,
    style: StyleRefinement,
}

impl CarouselPrevious {
    /// Creates a previous-slide control bound to `state`.
    pub fn new(state: &Entity<CarouselState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::Small,
            style: StyleRefinement::default(),
        }
    }
}

impl crate::Sizable for CarouselPrevious {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for CarouselPrevious {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CarouselPrevious {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        carousel_control(self.state, self.size, self.style, false, cx)
    }
}

/// A next-slide control positioned around the Carousel viewport.
#[derive(IntoElement)]
pub struct CarouselNext {
    state: Entity<CarouselState>,
    size: Size,
    style: StyleRefinement,
}

impl CarouselNext {
    /// Creates a next-slide control bound to `state`.
    pub fn new(state: &Entity<CarouselState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::Small,
            style: StyleRefinement::default(),
        }
    }
}

impl crate::Sizable for CarouselNext {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for CarouselNext {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CarouselNext {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        carousel_control(self.state, self.size, self.style, true, cx)
    }
}

fn carousel_control(
    state: Entity<CarouselState>,
    size: Size,
    style: StyleRefinement,
    next: bool,
    cx: &mut App,
) -> impl IntoElement {
    let snapshot = state.read(cx);
    let axis = snapshot.axis();
    let disabled = if next {
        !snapshot.has_next()
    } else {
        !snapshot.has_previous()
    };
    let (name, label, icon) = match (axis, next) {
        (Axis::Horizontal, false) => ("previous", t!("Carousel.previous"), IconName::ChevronLeft),
        (Axis::Horizontal, true) => ("next", t!("Carousel.next"), IconName::ChevronRight),
        (Axis::Vertical, false) => ("previous", t!("Carousel.previous"), IconName::ChevronUp),
        (Axis::Vertical, true) => ("next", t!("Carousel.next"), IconName::ChevronDown),
    };
    let id = ElementId::NamedChild(
        Arc::new(("carousel-control", state.entity_id()).into()),
        name.into(),
    );

    Button::new(id)
        .outline()
        .compact()
        .with_size(size)
        .icon(icon)
        .accessibility_label(label.clone())
        .tooltip(label)
        .disabled(disabled)
        .absolute()
        .rounded_full_style(cx)
        .when(axis.is_horizontal() && !next, |this| {
            this.left(px(-40.)).top(relative(0.5)).mt(px(-15.))
        })
        .when(axis.is_horizontal() && next, |this| {
            this.right(px(-40.)).top(relative(0.5)).mt(px(-15.))
        })
        .when(axis.is_vertical() && !next, |this| {
            this.top(px(-40.)).left(relative(0.5)).ml(px(-15.))
        })
        .when(axis.is_vertical() && next, |this| {
            this.bottom(px(-40.)).left(relative(0.5)).ml(px(-15.))
        })
        .when(!disabled, |this| {
            this.on_click(move |_, _, cx| {
                state.update(cx, |state, cx| {
                    if next {
                        state.select_next(cx);
                    } else {
                        state.select_previous(cx);
                    }
                });
            })
        })
        .refine_style(&style)
}

/// A composable container for Carousel pagination items.
#[derive(IntoElement)]
pub struct CarouselPagination {
    id: ElementId,
    style: StyleRefinement,
    accessibility_label: SharedString,
    children: Vec<AnyElement>,
}

impl CarouselPagination {
    /// Creates an empty pagination container.
    #[track_caller]
    pub fn new() -> Self {
        Self {
            id: ElementId::CodeLocation(*Location::caller()),
            style: StyleRefinement::default(),
            accessibility_label: t!("Carousel.pagination").into(),
            children: Vec::new(),
        }
    }

    /// Sets the name announced for the pagination group.
    pub fn with_accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = label.into();
        self
    }
}

impl ParentElement for CarouselPagination {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CarouselPagination {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CarouselPagination {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .role(Role::Group)
            .aria_label(self.accessibility_label)
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .children(self.children)
            .refine_style(&self.style)
    }
}

/// One application-styled pagination control for a Carousel item.
#[derive(IntoElement)]
pub struct CarouselPaginationItem {
    id: ElementId,
    index: usize,
    state: Entity<CarouselState>,
    size: Size,
    style: StyleRefinement,
    accessibility_label: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl CarouselPaginationItem {
    /// Creates a pagination item for the zero-based `index`.
    pub fn new(id: impl Into<ElementId>, index: usize, state: &Entity<CarouselState>) -> Self {
        Self {
            id: id.into(),
            index,
            state: state.clone(),
            size: Size::XSmall,
            style: StyleRefinement::default(),
            accessibility_label: None,
            children: Vec::new(),
        }
    }

    /// Replaces the generated "Go to slide N" accessibility label.
    pub fn with_accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl crate::Sizable for CarouselPaginationItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for CarouselPaginationItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CarouselPaginationItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CarouselPaginationItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let selected = self.state.read(cx).selected_index() == Some(self.index);
        let disabled = self.index >= self.state.read(cx).item_count();
        let label = self.accessibility_label.unwrap_or_else(|| {
            t!(
                "Carousel.go_to_slide",
                current = self.index.saturating_add(1)
            )
            .into()
        });
        let state = self.state;
        let index = self.index;

        Button::new(self.id)
            .compact()
            .with_size(self.size)
            .selected(selected)
            .accessibility_label(label)
            .disabled(disabled)
            .children(self.children)
            .when(!disabled, |this| {
                this.on_click(move |_, _, cx| {
                    state.update(cx, |state, cx| {
                        state.select_index(index, cx);
                    });
                })
            })
            .refine_style(&self.style)
    }
}

fn axis_value(point: Point<Pixels>, axis: Axis) -> Pixels {
    if axis.is_horizontal() {
        point.x
    } else {
        point.y
    }
}

fn set_axis_value(point: &mut Point<Pixels>, axis: Axis, value: Pixels) {
    if axis.is_horizontal() {
        point.x = value;
    } else {
        point.y = value;
    }
}

fn snap_offset(handle: &gpui::ScrollHandle, axis: Axis, index: usize) -> Option<Pixels> {
    let viewport = handle.bounds();
    let item = handle.bounds_for_item(index)?;
    let target = if axis.is_horizontal() {
        viewport.left() - item.left()
    } else {
        viewport.top() - item.top()
    };
    let max = axis_value(handle.max_offset(), axis).max(Pixels::ZERO);
    Some(target.clamp(-max, Pixels::ZERO))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, point};

    #[test]
    fn axis_helpers_only_change_the_requested_coordinate() {
        let mut value = point(px(3.), px(7.));
        set_axis_value(&mut value, Axis::Horizontal, px(11.));
        assert_eq!(value, point(px(11.), px(7.)));
        set_axis_value(&mut value, Axis::Vertical, px(-5.));
        assert_eq!(value, point(px(11.), px(-5.)));
    }

    #[gpui::test]
    fn carousel_controls_accept_semantic_sizes(cx: &mut gpui::TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(2)));

        assert_eq!(CarouselPrevious::new(&state).large().size, Size::Large);
        assert_eq!(CarouselNext::new(&state).xsmall().size, Size::XSmall);
        assert_eq!(
            CarouselPaginationItem::new("pagination", 0, &state)
                .small()
                .size,
            Size::Small
        );
    }
}
