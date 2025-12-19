use std::{ops::Range, rc::Rc, time::Duration};

use crate::{
    ActiveTheme, ElementExt, Icon, IconName, StyleSized as _, StyledExt, VirtualListScrollHandle,
    actions::{Cancel, SelectDown, SelectUp},
    h_flex,
    menu::{ContextMenuExt, PopupMenu},
    scroll::{ScrollableMask, Scrollbar},
    v_flex,
};
use gpui::{
    AppContext, Axis, Bounds, ClickEvent, Context, Div, DragMoveEvent, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, ListSizingBehavior, MouseButton, MouseDownEvent,
    ParentElement, Pixels, Point, Render, ScrollStrategy, SharedString, Stateful,
    StatefulInteractiveElement as _, Styled, Task, UniformListScrollHandle, Window, div,
    prelude::FluentBuilder, px, uniform_list,
};

use super::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SelectionState {
    Column,
    Row,
}

/// The Table event.
#[derive(Clone)]
pub enum TableEvent {
    /// Single click or move to selected row.
    SelectRow(usize),
    /// Double click on the row.
    DoubleClickedRow(usize),
    /// Selected column.
    SelectColumn(usize),
    /// The column widths have changed.
    ///
    /// The `Vec<Pixels>` contains the new widths of all columns.
    ColumnWidthsChanged(Vec<Pixels>),
    /// A column has been moved.
    ///
    /// The first `usize` is the original index of the column,
    /// and the second `usize` is the new index of the column.
    MoveColumn(usize, usize),
}

/// The visible range of the rows and columns.
#[derive(Debug, Default)]
pub struct TableVisibleRange {
    /// The visible range of the rows.
    rows: Range<usize>,
    /// The visible range of the columns.
    cols: Range<usize>,
}

impl TableVisibleRange {
    /// Returns the visible range of the rows.
    pub fn rows(&self) -> &Range<usize> {
        &self.rows
    }

    /// Returns the visible range of the columns.
    pub fn cols(&self) -> &Range<usize> {
        &self.cols
    }
}

/// The state for [`Table`].
pub struct TableState<D: TableDelegate> {
    focus_handle: FocusHandle,
    delegate: D,
    pub(super) options: TableOptions,
    /// The bounds of the table container.
    bounds: Bounds<Pixels>,
    /// The bounds of the fixed head cols.
    fixed_head_cols_bounds: Bounds<Pixels>,

    col_groups: Vec<ColGroup>,

    /// Whether the table can loop selection, default is true.
    ///
    /// When the prev/next selection is out of the table bounds, the selection will loop to the other side.
    pub loop_selection: bool,
    /// Whether the table can select column.
    pub col_selectable: bool,
    /// Whether the table can select row.
    pub row_selectable: bool,
    /// Whether the table can sort.
    pub sortable: bool,
    /// Whether the table can resize columns.
    pub col_resizable: bool,
    /// Whether the table can move columns.
    pub col_movable: bool,
    /// Enable/disable fixed columns feature.
    pub col_fixed: bool,

    pub vertical_scroll_handle: UniformListScrollHandle,
    pub horizontal_scroll_handle: VirtualListScrollHandle,

    selected_row: Option<usize>,
    selection_state: SelectionState,
    right_clicked_row: Option<usize>,
    selected_col: Option<usize>,

    /// The column index that is being resized.
    resizing_col: Option<usize>,

    /// The visible range of the rows and columns.
    visible_range: TableVisibleRange,

    _measure: Vec<Duration>,
    _load_more_task: Task<()>,
}

impl<D> TableState<D>
where
    D: TableDelegate,
{
    /// Create a new TableState with the given delegate.
    pub fn new(delegate: D, _: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            focus_handle: cx.focus_handle(),
            options: TableOptions::default(),
            delegate,
            col_groups: Vec::new(),
            horizontal_scroll_handle: VirtualListScrollHandle::new(),
            vertical_scroll_handle: UniformListScrollHandle::new(),
            selection_state: SelectionState::Row,
            selected_row: None,
            right_clicked_row: None,
            selected_col: None,
            resizing_col: None,
            bounds: Bounds::default(),
            fixed_head_cols_bounds: Bounds::default(),
            visible_range: TableVisibleRange::default(),
            loop_selection: true,
            col_selectable: true,
            row_selectable: true,
            sortable: true,
            col_movable: true,
            col_resizable: true,
            col_fixed: true,
            _load_more_task: Task::ready(()),
            _measure: Vec::new(),
        };

        this.prepare_col_groups(cx);
        this
    }

    /// Returns a reference to the delegate.
    pub fn delegate(&self) -> &D {
        &self.delegate
    }

    /// Returns a mutable reference to the delegate.
    pub fn delegate_mut(&mut self) -> &mut D {
        &mut self.delegate
    }

    /// Set to loop selection, default to true.
    pub fn loop_selection(mut self, loop_selection: bool) -> Self {
        self.loop_selection = loop_selection;
        self
    }

    /// Set to enable/disable column movable, default to true.
    pub fn col_movable(mut self, col_movable: bool) -> Self {
        self.col_movable = col_movable;
        self
    }

    /// Set to enable/disable column resizable, default to true.
    pub fn col_resizable(mut self, col_resizable: bool) -> Self {
        self.col_resizable = col_resizable;
        self
    }

    /// Set to enable/disable column sortable, default true
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    /// Set to enable/disable row selectable, default true
    pub fn row_selectable(mut self, row_selectable: bool) -> Self {
        self.row_selectable = row_selectable;
        self
    }

    /// Set to enable/disable column selectable, default true
    pub fn col_selectable(mut self, col_selectable: bool) -> Self {
        self.col_selectable = col_selectable;
        self
    }

    /// When we update columns or rows, we need to refresh the table.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.prepare_col_groups(cx);
    }

    /// Scroll to the row at the given index.
    pub fn scroll_to_row(&mut self, row_ix: usize, cx: &mut Context<Self>) {
        self.vertical_scroll_handle
            .scroll_to_item(row_ix, ScrollStrategy::Top);
        cx.notify();
    }

    // Scroll to the column at the given index.
    pub fn scroll_to_col(&mut self, col_ix: usize, cx: &mut Context<Self>) {
        let col_ix = col_ix.saturating_sub(self.fixed_left_cols_count());

        self.horizontal_scroll_handle
            .scroll_to_item(col_ix, ScrollStrategy::Top);
        cx.notify();
    }

    /// Returns the selected row index.
    pub fn selected_row(&self) -> Option<usize> {
        self.selected_row
    }

    /// Sets the selected row to the given index.
    pub fn set_selected_row(&mut self, row_ix: usize, cx: &mut Context<Self>) {
        let is_down = match self.selected_row {
            Some(selected_row) => row_ix > selected_row,
            None => true,
        };

        self.selection_state = SelectionState::Row;
        self.right_clicked_row = None;
        self.selected_row = Some(row_ix);
        if let Some(row_ix) = self.selected_row {
            self.vertical_scroll_handle.scroll_to_item(
                row_ix,
                if is_down {
                    ScrollStrategy::Bottom
                } else {
                    ScrollStrategy::Top
                },
            );
        }
        cx.emit(TableEvent::SelectRow(row_ix));
        cx.notify();
    }

    /// Returns the selected column index.
    pub fn selected_col(&self) -> Option<usize> {
        self.selected_col
    }

    /// Sets the selected col to the given index.
    pub fn set_selected_col(&mut self, col_ix: usize, cx: &mut Context<Self>) {
        self.selection_state = SelectionState::Column;
        self.selected_col = Some(col_ix);
        if let Some(col_ix) = self.selected_col {
            self.scroll_to_col(col_ix, cx);
        }
        cx.emit(TableEvent::SelectColumn(col_ix));
        cx.notify();
    }

    /// Clear the selection of the table.
    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.selection_state = SelectionState::Row;
        self.selected_row = None;
        self.selected_col = None;
        cx.notify();
    }

    /// Returns the visible range of the rows and columns.
    ///
    /// See [`TableVisibleRange`].
    pub fn visible_range(&self) -> &TableVisibleRange {
        &self.visible_range
    }

    fn prepare_col_groups(&mut self, cx: &mut Context<Self>) {
        self.col_groups = (0..self.delegate.columns_count(cx))
            .map(|col_ix| {
                let column = self.delegate().column(col_ix, cx);
                ColGroup {
                    width: column.width,
                    bounds: Bounds::default(),
                    column,
                }
            })
            .collect();
        cx.notify();
    }

    fn fixed_left_cols_count(&self) -> usize {
        if !self.col_fixed {
            return 0;
        }

        self.col_groups
            .iter()
            .filter(|col| col.column.fixed == Some(ColumnFixed::Left))
            .count()
    }

    fn on_row_right_click(
        &mut self,
        _: &MouseDownEvent,
        row_ix: usize,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.right_clicked_row = Some(row_ix);
    }

    fn on_row_left_click(
        &mut self,
        e: &ClickEvent,
        row_ix: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_selected_row(row_ix, cx);

        if e.click_count() == 2 {
            cx.emit(TableEvent::DoubleClickedRow(row_ix));
        }
    }

    fn on_col_head_click(&mut self, col_ix: usize, _: &mut Window, cx: &mut Context<Self>) {
        if !self.col_selectable {
            return;
        }

        let Some(col_group) = self.col_groups.get(col_ix) else {
            return;
        };

        if !col_group.column.selectable {
            return;
        }

        self.set_selected_col(col_ix, cx)
    }

    fn has_selection(&self) -> bool {
        self.selected_row.is_some() || self.selected_col.is_some()
    }

    pub(super) fn action_cancel(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_selection() {
            self.clear_selection(cx);
            return;
        }
        cx.propagate();
    }

    pub(super) fn action_select_prev(
        &mut self,
        _: &SelectUp,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rows_count = self.delegate.rows_count(cx);
        if rows_count < 1 {
            return;
        }

        let mut selected_row = self.selected_row.unwrap_or(0);
        if selected_row > 0 {
            selected_row = selected_row.saturating_sub(1);
        } else {
            if self.loop_selection {
                selected_row = rows_count.saturating_sub(1);
            }
        }

        self.set_selected_row(selected_row, cx);
    }

    pub(super) fn action_select_next(
        &mut self,
        _: &SelectDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rows_count = self.delegate.rows_count(cx);
        if rows_count < 1 {
            return;
        }

        let selected_row = match self.selected_row {
            Some(selected_row) if selected_row < rows_count.saturating_sub(1) => selected_row + 1,
            Some(selected_row) => {
                if self.loop_selection {
                    0
                } else {
                    selected_row
                }
            }
            _ => 0,
        };

        self.set_selected_row(selected_row, cx);
    }

    pub(super) fn action_select_prev_col(
        &mut self,
        _: &SelectPrevColumn,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut selected_col = self.selected_col.unwrap_or(0);
        let columns_count = self.delegate.columns_count(cx);
        if selected_col > 0 {
            selected_col = selected_col.saturating_sub(1);
        } else {
            if self.loop_selection {
                selected_col = columns_count.saturating_sub(1);
            }
        }
        self.set_selected_col(selected_col, cx);
    }

    pub(super) fn action_select_next_col(
        &mut self,
        _: &SelectNextColumn,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut selected_col = self.selected_col.unwrap_or(0);
        if selected_col < self.delegate.columns_count(cx).saturating_sub(1) {
            selected_col += 1;
        } else {
            if self.loop_selection {
                selected_col = 0;
            }
        }

        self.set_selected_col(selected_col, cx);
    }

    /// Scroll table when mouse position is near the edge of the table bounds.
    fn scroll_table_by_col_resizing(
        &mut self,
        mouse_position: Point<Pixels>,
        col_group: &ColGroup,
    ) {
        // Do nothing if pos out of the table bounds right for avoid scroll to the right.
        if mouse_position.x > self.bounds.right() {
            return;
        }

        let mut offset = self.horizontal_scroll_handle.offset();
        let col_bounds = col_group.bounds;

        if mouse_position.x < self.bounds.left()
            && col_bounds.right() < self.bounds.left() + px(20.)
        {
            offset.x += px(1.);
        } else if mouse_position.x > self.bounds.right()
            && col_bounds.right() > self.bounds.right() - px(20.)
        {
            offset.x -= px(1.);
        }

        self.horizontal_scroll_handle.set_offset(offset);
    }

    /// The `ix`` is the index of the col to resize,
    /// and the `size` is the new size for the col.
    fn resize_cols(&mut self, ix: usize, size: Pixels, _: &mut Window, cx: &mut Context<Self>) {
        if !self.col_resizable {
            return;
        }

        let Some(col_group) = self.col_groups.get_mut(ix) else {
            return;
        };

        if !col_group.is_resizable() {
            return;
        }

        let new_width = size.clamp(col_group.column.min_width, col_group.column.max_width);

        // Only update if it actually changed
        if col_group.width != new_width {
            col_group.width = new_width;
            cx.notify();
        }
    }

    fn perform_sort(&mut self, col_ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if !self.sortable {
            return;
        }

        let sort = self.col_groups.get(col_ix).and_then(|g| g.column.sort);
        if sort.is_none() {
            return;
        }

        let sort = sort.unwrap();
        let sort = match sort {
            ColumnSort::Ascending => ColumnSort::Default,
            ColumnSort::Descending => ColumnSort::Ascending,
            ColumnSort::Default => ColumnSort::Descending,
        };

        for (ix, col_group) in self.col_groups.iter_mut().enumerate() {
            if ix == col_ix {
                col_group.column.sort = Some(sort);
            } else {
                if col_group.column.sort.is_some() {
                    col_group.column.sort = Some(ColumnSort::Default);
                }
            }
        }

        self.delegate_mut().perform_sort(col_ix, sort, window, cx);

        cx.notify();
    }

    fn move_column(
        &mut self,
        col_ix: usize,
        to_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if col_ix == to_ix {
            return;
        }

        self.delegate.move_column(col_ix, to_ix, window, cx);
        let col_group = self.col_groups.remove(col_ix);
        self.col_groups.insert(to_ix, col_group);

        cx.emit(TableEvent::MoveColumn(col_ix, to_ix));
        cx.notify();
    }

    /// Dispatch delegate's `load_more` method when the visible range is near the end.
    fn load_more_if_need(
        &mut self,
        rows_count: usize,
        visible_end: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let threshold = self.delegate.load_more_threshold();
        // Securely handle subtract logic to prevent attempt to subtract with overflow
        if visible_end >= rows_count.saturating_sub(threshold) {
            if !self.delegate.has_more(cx) {
                return;
            }

            self._load_more_task = cx.spawn_in(window, async move |view, window| {
                _ = view.update_in(window, |view, window, cx| {
                    view.delegate.load_more(window, cx);
                });
            });
        }
    }

    fn update_visible_range_if_need(
        &mut self,
        visible_range: Range<usize>,
        axis: Axis,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Skip when visible range is only 1 item.
        // The visual_list will use first item to measure.
        if visible_range.len() <= 1 {
            return;
        }

        if axis == Axis::Vertical {
            if self.visible_range.rows == visible_range {
                return;
            }
            self.delegate_mut()
                .visible_rows_changed(visible_range.clone(), window, cx);
            self.visible_range.rows = visible_range;
        } else {
            if self.visible_range.cols == visible_range {
                return;
            }
            self.delegate_mut()
                .visible_columns_changed(visible_range.clone(), window, cx);
            self.visible_range.cols = visible_range;
        }
    }

    fn render_cell(&self, col_ix: usize, _window: &mut Window, _cx: &mut Context<Self>) -> Div {
        let Some(col_group) = self.col_groups.get(col_ix) else {
            return div();
        };

        let col_width = col_group.width;
        let col_padding = col_group.column.paddings;
        div()
            .w(col_width)
            .h_full()
            .flex_shrink_0()
            .overflow_hidden()
            .whitespace_nowrap()
            .table_cell_size(self.options.size)
            .map(|this| match col_padding {
                Some(padding) => this
                    .pl(padding.left)
                    .pr(padding.right)
                    .pt(padding.top)
                    .pb(padding.bottom),
                None => this,
            })
    }

    /// Show Column selection style, when the column is selected and the selection state is Column.
    fn render_col_wrap(&self, col_ix: usize, _: &mut Window, cx: &mut Context<Self>) -> Div {
        let el = h_flex().h_full();
        let selectable = self.col_selectable
            && self
                .col_groups
                .get(col_ix)
                .map(|col_group| col_group.column.selectable)
                .unwrap_or(false);

        if selectable
            && self.selected_col == Some(col_ix)
            && self.selection_state == SelectionState::Column
        {
            el.bg(cx.theme().table_active)
        } else {
            el
        }
    }

    fn render_resize_handle(
        &self,
        ix: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        const HANDLE_SIZE: Pixels = px(2.);

        let resizable = self.col_resizable
            && self
                .col_groups
                .get(ix)
                .map(|col| col.is_resizable())
                .unwrap_or(false);
        if !resizable {
            return div().into_any_element();
        }

        let group_id = SharedString::from(format!("resizable-handle:{}", ix));

        h_flex()
            .id(("resizable-handle", ix))
            .group(group_id.clone())
            .occlude()
            .cursor_col_resize()
            .h_full()
            .w(HANDLE_SIZE)
            .ml(-(HANDLE_SIZE))
            .justify_end()
            .items_center()
            .child(
                div()
                    .h_full()
                    .justify_center()
                    .bg(cx.theme().table_row_border)
                    .group_hover(&group_id, |this| this.bg(cx.theme().border).h_full())
                    .w(px(1.)),
            )
            .on_drag_move(
                cx.listener(move |view, e: &DragMoveEvent<ResizeColumn>, window, cx| {
                    match e.drag(cx) {
                        ResizeColumn((entity_id, ix)) => {
                            if cx.entity_id() != *entity_id {
                                return;
                            }

                            // sync col widths into real widths
                            // TODO: Consider to remove this, this may not need now.
                            // for (_, col_group) in view.col_groups.iter_mut().enumerate() {
                            //     col_group.width = col_group.bounds.size.width;
                            // }

                            let ix = *ix;
                            view.resizing_col = Some(ix);

                            let col_group = view
                                .col_groups
                                .get(ix)
                                .expect("BUG: invalid col index")
                                .clone();

                            view.resize_cols(
                                ix,
                                e.event.position.x - HANDLE_SIZE - col_group.bounds.left(),
                                window,
                                cx,
                            );

                            // scroll the table if the drag is near the edge
                            view.scroll_table_by_col_resizing(e.event.position, &col_group);
                        }
                    };
                }),
            )
            .on_drag(ResizeColumn((cx.entity_id(), ix)), |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|view, _, _, cx| {
                    if view.resizing_col.is_none() {
                        return;
                    }

                    view.resizing_col = None;

                    let new_widths = view.col_groups.iter().map(|g| g.width).collect();
                    cx.emit(TableEvent::ColumnWidthsChanged(new_widths));
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    fn render_sort_icon(
        &self,
        col_ix: usize,
        col_group: &ColGroup,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        if !self.sortable {
            return None;
        }

        let Some(sort) = col_group.column.sort else {
            return None;
        };

        let (icon, is_on) = match sort {
            ColumnSort::Ascending => (IconName::SortAscending, true),
            ColumnSort::Descending => (IconName::SortDescending, true),
            ColumnSort::Default => (IconName::ChevronsUpDown, false),
        };

        Some(
            div()
                .id(("icon-sort", col_ix))
                .p(px(2.))
                .rounded(cx.theme().radius / 2.)
                .map(|this| match is_on {
                    true => this,
                    false => this.opacity(0.5),
                })
                .hover(|this| this.bg(cx.theme().secondary).opacity(7.))
                .active(|this| this.bg(cx.theme().secondary_active).opacity(1.))
                .on_click(
                    cx.listener(move |table, _, window, cx| table.perform_sort(col_ix, window, cx)),
                )
                .child(
                    Icon::new(icon)
                        .size_3()
                        .text_color(cx.theme().secondary_foreground),
                ),
        )
    }

    /// Render the column header.
    /// The children must be one by one items.
    /// Because the horizontal scroll handle will use the child_item_bounds to
    /// calculate the item position for itself's `scroll_to_item` method.
    fn render_th(&mut self, col_ix: usize, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let entity_id = cx.entity_id();
        let col_group = self.col_groups.get(col_ix).expect("BUG: invalid col index");

        let movable = self.col_movable && col_group.column.movable;
        let paddings = col_group.column.paddings;
        let name = col_group.column.name.clone();

        h_flex()
            .h_full()
            .child(
                self.render_cell(col_ix, window, cx)
                    .id(("col-header", col_ix))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.on_col_head_click(col_ix, window, cx);
                    }))
                    .child(
                        h_flex()
                            .size_full()
                            .justify_between()
                            .items_center()
                            .child(self.delegate.render_th(col_ix, window, cx))
                            .when_some(paddings, |this, paddings| {
                                // Leave right space for the sort icon, if this column have custom padding
                                let offset_pr =
                                    self.options.size.table_cell_padding().right - paddings.right;
                                this.pr(offset_pr.max(px(0.)))
                            })
                            .children(self.render_sort_icon(col_ix, &col_group, window, cx)),
                    )
                    .when(movable, |this| {
                        this.on_drag(
                            DragColumn {
                                entity_id,
                                col_ix,
                                name,
                                width: col_group.width,
                            },
                            |drag, _, _, cx| {
                                cx.stop_propagation();
                                cx.new(|_| drag.clone())
                            },
                        )
                        .drag_over::<DragColumn>(|this, _, _, cx| {
                            this.rounded_l_none()
                                .border_l_2()
                                .border_r_0()
                                .border_color(cx.theme().drag_border)
                        })
                        .on_drop(cx.listener(
                            move |table, drag: &DragColumn, window, cx| {
                                // If the drag col is not the same as the drop col, then swap the cols.
                                if drag.entity_id != cx.entity_id() {
                                    return;
                                }

                                table.move_column(drag.col_ix, col_ix, window, cx);
                            },
                        ))
                    }),
            )
            // resize handle
            .child(self.render_resize_handle(col_ix, window, cx))
            // to save the bounds of this col.
            .on_prepaint({
                let view = cx.entity().clone();
                move |bounds, _, cx| view.update(cx, |r, _| r.col_groups[col_ix].bounds = bounds)
            })
    }

    fn render_table_header(
        &mut self,
        left_columns_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();
        let horizontal_scroll_handle = self.horizontal_scroll_handle.clone();

        // Reset fixed head columns bounds, if no fixed columns are present
        if left_columns_count == 0 {
            self.fixed_head_cols_bounds = Bounds::default();
        }

        let mut header = self.delegate_mut().render_header(window, cx);
        let style = header.style().clone();

        header
            .h_flex()
            .w_full()
            .h(self.options.size.table_row_height())
            .flex_shrink_0()
            .border_b_1()
            .border_color(cx.theme().border)
            .text_color(cx.theme().table_head_foreground)
            .refine_style(&style)
            .when(left_columns_count > 0, |this| {
                let view = view.clone();
                // Render left fixed columns
                this.child(
                    h_flex()
                        .relative()
                        .h_full()
                        .bg(cx.theme().table_head)
                        .children(
                            self.col_groups
                                .clone()
                                .into_iter()
                                .filter(|col| col.column.fixed == Some(ColumnFixed::Left))
                                .enumerate()
                                .map(|(col_ix, _)| self.render_th(col_ix, window, cx)),
                        )
                        .child(
                            // Fixed columns border
                            div()
                                .absolute()
                                .top_0()
                                .right_0()
                                .bottom_0()
                                .w_0()
                                .flex_shrink_0()
                                .border_r_1()
                                .border_color(cx.theme().border),
                        )
                        .on_prepaint(move |bounds, _, cx| {
                            view.update(cx, |r, _| r.fixed_head_cols_bounds = bounds)
                        }),
                )
            })
            .child(
                // Columns
                h_flex()
                    .id("table-head")
                    .size_full()
                    .overflow_scroll()
                    .relative()
                    .track_scroll(&horizontal_scroll_handle)
                    .bg(cx.theme().table_head)
                    .child(
                        h_flex()
                            .relative()
                            .children(
                                self.col_groups
                                    .clone()
                                    .into_iter()
                                    .skip(left_columns_count)
                                    .enumerate()
                                    .map(|(col_ix, _)| {
                                        self.render_th(left_columns_count + col_ix, window, cx)
                                    }),
                            )
                            .child(self.delegate.render_last_empty_col(window, cx)),
                    ),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_table_row(
        &mut self,
        row_ix: usize,
        rows_count: usize,
        left_columns_count: usize,
        col_sizes: Rc<Vec<gpui::Size<Pixels>>>,
        columns_count: usize,
        is_filled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let horizontal_scroll_handle = self.horizontal_scroll_handle.clone();
        let is_stripe_row = self.options.stripe && row_ix % 2 != 0;
        let is_selected = self.selected_row == Some(row_ix);
        let view = cx.entity().clone();
        let row_height = self.options.size.table_row_height();

        if row_ix < rows_count {
            let is_last_row = row_ix + 1 == rows_count;
            let need_render_border = is_selected || !is_last_row || !is_filled;

            let mut tr = self.delegate.render_tr(row_ix, window, cx);
            let style = tr.style().clone();

            tr.h_flex()
                .w_full()
                .h(row_height)
                .when(need_render_border, |this| {
                    this.border_b_1().border_color(cx.theme().table_row_border)
                })
                .when(is_stripe_row, |this| this.bg(cx.theme().table_even))
                .refine_style(&style)
                .hover(|this| {
                    if is_selected || self.right_clicked_row == Some(row_ix) {
                        this
                    } else {
                        this.bg(cx.theme().table_hover)
                    }
                })
                .when(left_columns_count > 0, |this| {
                    // Left fixed columns
                    this.child(
                        h_flex()
                            .relative()
                            .h_full()
                            .children({
                                let mut items = Vec::with_capacity(left_columns_count);

                                (0..left_columns_count).for_each(|col_ix| {
                                    items.push(self.render_col_wrap(col_ix, window, cx).child(
                                        self.render_cell(col_ix, window, cx).child(
                                            self.measure_render_td(row_ix, col_ix, window, cx),
                                        ),
                                    ));
                                });

                                items
                            })
                            .child(
                                // Fixed columns border
                                div()
                                    .absolute()
                                    .top_0()
                                    .right_0()
                                    .bottom_0()
                                    .w_0()
                                    .flex_shrink_0()
                                    .border_r_1()
                                    .border_color(cx.theme().border),
                            ),
                    )
                })
                .child(
                    h_flex()
                        .flex_1()
                        .h_full()
                        .overflow_hidden()
                        .relative()
                        .child(
                            crate::virtual_list::virtual_list(
                                view,
                                row_ix,
                                Axis::Horizontal,
                                col_sizes,
                                {
                                    move |table, visible_range: Range<usize>, window, cx| {
                                        table.update_visible_range_if_need(
                                            visible_range.clone(),
                                            Axis::Horizontal,
                                            window,
                                            cx,
                                        );

                                        let mut items = Vec::with_capacity(
                                            visible_range.end - visible_range.start,
                                        );

                                        visible_range.for_each(|col_ix| {
                                            let col_ix = col_ix + left_columns_count;
                                            let el =
                                                table.render_col_wrap(col_ix, window, cx).child(
                                                    table.render_cell(col_ix, window, cx).child(
                                                        table.measure_render_td(
                                                            row_ix, col_ix, window, cx,
                                                        ),
                                                    ),
                                                );

                                            items.push(el);
                                        });

                                        items
                                    }
                                },
                            )
                            .with_scroll_handle(&self.horizontal_scroll_handle),
                        )
                        .child(self.delegate.render_last_empty_col(window, cx)),
                )
                // Row selected style
                .when_some(self.selected_row, |this, _| {
                    this.when(
                        is_selected && self.selection_state == SelectionState::Row,
                        |this| {
                            this.border_color(gpui::transparent_white()).child(
                                div()
                                    .top(if row_ix == 0 { px(0.) } else { px(-1.) })
                                    .left(px(0.))
                                    .right(px(0.))
                                    .bottom(px(-1.))
                                    .absolute()
                                    .bg(cx.theme().table_active)
                                    .border_1()
                                    .border_color(cx.theme().table_active_border),
                            )
                        },
                    )
                })
                // Row right click row style
                .when(self.right_clicked_row == Some(row_ix), |this| {
                    this.border_color(gpui::transparent_white()).child(
                        div()
                            .top(if row_ix == 0 { px(0.) } else { px(-1.) })
                            .left(px(0.))
                            .right(px(0.))
                            .bottom(px(-1.))
                            .absolute()
                            .border_1()
                            .border_color(cx.theme().selection),
                    )
                })
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, e, window, cx| {
                        this.on_row_right_click(e, row_ix, window, cx);
                    }),
                )
                .on_click(cx.listener(move |this, e, window, cx| {
                    this.on_row_left_click(e, row_ix, window, cx);
                }))
        } else {
            // Render fake rows to fill the rest table space
            self.delegate
                .render_tr(row_ix, window, cx)
                .h_flex()
                .w_full()
                .h(row_height)
                .border_b_1()
                .border_color(cx.theme().table_row_border)
                .when(is_stripe_row, |this| this.bg(cx.theme().table_even))
                .children((0..columns_count).map(|col_ix| {
                    h_flex()
                        .left(horizontal_scroll_handle.offset().x)
                        .child(self.render_cell(col_ix, window, cx))
                }))
                .child(self.delegate.render_last_empty_col(window, cx))
        }
    }

    /// Calculate the extra rows needed to fill the table empty space when `stripe` is true.
    fn calculate_extra_rows_needed(
        &self,
        total_height: Pixels,
        actual_height: Pixels,
        row_height: Pixels,
    ) -> usize {
        let mut extra_rows_needed = 0;

        let remaining_height = total_height - actual_height;
        if remaining_height > px(0.) {
            extra_rows_needed = (remaining_height / row_height).floor() as usize;
        }

        extra_rows_needed
    }

    #[inline]
    fn measure_render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if !crate::measure_enable() {
            return self
                .delegate
                .render_td(row_ix, col_ix, window, cx)
                .into_any_element();
        }

        let start = std::time::Instant::now();
        let el = self.delegate.render_td(row_ix, col_ix, window, cx);
        self._measure.push(start.elapsed());
        el.into_any_element()
    }

    fn measure(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        if !crate::measure_enable() {
            return;
        }

        // Print avg measure time of each td
        if self._measure.len() > 0 {
            let total = self
                ._measure
                .iter()
                .fold(Duration::default(), |acc, d| acc + *d);
            let avg = total / self._measure.len() as u32;
            eprintln!(
                "last render {} cells total: {:?}, avg: {:?}",
                self._measure.len(),
                total,
                avg,
            );
        }
        self._measure.clear();
    }

    fn render_vertical_scrollbar(
        &mut self,

        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        Some(
            div()
                .occlude()
                .absolute()
                .top(self.options.size.table_row_height())
                .right_0()
                .bottom_0()
                .w(Scrollbar::width())
                .child(Scrollbar::vertical(&self.vertical_scroll_handle).max_fps(60)),
        )
    }

    fn render_horizontal_scrollbar(
        &mut self,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .occlude()
            .absolute()
            .left(self.fixed_head_cols_bounds.size.width)
            .right_0()
            .bottom_0()
            .h(Scrollbar::width())
            .child(Scrollbar::horizontal(&self.horizontal_scroll_handle))
    }
}

impl<D> Focusable for TableState<D>
where
    D: TableDelegate,
{
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl<D> EventEmitter<TableEvent> for TableState<D> where D: TableDelegate {}

impl<D> Render for TableState<D>
where
    D: TableDelegate,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.measure(window, cx);

        let columns_count = self.delegate.columns_count(cx);
        let left_columns_count = self
            .col_groups
            .iter()
            .filter(|col| self.col_fixed && col.column.fixed == Some(ColumnFixed::Left))
            .count();
        let rows_count = self.delegate.rows_count(cx);
        let loading = self.delegate.loading(cx);

        let row_height = self.options.size.table_row_height();
        let total_height = self
            .vertical_scroll_handle
            .0
            .borrow()
            .base_handle
            .bounds()
            .size
            .height;
        let actual_height = row_height * rows_count as f32;
        let extra_rows_count =
            self.calculate_extra_rows_needed(total_height, actual_height, row_height);
        let render_rows_count = if self.options.stripe {
            rows_count + extra_rows_count
        } else {
            rows_count
        };
        let right_clicked_row = self.right_clicked_row;
        let is_filled = total_height > Pixels::ZERO && total_height <= actual_height;

        let loading_view = if loading {
            Some(
                self.delegate
                    .render_loading(self.options.size, window, cx)
                    .into_any_element(),
            )
        } else {
            None
        };

        let empty_view = if rows_count == 0 {
            Some(
                div()
                    .size_full()
                    .child(self.delegate.render_empty(window, cx))
                    .into_any_element(),
            )
        } else {
            None
        };

        let inner_table = v_flex()
            .id("table-inner")
            .size_full()
            .overflow_hidden()
            .child(self.render_table_header(left_columns_count, window, cx))
            .context_menu({
                let view = cx.entity().clone();
                move |this, window: &mut Window, cx: &mut Context<PopupMenu>| {
                    if let Some(row_ix) = view.read(cx).right_clicked_row {
                        view.update(cx, |menu, cx| {
                            menu.delegate_mut().context_menu(row_ix, this, window, cx)
                        })
                    } else {
                        this
                    }
                }
            })
            .map(|this| {
                if rows_count == 0 {
                    this.children(empty_view)
                } else {
                    this.child(
                        h_flex().id("table-body").flex_grow().size_full().child(
                            uniform_list(
                                "table-uniform-list",
                                render_rows_count,
                                cx.processor(
                                    move |table, visible_range: Range<usize>, window, cx| {
                                        // We must calculate the col sizes here, because the col sizes
                                        // need render_th first, then that method will set the bounds of each col.
                                        let col_sizes: Rc<Vec<gpui::Size<Pixels>>> = Rc::new(
                                            table
                                                .col_groups
                                                .iter()
                                                .skip(left_columns_count)
                                                .map(|col| col.bounds.size)
                                                .collect(),
                                        );

                                        table.load_more_if_need(
                                            rows_count,
                                            visible_range.end,
                                            window,
                                            cx,
                                        );
                                        table.update_visible_range_if_need(
                                            visible_range.clone(),
                                            Axis::Vertical,
                                            window,
                                            cx,
                                        );

                                        if visible_range.end > rows_count {
                                            table.scroll_to_row(
                                                std::cmp::min(
                                                    visible_range.start,
                                                    rows_count.saturating_sub(1),
                                                ),
                                                cx,
                                            );
                                        }

                                        let mut items = Vec::with_capacity(
                                            visible_range.end.saturating_sub(visible_range.start),
                                        );

                                        // Render fake rows to fill the table
                                        visible_range.for_each(|row_ix| {
                                            // Render real rows for available data
                                            items.push(table.render_table_row(
                                                row_ix,
                                                rows_count,
                                                left_columns_count,
                                                col_sizes.clone(),
                                                columns_count,
                                                is_filled,
                                                window,
                                                cx,
                                            ));
                                        });

                                        items
                                    },
                                ),
                            )
                            .flex_grow()
                            .size_full()
                            .with_sizing_behavior(ListSizingBehavior::Auto)
                            .track_scroll(&self.vertical_scroll_handle)
                            .into_any_element(),
                        ),
                    )
                }
            });

        div()
            .size_full()
            .children(loading_view)
            .when(!loading, |this| {
                this.child(inner_table)
                    .child(ScrollableMask::new(
                        Axis::Horizontal,
                        &self.horizontal_scroll_handle,
                    ))
                    .when(right_clicked_row.is_some(), |this| {
                        this.on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            this.right_clicked_row = None;
                            cx.notify();
                        }))
                    })
            })
            .on_prepaint({
                let state = cx.entity();
                move |bounds, _, cx| state.update(cx, |state, _| state.bounds = bounds)
            })
            .when(!window.is_inspector_picking(cx), |this| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .size_full()
                        .when(self.options.scrollbar_visible.bottom, |this| {
                            this.child(self.render_horizontal_scrollbar(window, cx))
                        })
                        .when(
                            self.options.scrollbar_visible.right && rows_count > 0,
                            |this| this.children(self.render_vertical_scrollbar(window, cx)),
                        ),
                )
            })
    }
}
