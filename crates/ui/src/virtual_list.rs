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
use std::{cmp, ops::Range, rc::Rc};

use gpui::{
    div, point, px, size, Along, AnyElement, App, AvailableSpace, Axis, Bounds, ContentMask,
    Context, Div, Element, ElementId, Entity, GlobalElementId, Hitbox, InteractiveElement,
    IntoElement, IsZero as _, Pixels, Render, ScrollHandle, Size, Stateful,
    StatefulInteractiveElement, StyleRefinement, Styled, Window,
};
use smallvec::SmallVec;

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
    f: impl 'static + Fn(&mut V, Range<usize>, Size<Pixels>, &mut Window, &mut Context<V>) -> Vec<R>,
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
    f: impl 'static + Fn(&mut V, Range<usize>, Size<Pixels>, &mut Window, &mut Context<V>) -> Vec<R>,
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
    f: impl 'static + Fn(&mut V, Range<usize>, Size<Pixels>, &mut Window, &mut Context<V>) -> Vec<R>,
) -> VirtualList
where
    R: IntoElement,
    V: Render,
{
    let id: ElementId = id.into();
    let scroll_handle = ScrollHandle::default();
    let render_range = move |visible_range, content_size, window: &mut Window, cx: &mut App| {
        view.update(cx, |this, cx| {
            f(this, visible_range, content_size, window, cx)
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
    scroll_handle: ScrollHandle,
    // scroll_handle: ScrollHandle,
    items_count: usize,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    render_items: Box<
        dyn for<'a> Fn(
            Range<usize>,
            Size<Pixels>,
            &'a mut Window,
            &'a mut App,
        ) -> SmallVec<[AnyElement; 64]>,
    >,
}

impl Styled for VirtualList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl VirtualList {
    pub fn track_scroll(mut self, scroll_handle: &ScrollHandle) -> Self {
        self.base = self.base.track_scroll(&scroll_handle);
        self.scroll_handle = scroll_handle.clone();
        self
    }

    /// Specify for table.
    pub(crate) fn with_scroll_handle(mut self, scroll_handle: &ScrollHandle) -> Self {
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

        let mut items = (self.render_items)(item_ix..item_ix + 1, Size::default(), window, cx);
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
    container_size: Size<Pixels>,
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
        let style = self
            .base
            .interactivity()
            .compute_style(global_id, None, window, cx);
        let font_size = window.text_style().font_size.to_pixels(window.rem_size());

        // Including the gap between items for calculate the item size
        let gap = match self.axis {
            Axis::Horizontal => style.gap.width,
            Axis::Vertical => style.gap.height,
        }
        .to_pixels(font_size.into(), window.rem_size());

        let (layout_id, size_layout) = window.with_element_state(
            global_id.unwrap(),
            |state: Option<ItemSizeLayout>, window| {
                let mut state = state.unwrap_or(ItemSizeLayout::default());

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
                    state.container_size = Size {
                        width: px(self.item_sizes.iter().map(|size| size.width.0).sum::<f32>()),
                        height: px(self
                            .item_sizes
                            .iter()
                            .map(|size| size.height.0)
                            .sum::<f32>()),
                    };
                }

                let (layout_id, _) = self
                    .base
                    .request_layout(global_id, inspector_id, window, cx);

                ((layout_id, state.clone()), state)
            },
        );

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
        let style = self
            .base
            .interactivity()
            .compute_style(global_id, None, window, cx);
        let border = style.border_widths.to_pixels(window.rem_size());
        let padding = style
            .padding
            .to_pixels(bounds.size.into(), window.rem_size());

        let first_item_size = self.measure_item(window, cx);

        let padded_bounds = Bounds::from_corners(
            bounds.origin + point(border.left + padding.left, border.top + padding.top),
            bounds.bottom_right()
                - point(border.right + padding.right, border.bottom + padding.bottom),
        );

        // Get border + padding pixel size
        let padding_size = match self.axis {
            Axis::Horizontal => border.left + padding.left + border.right + padding.right,
            Axis::Vertical => border.top + padding.top + border.bottom + padding.bottom,
        };

        let item_sizes = &layout.size_layout.sizes;
        let item_origins = &layout.size_layout.origins;

        let content_size = match self.axis {
            Axis::Horizontal => Size {
                width: layout.size_layout.container_size.width + padding_size,
                height: (first_item_size.height + padding_size).max(padded_bounds.size.height),
            },
            Axis::Vertical => Size {
                width: (first_item_size.width + padding_size).max(padded_bounds.size.width),
                height: layout.size_layout.container_size.height + padding_size,
            },
        };

        self.base.interactivity().prepaint(
            global_id,
            inspector_id,
            bounds,
            content_size,
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
                    let is_scrolled = match self.axis {
                        Axis::Horizontal => !scroll_offset.x.is_zero(),
                        Axis::Vertical => !scroll_offset.y.is_zero(),
                    };

                    let min_scroll_offset = match self.axis {
                        Axis::Horizontal => padded_bounds.size.width - content_size.width,
                        Axis::Vertical => padded_bounds.size.height - content_size.height,
                    };

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

                    let items =
                        (self.render_items)(visible_range.clone(), content_size, window, cx);

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
