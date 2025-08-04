//! Vistual List for render a large number of differently sized rows/columns.
//!
//! > NOTE: This must ensure each column width or row height.
//!
//! Only visible range are rendered for performance reasons.
//!
//! Inspired by `gpui::uniform_list`.
//! https://github.com/zed-industries/zed/blob/0ae1603610ab6b265bdfbee7b8dbc23c5ab06edc/crates/gpui/src/elements/uniform_list.rs
//!
//! Unlike the `uniform_list`, the each item can have different size.
//!
//! This is useful for more complex layout, for example, a table with different row height.
use std::{
    cell::RefCell,
    cmp,
    ops::{Deref, Range},
    rc::Rc,
};

use gpui::{
    div, point, prelude::FluentBuilder as _, px, size, Along, AnyElement, App, AvailableSpace,
    Axis, Bounds, ContentMask, Context, Div, Element, ElementId, Entity, GlobalElementId, Half,
    Hitbox, InteractiveElement, IntoElement, IsZero as _, Pixels, Point, Render, ScrollHandle,
    ScrollStrategy, Size, Stateful, StatefulInteractiveElement, Style, StyleRefinement, Styled,
    Window,
};
use smallvec::SmallVec;

use crate::{scroll::ScrollHandleOffsetable, AxisExt, StyledExt};

struct VirtualListScrollHandleState {
    axis: Axis,
    items_bounds: Vec<Bounds<Pixels>>,
}

#[derive(Clone)]
pub struct VirtualListScrollHandle {
    state: Rc<RefCell<VirtualListScrollHandleState>>,
    base_handle: ScrollHandle,
}

impl From<ScrollHandle> for VirtualListScrollHandle {
    fn from(handle: ScrollHandle) -> Self {
        let mut this = VirtualListScrollHandle::new();
        this.base_handle = handle;
        this
    }
}

impl AsRef<ScrollHandle> for VirtualListScrollHandle {
    fn as_ref(&self) -> &ScrollHandle {
        &self.base_handle
    }
}

impl ScrollHandleOffsetable for VirtualListScrollHandle {
    fn offset(&self) -> Point<Pixels> {
        self.base_handle.offset()
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.base_handle.set_offset(offset);
    }

    fn content_size(&self) -> Size<Pixels> {
        self.base_handle.content_size()
    }
}

impl Deref for VirtualListScrollHandle {
    type Target = ScrollHandle;

    fn deref(&self) -> &Self::Target {
        &self.base_handle
    }
}

impl VirtualListScrollHandle {
    pub fn new() -> Self {
        VirtualListScrollHandle {
            state: Rc::new(RefCell::new(VirtualListScrollHandleState {
                axis: Axis::Vertical,
                items_bounds: vec![],
            })),
            base_handle: ScrollHandle::default(),
        }
    }

    pub fn base_handle(&self) -> &ScrollHandle {
        &self.base_handle
    }

    fn set_items_bounds(&self, items_bounds: Vec<Bounds<Pixels>>) {
        self.state.borrow_mut().items_bounds = items_bounds;
    }

    fn set_axis(&self, axis: Axis) {
        self.state.borrow_mut().axis = axis;
    }

    /// Scroll to the item at the given index.
    pub fn scroll_to_item(&self, ix: usize, strategy: ScrollStrategy) {
        let state = self.state.borrow();
        let Some(bounds) = state.items_bounds.get(ix) else {
            return;
        };

        let axis = state.axis;
        let mut scroll_offset = self.base_handle.offset();
        let container_bounds = self.base_handle().bounds();

        match strategy {
            ScrollStrategy::Center => {
                if axis.is_vertical() {
                    scroll_offset.y = container_bounds.top() + container_bounds.size.height.half()
                        - bounds.top()
                        - bounds.size.height.half()
                } else {
                    scroll_offset.x = container_bounds.left() + container_bounds.size.width.half()
                        - bounds.left()
                        - bounds.size.width.half()
                }
            }
            _ => {
                // Ref: https://github.com/zed-industries/zed/blob/0d145289e0867a8d5d63e5e1397a5ca69c9d49c3/crates/gpui/src/elements/div.rs#L3026
                if axis.is_vertical() {
                    if bounds.top() + scroll_offset.y < container_bounds.top() {
                        scroll_offset.y = container_bounds.top() - bounds.top();
                    } else if bounds.bottom() + scroll_offset.y > container_bounds.bottom() {
                        scroll_offset.y = container_bounds.bottom() - bounds.bottom();
                    }
                } else {
                    if bounds.left() + scroll_offset.x < container_bounds.left() {
                        scroll_offset.x = container_bounds.left() - bounds.left();
                    } else if bounds.right() + scroll_offset.x > container_bounds.right() {
                        scroll_offset.x = container_bounds.right() - bounds.right();
                    }
                }
            }
        }

        self.base_handle.set_offset(scroll_offset);
    }

    /// Scrolls to the bottom of the list.
    pub fn scroll_to_bottom(&self) {
        let state = self.state.borrow();
        self.scroll_to_item(
            state.items_bounds.len().saturating_sub(1),
            ScrollStrategy::Top,
        );
    }
}

/// Create a [`VirtualList`] in vertical direction.
///
/// This is like `uniform_list` in GPUI, but support two axis.
///
/// The `item_sizes` is the size of each column.
///
/// See also [`h_virtual_list`]
#[inline]
pub fn v_virtual_list<R, V>(
    view: Entity<V>,
    id: impl Into<ElementId>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    f: impl 'static + Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R>,
) -> VirtualList
where
    R: IntoElement,
    V: Render,
{
    virtual_list(view, id, Axis::Vertical, item_sizes, f)
}

/// Create a [`VirtualList`] in horizontal direction.
///
/// See also [`v_virtual_list`]
#[inline]
pub fn h_virtual_list<R, V>(
    view: Entity<V>,
    id: impl Into<ElementId>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    f: impl 'static + Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R>,
) -> VirtualList
where
    R: IntoElement,
    V: Render,
{
    virtual_list(view, id, Axis::Horizontal, item_sizes, f)
}

pub(crate) fn virtual_list<R, V>(
    view: Entity<V>,
    id: impl Into<ElementId>,
    axis: Axis,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    f: impl 'static + Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R>,
) -> VirtualList
where
    R: IntoElement,
    V: Render,
{
    let id: ElementId = id.into();
    let scroll_handle = VirtualListScrollHandle::new();
    let render_range = move |visible_range, window: &mut Window, cx: &mut App| {
        view.update(cx, |this, cx| {
            f(this, visible_range, window, cx)
                .into_iter()
                .map(|component| component.into_any_element())
                .collect()
        })
    };

    VirtualList {
        id: id.clone(),
        axis,
        base: div()
            .id(id)
            .size_full()
            .overflow_scroll()
            .track_scroll(&scroll_handle),
        scroll_handle,
        items_count: item_sizes.len(),
        item_sizes,
        render_items: Box::new(render_range),
    }
}

/// VirtualList component for rendering a large number of differently sized items.
pub struct VirtualList {
    id: ElementId,
    axis: Axis,
    base: Stateful<Div>,
    scroll_handle: VirtualListScrollHandle,
    items_count: usize,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    render_items: Box<
        dyn for<'a> Fn(Range<usize>, &'a mut Window, &'a mut App) -> SmallVec<[AnyElement; 64]>,
    >,
}

impl Styled for VirtualList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl VirtualList {
    pub fn track_scroll(mut self, scroll_handle: &VirtualListScrollHandle) -> Self {
        self.base = self.base.track_scroll(&scroll_handle);
        self.scroll_handle = scroll_handle.clone();
        self
    }

    /// Specify for table.
    ///
    /// Table is special, because the `scroll_handle` is based on Table head (That is not a virtual list).
    pub(crate) fn with_scroll_handle(mut self, scroll_handle: &VirtualListScrollHandle) -> Self {
        self.base = div().id(self.id.clone()).size_full();
        self.scroll_handle = scroll_handle.clone();
        self
    }

    /// Measure first item to get the size.
    fn measure_item(&self, window: &mut Window, cx: &mut App) -> Size<Pixels> {
        if self.items_count == 0 {
            return Size::default();
        }

        // Avoid use first item to measure, because in most cases, this first item many complex.
        // So we try to use the second item to measure, if there is no second item, use the first item.
        let item_ix = if self.items_count > 1 { 1 } else { 0 };

        let mut items = (self.render_items)(item_ix..item_ix + 1, window, cx);
        let Some(mut item_to_measure) = items.pop() else {
            return Size::default();
        };
        let available_space = size(AvailableSpace::MinContent, AvailableSpace::MinContent);
        item_to_measure.layout_as_root(available_space, window, cx)
    }
}

/// Frame state used by the [VirtualItem].
pub struct VirtualListFrameState {
    /// Visible items to be painted.
    items: SmallVec<[AnyElement; 32]>,
    size_layout: ItemSizeLayout,
}

#[derive(Default, Clone)]
pub struct ItemSizeLayout {
    items_sizes: Rc<Vec<Size<Pixels>>>,
    content_size: Pixels,
    sizes: Vec<Pixels>,
    origins: Vec<Pixels>,
}

impl IntoElement for VirtualList {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for VirtualList {
    type RequestLayoutState = VirtualListFrameState;
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let rem_size = window.rem_size();
        let font_size = window.text_style().font_size.to_pixels(rem_size);
        let style = self
            .base
            .interactivity()
            .compute_style(global_id, None, window, cx);

        let size_layout =
            window.with_element_state(global_id.unwrap(), |state: Option<ItemSizeLayout>, _| {
                let mut state = state.unwrap_or(ItemSizeLayout::default());

                // Including the gap between items for calculate the item size
                let gap = style
                    .gap
                    .along(self.axis)
                    .to_pixels(font_size.into(), rem_size);

                if state.items_sizes != self.item_sizes {
                    state.items_sizes = self.item_sizes.clone();
                    // Prepare each item's size by axis
                    state.sizes = self
                        .item_sizes
                        .iter()
                        .enumerate()
                        .map(|(i, size)| {
                            let size = size.along(self.axis);
                            if i + 1 == self.items_count {
                                size
                            } else {
                                size + gap
                            }
                        })
                        .collect::<Vec<_>>();

                    // Prepare each item's origin by axis
                    state.origins = state
                        .sizes
                        .iter()
                        .scan(px(0.), |cumulative, size| match self.axis {
                            Axis::Horizontal => {
                                let x = *cumulative;
                                *cumulative += *size;
                                Some(x)
                            }
                            Axis::Vertical => {
                                let y = *cumulative;
                                *cumulative += *size;
                                Some(y)
                            }
                        })
                        .collect::<Vec<_>>();
                    state.content_size = px(state.sizes.iter().map(|size| size.0).sum::<f32>());
                }

                (state.clone(), state)
            });

        let (layout_id, _) = self
            .base
            .request_layout(global_id, inspector_id, window, cx);

        let mut items_element = div()
            .when(self.axis.is_horizontal(), |this| {
                this.h_full().w(size_layout.content_size)
            })
            .when(self.axis.is_vertical(), |this| {
                this.w_full().h(size_layout.content_size)
            });
        let (items_layout_id, _) =
            items_element.request_layout(global_id, inspector_id, window, cx);
        let layout_id = window.request_layout(style, vec![layout_id, items_layout_id], cx);

        (
            layout_id,
            VirtualListFrameState {
                items: SmallVec::new(),
                size_layout,
            },
        )
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        dbg!(&bounds);
        let first_item_size = self.measure_item(window, cx);

        let style = self
            .base
            .interactivity()
            .compute_style(global_id, None, window, cx);
        let border = style.border_widths.to_pixels(window.rem_size());
        let padding = style
            .padding
            .to_pixels(bounds.size.into(), window.rem_size());

        let container_with_padding_bounds = Bounds::from_corners(
            bounds.origin + point(border.left + padding.left, border.top + padding.top),
            bounds.bottom_right()
                - point(border.right + padding.right, border.bottom + padding.bottom),
        );

        // Get border + padding pixel size
        let padding_width = match self.axis {
            Axis::Horizontal => border.left + padding.left + border.right + padding.right,
            Axis::Vertical => border.top + padding.top + border.bottom + padding.bottom,
        };

        let item_sizes = &layout.size_layout.sizes;
        let item_origins = &layout.size_layout.origins;

        let scroll_size = match self.axis {
            Axis::Horizontal => Size {
                width: layout.size_layout.content_size + padding_width,
                height: (first_item_size.height + padding_width)
                    .max(container_with_padding_bounds.size.height),
            },
            Axis::Vertical => Size {
                width: (first_item_size.width + padding_width)
                    .max(container_with_padding_bounds.size.width),
                height: layout.size_layout.content_size + padding_width,
            },
        };

        // Update scroll_handle with the item bounds
        let items_bounds = item_origins
            .iter()
            .enumerate()
            .map(|(i, &origin)| {
                let item_size = item_sizes[i];

                Bounds {
                    origin: match self.axis {
                        Axis::Horizontal => point(bounds.left() + origin + padding.left, px(0.)),
                        Axis::Vertical => point(px(0.), bounds.top() + origin + padding.top),
                    },
                    size: match self.axis {
                        Axis::Horizontal => size(item_size, bounds.size.height),
                        Axis::Vertical => size(bounds.size.width, item_size),
                    },
                }
            })
            .collect::<Vec<_>>();
        self.scroll_handle.set_axis(self.axis);
        self.scroll_handle.set_items_bounds(items_bounds);

        self.base.interactivity().prepaint(
            global_id,
            inspector_id,
            bounds,
            scroll_size,
            window,
            cx,
            |style, _, hitbox, window, cx| {
                let mut scroll_offset = self.scroll_handle.offset();
                let border = style.border_widths.to_pixels(window.rem_size());
                let padding = style
                    .padding
                    .to_pixels(bounds.size.into(), window.rem_size());

                let padded_bounds = Bounds::from_corners(
                    bounds.origin + point(border.left + padding.left, border.top),
                    bounds.bottom_right() - point(border.right + padding.right, border.bottom),
                );

                if self.items_count > 0 {
                    let is_scrolled = !scroll_offset.along(self.axis).is_zero();
                    let min_scroll_offset =
                        padded_bounds.size.along(self.axis) - scroll_size.along(self.axis);

                    if is_scrolled {
                        match self.axis {
                            Axis::Horizontal if scroll_offset.x < min_scroll_offset => {
                                scroll_offset.x = min_scroll_offset;
                            }
                            Axis::Vertical if scroll_offset.y < min_scroll_offset => {
                                scroll_offset.y = min_scroll_offset;
                            }
                            _ => {}
                        }
                    }

                    let (first_visible_element_ix, last_visible_element_ix) = match self.axis {
                        Axis::Horizontal => {
                            let mut cumulative_size = px(0.);
                            let mut first_visible_element_ix = 0;
                            for (i, &size) in item_sizes.iter().enumerate() {
                                cumulative_size += size;
                                if cumulative_size > -(scroll_offset.x + padding.left) {
                                    first_visible_element_ix = i;
                                    break;
                                }
                            }

                            cumulative_size = px(0.);
                            let mut last_visible_element_ix = 0;
                            for (i, &size) in item_sizes.iter().enumerate() {
                                cumulative_size += size;
                                if cumulative_size > (-scroll_offset.x + padded_bounds.size.width) {
                                    last_visible_element_ix = i + 1;
                                    break;
                                }
                            }
                            if last_visible_element_ix == 0 {
                                last_visible_element_ix = self.items_count;
                            } else {
                                last_visible_element_ix += 1;
                            }
                            (first_visible_element_ix, last_visible_element_ix)
                        }
                        Axis::Vertical => {
                            let mut cumulative_size = px(0.);
                            let mut first_visible_element_ix = 0;
                            for (i, &size) in item_sizes.iter().enumerate() {
                                cumulative_size += size;
                                if cumulative_size > -(scroll_offset.y + padding.top) {
                                    first_visible_element_ix = i;
                                    break;
                                }
                            }

                            cumulative_size = px(0.);
                            let mut last_visible_element_ix = 0;
                            for (i, &size) in item_sizes.iter().enumerate() {
                                cumulative_size += size;
                                if cumulative_size > (-scroll_offset.y + padded_bounds.size.height)
                                {
                                    last_visible_element_ix = i + 1;
                                    break;
                                }
                            }
                            if last_visible_element_ix == 0 {
                                last_visible_element_ix = self.items_count;
                            } else {
                                last_visible_element_ix += 1;
                            }
                            (first_visible_element_ix, last_visible_element_ix)
                        }
                    };

                    let visible_range = first_visible_element_ix
                        ..cmp::min(last_visible_element_ix, self.items_count);

                    let items = (self.render_items)(visible_range.clone(), window, cx);

                    let content_mask = ContentMask { bounds };
                    window.with_content_mask(Some(content_mask), |window| {
                        for (mut item, ix) in items.into_iter().zip(visible_range.clone()) {
                            let item_origin = match self.axis {
                                Axis::Horizontal => {
                                    padded_bounds.origin
                                        + point(
                                            item_origins[ix] + scroll_offset.x,
                                            padding.top + scroll_offset.y,
                                        )
                                }
                                Axis::Vertical => {
                                    padded_bounds.origin
                                        + point(
                                            scroll_offset.x,
                                            padding.top + item_origins[ix] + scroll_offset.y,
                                        )
                                }
                            };

                            let available_space = match self.axis {
                                Axis::Horizontal => size(
                                    AvailableSpace::Definite(item_sizes[ix]),
                                    AvailableSpace::Definite(padded_bounds.size.height),
                                ),
                                Axis::Vertical => size(
                                    AvailableSpace::Definite(padded_bounds.size.width),
                                    AvailableSpace::Definite(item_sizes[ix]),
                                ),
                            };

                            item.layout_as_root(available_space, window, cx);
                            item.prepaint_at(item_origin, window, cx);
                            layout.items.push(item);
                        }
                    });
                }

                hitbox
            },
        )
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.base.interactivity().paint(
            global_id,
            inspector_id,
            bounds,
            hitbox.as_ref(),
            window,
            cx,
            |_, window, cx| {
                for item in &mut layout.items {
                    item.paint(window, cx);
                }
            },
        )
    }
}
