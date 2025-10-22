use std::{ops::Range, rc::Rc};

use crate::{
    actions::{Cancel, SelectDown, SelectUp},
    context_menu::ContextMenuExt,
    h_flex,
    popup_menu::PopupMenu,
    scroll::{ScrollableMask, Scrollbar, ScrollbarState},
    table::loading::Loading,
    v_flex, ActiveTheme, Icon, IconName, Sizable, Size, StyleSized as _, StyledExt,
    VirtualListScrollHandle,
};
use gpui::{
    actions, canvas, div, prelude::FluentBuilder, px, uniform_list, AnyElement, App, AppContext,
    Axis, Bounds, Context, Div, DragMoveEvent, Edges, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyBinding, ListSizingBehavior, MouseButton, MouseDownEvent,
    ParentElement, Pixels, Point, Render, RenderOnce, ScrollStrategy, ScrollWheelEvent,
    SharedString, Stateful, StatefulInteractiveElement as _, Styled, Task, UniformListScrollHandle,
    Window,
};

mod column;
mod loading;

pub use column::*;

actions!(table, [SelectPrevColumn, SelectNextColumn]);

pub(crate) fn init(cx: &mut App) {
    let context = Some("Table");
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, context),
        KeyBinding::new("up", SelectUp, context),
        KeyBinding::new("down", SelectDown, context),
        KeyBinding::new("left", SelectPrevColumn, context),
        KeyBinding::new("right", SelectNextColumn, context),
    ]);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SelectionState {
    Column,
    Row,
}

#[derive(Clone)]
pub enum TableEvent {
    /// Single click or move to selected row.
    SelectRow(usize),
    /// Double click on the row.
    DoubleClickedRow(usize),
    SelectColumn(usize),
    ColumnWidthsChanged(Vec<Pixels>),
    MoveColumn(usize, usize),
}

/// The visible range of the rows and columns.
#[derive(Debug, Default)]
pub struct VisibleRangeState {
    /// The visible range of the rows.
    rows: Range<usize>,
    /// The visible range of the columns.
    cols: Range<usize>,
}

impl VisibleRangeState {
    /// Returns the visible range of the rows.
    pub fn rows(&self) -> Range<usize> {
        self.rows.clone()
    }

    /// Returns the visible range of the columns.
    pub fn cols(&self) -> Range<usize> {
        self.cols.clone()
    }
}

pub struct TableState {
    focus_handle: FocusHandle,
    col_groups: Vec<ColGroup>,
    rows_count: usize,

    /// The bounds of the table container.
    bounds: Bounds<Pixels>,
    /// The bounds of the fixed head cols.
    fixed_head_cols_bounds: Bounds<Pixels>,

    pub vertical_scroll_handle: UniformListScrollHandle,
    pub vertical_scroll_state: ScrollbarState,
    pub horizontal_scroll_handle: VirtualListScrollHandle,
    pub horizontal_scroll_state: ScrollbarState,

    selected_row: Option<usize>,
    selection_state: SelectionState,
    right_clicked_row: Option<usize>,
    selected_col: Option<usize>,

    /// The column index that is being resized.
    resizing_col: Option<usize>,

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

    load_more_threshold: usize,

    /// The visible range of the rows and columns.
    visible_range: VisibleRangeState,

    _load_more_task: Task<()>,
}

impl TableState {
    pub fn new(
        columns: Vec<Column>,
        rows_count: usize,
        _: &mut Window,
        cx: &mut Context<TableState>,
    ) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            col_groups: Self::prepare_col_groups(columns),
            rows_count,
            horizontal_scroll_handle: VirtualListScrollHandle::new(),
            vertical_scroll_handle: UniformListScrollHandle::new(),
            vertical_scroll_state: ScrollbarState::default(),
            horizontal_scroll_state: ScrollbarState::default(),
            selection_state: SelectionState::Row,
            selected_row: None,
            right_clicked_row: None,
            selected_col: None,
            resizing_col: None,
            bounds: Bounds::default(),
            fixed_head_cols_bounds: Bounds::default(),
            visible_range: VisibleRangeState::default(),
            loop_selection: true,
            col_selectable: true,
            row_selectable: true,
            sortable: true,
            col_movable: true,
            col_resizable: true,
            col_fixed: true,
            load_more_threshold: 20,
            _load_more_task: Task::ready(()),
        }
    }

    pub fn set_columns(&mut self, columns: Vec<Column>, cx: &mut Context<Self>) {
        self.col_groups = Self::prepare_col_groups(columns);
        cx.notify();
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

    /// Set a threshold value (n rows), of course, when scrolling to the bottom,
    /// the remaining number of rows triggers `load_more`.
    /// This should smaller than the total number of first load rows.
    ///
    /// Default: 20 rows
    pub fn load_more_threshold(mut self, threshold: usize) -> Self {
        self.load_more_threshold = threshold;
        self
    }

    pub fn set_rows_count(&mut self, rows_count: usize, cx: &mut Context<Self>) {
        self.rows_count = rows_count;
        cx.notify();
    }

    pub fn rows_count(&self) -> usize {
        self.rows_count
    }

    fn prepare_col_groups(columns: Vec<Column>) -> Vec<ColGroup> {
        columns
            .iter()
            .map(|column| ColGroup {
                width: column.width,
                bounds: Bounds::default(),
                column: column.clone(),
            })
            .collect()
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
        self.selection_state = SelectionState::Row;
        self.right_clicked_row = None;
        self.selected_row = Some(row_ix);
        if let Some(row_ix) = self.selected_row {
            self.vertical_scroll_handle
                .scroll_to_item(row_ix, ScrollStrategy::Top);
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
    pub fn visible_range(&self) -> &VisibleRangeState {
        &self.visible_range
    }

    fn on_row_click(
        &mut self,
        ev: &MouseDownEvent,
        row_ix: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if ev.button == MouseButton::Right {
            self.right_clicked_row = Some(row_ix);
        } else {
            self.set_selected_row(row_ix, cx);

            if ev.click_count == 2 {
                cx.emit(TableEvent::DoubleClickedRow(row_ix));
            }
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

    #[inline]
    fn has_selection(&self) -> bool {
        self.selected_row.is_some() || self.selected_col.is_some()
    }

    fn action_cancel(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_selection() {
            self.clear_selection(cx);
            return;
        }
        cx.propagate();
    }

    #[inline]
    fn columns_count(&self) -> usize {
        self.col_groups.len()
    }

    fn action_select_prev(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        let rows_count = self.rows_count;
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

    fn action_select_next(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        let rows_count = self.rows_count;
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

    fn action_select_prev_col(
        &mut self,
        _: &SelectPrevColumn,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut selected_col = self.selected_col.unwrap_or(0);
        let columns_count = self.columns_count();
        if selected_col > 0 {
            selected_col = selected_col.saturating_sub(1);
        } else {
            if self.loop_selection {
                selected_col = columns_count.saturating_sub(1);
            }
        }
        self.set_selected_col(selected_col, cx);
    }

    fn action_select_next_col(
        &mut self,
        _: &SelectNextColumn,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut selected_col = self.selected_col.unwrap_or(0);
        if selected_col < self.columns_count().saturating_sub(1) {
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

        const MIN_WIDTH: Pixels = px(10.0);
        const MAX_WIDTH: Pixels = px(1200.0);

        let Some(col_group) = self.col_groups.get_mut(ix) else {
            return;
        };

        if !col_group.is_resizable() {
            return;
        }
        let size = size.floor();

        let old_width = col_group.width;
        let new_width = size;
        if new_width < MIN_WIDTH {
            return;
        }
        let changed_width = new_width - old_width;
        // If change size is less than 1px, do nothing.
        if changed_width > px(-1.0) && changed_width < px(1.0) {
            return;
        }
        col_group.width = new_width.min(MAX_WIDTH);

        cx.notify();
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        on_sort: Rc<dyn Fn(usize, ColumnSort, &mut Window, &mut App)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

        (on_sort)(col_ix, sort, window, cx);

        cx.notify();
    }

    fn move_column(
        &mut self,
        col_ix: usize,
        to_ix: usize,
        on_move_column: Rc<dyn Fn(usize, usize, &mut Window, &mut App)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if col_ix == to_ix {
            return;
        }

        (on_move_column)(col_ix, to_ix, window, cx);
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
        load_more: Rc<dyn Fn(&mut Window, &mut App)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let threshold = self.load_more_threshold;
        // Securely handle subtract logic to prevent attempt to subtract with overflow
        if visible_end >= rows_count.saturating_sub(threshold) {
            self._load_more_task = cx.spawn_in(window, async move |view, window| {
                _ = view.update_in(window, |_, window, cx| {
                    (load_more)(window, cx);
                });
            });
        }
    }

    fn update_visible_range_if_need(
        &mut self,
        visible_range: Range<usize>,
        axis: Axis,
        on_visible_rows_changed: Rc<dyn Fn(Range<usize>, &mut Window, &mut App)>,
        on_visible_columns_changed: Rc<dyn Fn(Range<usize>, &mut Window, &mut App)>,
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
            (on_visible_rows_changed)(visible_range.clone(), window, cx);
            self.visible_range.rows = visible_range;
        } else {
            if self.visible_range.cols == visible_range {
                return;
            }
            (on_visible_columns_changed)(visible_range.clone(), window, cx);
            self.visible_range.cols = visible_range;
        }
    }

    fn render_cell(
        &self,
        col_ix: usize,
        size: Size,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Div {
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
            .table_cell_size(size)
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
                    .group_hover(group_id, |this| this.bg(cx.theme().border).h_full())
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
        render_context: &TableRenderContext,
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

        let on_sort = render_context.on_sort.clone();
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
                .on_click(cx.listener(move |state, _, window, cx| {
                    state.perform_sort(col_ix, on_sort.clone(), window, cx)
                }))
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
    fn render_th(
        &self,
        col_ix: usize,
        size: Size,
        render_context: &TableRenderContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let entity_id = cx.entity_id();
        let col_group = self.col_groups.get(col_ix).expect("BUG: invalid col index");

        let movable = self.col_movable && col_group.column.movable;
        let paddings = col_group.column.paddings;
        let name = col_group.column.name.clone();

        let on_move_column = render_context.on_move_column.clone();

        h_flex()
            .h_full()
            .child(
                self.render_cell(col_ix, size, window, cx)
                    .id(("col-header", col_ix))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            this.on_col_head_click(col_ix, window, cx);
                        }),
                    )
                    .child(
                        h_flex()
                            .size_full()
                            .justify_between()
                            .items_center()
                            .child((render_context.render_head)(
                                col_ix,
                                &col_group.column,
                                window,
                                cx,
                            ))
                            .when_some(paddings, |this, paddings| {
                                // Leave right space for the sort icon, if this column have custom padding
                                let offset_pr = size.table_cell_padding().right - paddings.right;
                                this.pr(offset_pr.max(px(0.)))
                            })
                            .children(self.render_sort_icon(
                                col_ix,
                                &col_group,
                                render_context,
                                window,
                                cx,
                            )),
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

                                table.move_column(
                                    drag.col_ix,
                                    col_ix,
                                    on_move_column.clone(),
                                    window,
                                    cx,
                                );
                            },
                        ))
                    }),
            )
            // resize handle
            .child(self.render_resize_handle(col_ix, window, cx))
            // to save the bounds of this col.
            .child({
                let state = cx.entity();
                canvas(
                    move |bounds, _, cx| {
                        state.update(cx, |state, _| state.col_groups[col_ix].bounds = bounds)
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            })
    }

    fn render_table_head(
        &self,
        left_columns_count: usize,
        size: Size,
        render_context: &TableRenderContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let horizontal_scroll_handle = self.horizontal_scroll_handle.clone();

        h_flex()
            .w_full()
            .h(size.table_row_height())
            .flex_shrink_0()
            .border_b_1()
            .border_color(cx.theme().border)
            .text_color(cx.theme().table_head_foreground)
            .when(left_columns_count > 0, |this| {
                let state = cx.entity();
                // Render left fixed columns
                this.child(
                    h_flex()
                        .relative()
                        .h_full()
                        .bg(cx.theme().table_head)
                        .children(
                            self.col_groups
                                .iter()
                                .filter(|col| col.column.fixed == Some(ColumnFixed::Left))
                                .enumerate()
                                .map(|(col_ix, _)| {
                                    self.render_th(col_ix, size, &render_context, window, cx)
                                }),
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
                        .child(
                            canvas(
                                move |bounds, _, cx| {
                                    state.update(cx, |r, _| r.fixed_head_cols_bounds = bounds)
                                },
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .size_full(),
                        ),
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
                                    .iter()
                                    .skip(left_columns_count)
                                    .enumerate()
                                    .map(|(col_ix, _)| {
                                        self.render_th(
                                            left_columns_count + col_ix,
                                            size,
                                            &render_context,
                                            window,
                                            cx,
                                        )
                                    }),
                            )
                            .child((render_context.render_last_empty_column)(window, cx)),
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
        extra_rows_count: usize,
        size: Size,
        stripe: bool,
        render_context: &TableRenderContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let horizontal_scroll_handle = self.horizontal_scroll_handle.clone();
        let is_stripe_row = stripe && row_ix % 2 != 0;
        let is_selected = self.selected_row == Some(row_ix);
        let view = cx.entity().clone();

        let render_cell = render_context.render_cell.clone();
        let on_visible_rows_changed = render_context.on_visible_rows_changed.clone();
        let on_visible_columns_changed = render_context.on_visible_columns_changed.clone();

        if row_ix < rows_count {
            let is_last_row = row_ix == rows_count - 1;
            let table_is_filled = extra_rows_count == 0;
            let need_render_border = if is_last_row {
                if is_selected {
                    true
                } else if table_is_filled {
                    false
                } else {
                    !stripe
                }
            } else {
                true
            };

            let mut tr = (render_context.render_row)(row_ix, window, cx);
            let style = tr.style().clone();

            tr.h_flex()
                .w_full()
                .h(size.table_row_height())
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
                                    items.push(
                                        self.render_col_wrap(col_ix, window, cx).child(
                                            self.render_cell(col_ix, size, window, cx)
                                                .child(render_cell(row_ix, col_ix, window, cx)),
                                        ),
                                    );
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
                                            on_visible_rows_changed.clone(),
                                            on_visible_columns_changed.clone(),
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
                                                    table
                                                        .render_cell(col_ix, size, window, cx)
                                                        .child(
                                                            render_cell(row_ix, col_ix, window, cx)
                                                                .into_any_element(),
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
                        .child((render_context.render_last_empty_column)(window, cx)),
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
                    MouseButton::Left,
                    cx.listener(move |this, ev, window, cx| {
                        this.on_row_click(ev, row_ix, window, cx);
                    }),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, ev, window, cx| {
                        this.on_row_click(ev, row_ix, window, cx);
                    }),
                )
        } else {
            // Render fake rows to fill the rest table space
            (render_context.render_row)(row_ix, window, cx)
                .h_flex()
                .w_full()
                .h_full()
                .border_t_1()
                .border_color(cx.theme().table_row_border)
                .when(is_stripe_row, |this| this.bg(cx.theme().table_even))
                .children((0..columns_count).map(|col_ix| {
                    h_flex()
                        .left(horizontal_scroll_handle.offset().x)
                        .child(self.render_cell(col_ix, size, window, cx))
                }))
                .child((render_context.render_last_empty_column)(window, cx))
        }
    }

    /// Calculate the extra rows needed to fill the table empty space when `stripe` is true.
    fn calculate_extra_rows_needed(&self, size: Size, rows_count: usize) -> usize {
        let mut extra_rows_needed = 0;

        let row_height = size.table_row_height();
        let total_height = self
            .vertical_scroll_handle
            .0
            .borrow()
            .base_handle
            .bounds()
            .size
            .height;

        let actual_height = row_height * rows_count as f32;
        let remaining_height = total_height - actual_height;

        if remaining_height > px(0.) {
            extra_rows_needed = (remaining_height / row_height).ceil() as usize;
        }

        extra_rows_needed
    }
}

impl EventEmitter<TableEvent> for TableState {}

impl Render for TableState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone)]
struct TableRenderContext {
    render_last_empty_column: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    render_head: Rc<dyn Fn(usize, &Column, &mut Window, &mut App) -> AnyElement>,
    render_row: Rc<dyn Fn(usize, &mut Window, &mut App) -> Stateful<Div>>,
    render_cell: Rc<dyn Fn(usize, usize, &mut Window, &mut App) -> AnyElement>,
    render_empty: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    render_loading: Rc<dyn Fn(Size, &mut Window, &mut App) -> AnyElement>,
    on_sort: Rc<dyn Fn(usize, ColumnSort, &mut Window, &mut App)>,
    on_move_column: Rc<dyn Fn(usize, usize, &mut Window, &mut App)>,
    on_load_more: Rc<dyn Fn(&mut Window, &mut App)>,
    on_visible_rows_changed: Rc<dyn Fn(Range<usize>, &mut Window, &mut App)>,
    on_visible_columns_changed: Rc<dyn Fn(Range<usize>, &mut Window, &mut App)>,
    context_menu: Rc<dyn Fn(usize, PopupMenu, &Window, &App) -> PopupMenu>,
}

impl Default for TableRenderContext {
    fn default() -> Self {
        Self {
            render_last_empty_column: Rc::new(|_, _| {
                h_flex().w_3().h_full().flex_shrink_0().into_any_element()
            }),
            render_cell: Rc::new(|_, _, _, _| div().into_any_element()),
            render_row: Rc::new(|row_ix, _, _| h_flex().id(("row", row_ix))),
            render_head: Rc::new(|_, column, _, _| {
                div()
                    .size_full()
                    .child(column.name.clone())
                    .into_any_element()
            }),
            render_empty: Rc::new(|_, cx| {
                h_flex()
                    .size_full()
                    .justify_center()
                    .text_color(cx.theme().muted_foreground.opacity(0.6))
                    .child(Icon::new(IconName::Inbox).size_12())
                    .into_any_element()
            }),
            render_loading: Rc::new(|size, _, _| Loading::new().size(size).into_any_element()),
            context_menu: Rc::new(|_, menu, _, _| menu),
            on_sort: Rc::new(|_, _, _, _| {}),
            on_move_column: Rc::new(|_, _, _, _| {}),
            on_load_more: Rc::new(|_, _| {}),
            on_visible_rows_changed: Rc::new(|_, _, _| {}),
            on_visible_columns_changed: Rc::new(|_, _, _| {}),
        }
    }
}

#[derive(IntoElement)]
pub struct Table {
    state: Entity<TableState>,
    render_context: TableRenderContext,

    scrollbar_visible: Edges<bool>,
    /// Set stripe style of the table.
    stripe: bool,
    /// Set to use border style of the table.
    border: bool,
    loading: bool,
    /// The cell size of the table.
    size: Size,
}

impl Table {
    pub fn new(state: &Entity<TableState>) -> Self {
        Self {
            state: state.clone(),
            stripe: false,
            border: true,
            loading: false,
            size: Size::default(),
            scrollbar_visible: Edges::all(true),
            render_context: TableRenderContext::default(),
        }
    }

    /// Set to use stripe style of the table, default to false.
    pub fn stripe(mut self, stripe: bool) -> Self {
        self.stripe = stripe;
        self
    }

    /// Set to use border style of the table, default to true.
    pub fn border(mut self, border: bool) -> Self {
        self.border = border;
        self
    }

    /// Set to loading state.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set scrollbar visibility.
    pub fn scrollbar_visible(mut self, vertical: bool, horizontal: bool) -> Self {
        self.scrollbar_visible = Edges {
            right: vertical,
            bottom: horizontal,
            ..Default::default()
        };
        self
    }

    /// Render the row at the given row and column if you want to customize the row element.
    ///
    /// Default is a h_flex with id ("row", row_ix).
    pub fn row<F>(mut self, f: F) -> Self
    where
        F: Fn(usize, &mut Window, &mut App) -> Stateful<Div> + 'static,
    {
        self.render_context.render_row = Rc::new(f);
        self
    }

    /// Render the header cell at the given column index, default to the column name.
    ///
    /// The callback arguments:
    ///
    /// - `(col_ix, column)`: The column index and column reference.
    /// - `&mut Window`: The window reference.
    /// - `&mut App`: The app context reference.
    pub fn head<F>(mut self, f: F) -> Self
    where
        F: Fn(usize, &mut Window, &mut App) -> AnyElement + 'static,
    {
        self.render_context.render_head =
            Rc::new(move |col_ix, _, window, cx| f(col_ix, window, cx).into_any_element());
        self
    }

    /// Custom last empty column element, default to a 12px width div.
    pub fn last_empty_column<F, E>(mut self, f: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.render_context.render_last_empty_column =
            Rc::new(move |window, cx| f(window, cx).into_any_element());
        self
    }

    /// Return a Element to show when table is empty.
    pub fn empty<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> AnyElement + 'static,
    {
        self.render_context.render_empty =
            Rc::new(move |window, cx| f(window, cx).into_any_element());
        self
    }

    /// Return a Element to show when table is loading, default is a Loading element.
    pub fn render_loading<F, E>(mut self, f: F) -> Self
    where
        F: Fn(Size, &mut Window, &mut App) -> AnyElement + 'static,
    {
        self.render_context.render_loading =
            Rc::new(move |size, window, cx| f(size, window, cx).into_any_element());
        self
    }

    /// Render cell at the given row index and column index.
    ///
    /// The callback arguments:
    ///
    /// - `(row, col)`: The row and column index tuple.
    /// - `&mut Window`: The window reference.
    /// - `&mut App`: The app context reference.
    pub fn cell<F>(mut self, f: F) -> Self
    where
        F: Fn((usize, usize), &mut Window, &mut App) -> AnyElement + 'static,
    {
        self.render_context.render_cell = Rc::new(move |row_ix, col_ix, window, cx| {
            f((row_ix, col_ix), window, cx).into_any_element()
        });
        self
    }

    /// Render the context menu for the row at the given row index.
    ///
    /// The callback arguments:
    ///
    /// - `(row_ix, menu)`: The row index and the current PopupMenu.
    pub fn context_menu<F>(mut self, f: F) -> Self
    where
        F: Fn((usize, PopupMenu), &Window, &App) -> PopupMenu + 'static,
    {
        self.render_context.context_menu =
            Rc::new(move |row_ix, menu, window, cx| f((row_ix, menu), window, cx));
        self
    }

    /// Add callback to listen to sort.
    ///
    /// The callback arguments:
    ///
    /// - `(col_ix, sort)`: The column index and the new ColumnSort.
    pub fn on_sort<F>(mut self, f: F) -> Self
    where
        F: Fn((usize, ColumnSort), &mut Window, &mut App) + 'static,
    {
        self.render_context.on_sort =
            Rc::new(move |col_ix, sort, window, cx| f((col_ix, sort), window, cx));
        self
    }

    /// Add callback to listen to move column.
    ///
    /// The callback arguments:
    ///
    /// - `(from_col_ix, to_col_ix)`: The `from` column index and `to` column index.
    pub fn on_move_column<F>(mut self, f: F) -> Self
    where
        F: Fn((usize, usize), &mut Window, &mut App) + 'static,
    {
        self.render_context.on_move_column = Rc::new(move |from_col_ix, to_col_ix, window, cx| {
            f((from_col_ix, to_col_ix), window, cx)
        });
        self
    }

    /// Add callback to listen to load more rows.
    pub fn on_visible_rows_changed<F>(mut self, f: F) -> Self
    where
        F: Fn(Range<usize>, &mut Window, &mut App) + 'static,
    {
        self.render_context.on_visible_rows_changed =
            Rc::new(move |range, window, cx| f(range, window, cx));
        self
    }

    /// Add callback to listen to visible columns changed.
    pub fn on_visible_columns_changed<F>(mut self, f: F) -> Self
    where
        F: Fn(Range<usize>, &mut Window, &mut App) + 'static,
    {
        self.render_context.on_visible_columns_changed =
            Rc::new(move |range, window, cx| f(range, window, cx));
        self
    }

    /// Add callback to load more rows when scrolling to the bottom.
    pub fn on_load_more<F>(mut self, f: F) -> Self
    where
        F: Fn(usize, &mut Window, &mut App) + 'static,
    {
        self.render_context.on_load_more = Rc::new(move |window, cx| f(0, window, cx));
        self
    }

    fn render_vertical_scrollbar(
        &self,
        size: Size,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement> {
        let state = self.state.read(cx);

        Some(
            div()
                .occlude()
                .absolute()
                .top(size.table_row_height())
                .right_0()
                .bottom_0()
                .w(Scrollbar::width())
                .on_scroll_wheel(window.listener_for(
                    &self.state,
                    |_, _: &ScrollWheelEvent, _, cx| {
                        cx.notify();
                    },
                ))
                .child(
                    Scrollbar::uniform_scroll(
                        &state.vertical_scroll_state,
                        &state.vertical_scroll_handle,
                    )
                    .max_fps(60),
                ),
        )
    }

    fn render_horizontal_scrollbar(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let left = if state.fixed_left_cols_count() > 0 {
            state.fixed_head_cols_bounds.size.width
        } else {
            px(0.)
        };

        div()
            .occlude()
            .absolute()
            .left(left)
            .right_0()
            .bottom_0()
            .h(Scrollbar::width())
            .on_scroll_wheel(
                window.listener_for(&self.state, |_, _: &ScrollWheelEvent, _, cx| {
                    cx.notify();
                }),
            )
            .child(Scrollbar::horizontal(
                &state.horizontal_scroll_state,
                &state.horizontal_scroll_handle,
            ))
    }
}

impl Sizable for Table {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Focusable for TableState {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl RenderOnce for Table {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);

        let loading = self.loading;
        let left_columns_count = state.fixed_left_cols_count();
        let focus_handle = state.focus_handle.clone();
        let vertical_scroll_handle = state.vertical_scroll_handle.clone();
        let horizontal_scroll_handle = state.horizontal_scroll_handle.clone();
        let right_clicked_row = state.right_clicked_row;
        let columns_count = state.columns_count();
        let rows_count = state.rows_count;
        let extra_rows_count = state.calculate_extra_rows_needed(self.size, rows_count);
        let render_rows_count = if self.stripe {
            rows_count + extra_rows_count
        } else {
            rows_count
        };

        let inner_table = v_flex()
            .key_context("Table")
            .id("table")
            .track_focus(&focus_handle)
            .on_action(window.listener_for(&self.state, TableState::action_cancel))
            .on_action(window.listener_for(&self.state, TableState::action_select_next))
            .on_action(window.listener_for(&self.state, TableState::action_select_prev))
            .on_action(window.listener_for(&self.state, TableState::action_select_next_col))
            .on_action(window.listener_for(&self.state, TableState::action_select_prev_col))
            .size_full()
            .overflow_hidden()
            .child(self.state.update(cx, |state, cx| {
                state.render_table_head(
                    left_columns_count,
                    self.size,
                    &self.render_context,
                    window,
                    cx,
                )
            }))
            .context_menu({
                let context_menu = self.render_context.context_menu.clone();
                move |this, window: &mut Window, cx: &mut Context<PopupMenu>| {
                    if let Some(row_ix) = right_clicked_row {
                        (context_menu)(row_ix, this, window, cx)
                    } else {
                        this
                    }
                }
            })
            .map(|this| {
                if rows_count == 0 {
                    this.child(
                        div()
                            .size_full()
                            .child((self.render_context.render_empty)(window, cx)),
                    )
                } else {
                    this.child(h_flex().id("table-body").flex_grow().size_full().child(
                        self.state.update(cx, |_, cx| {
                            uniform_list("table-uniform-list", render_rows_count, {
                                let render_context = self.render_context.clone();
                                let size = self.size;
                                let stripe = self.stripe;
                                cx.processor(
                                    move |state, visible_range: Range<usize>, window, cx| {
                                        // We must calculate the col sizes here, because the col sizes
                                        // need render_th first, then that method will set the bounds of each col.
                                        let col_sizes: Rc<Vec<gpui::Size<Pixels>>> = Rc::new(
                                            state
                                                .col_groups
                                                .iter()
                                                .skip(left_columns_count)
                                                .map(|col| col.bounds.size)
                                                .collect(),
                                        );

                                        state.load_more_if_need(
                                            rows_count,
                                            visible_range.end,
                                            render_context.on_load_more.clone(),
                                            window,
                                            cx,
                                        );

                                        state.update_visible_range_if_need(
                                            visible_range.clone(),
                                            Axis::Vertical,
                                            render_context.on_visible_rows_changed.clone(),
                                            render_context.on_visible_columns_changed.clone(),
                                            window,
                                            cx,
                                        );

                                        if visible_range.end > rows_count {
                                            state.scroll_to_row(
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
                                            items.push(state.render_table_row(
                                                row_ix,
                                                rows_count,
                                                left_columns_count,
                                                col_sizes.clone(),
                                                columns_count,
                                                extra_rows_count,
                                                size,
                                                stripe,
                                                &render_context,
                                                window,
                                                cx,
                                            ));
                                        });

                                        items
                                    },
                                )
                            })
                            .flex_grow()
                            .size_full()
                            .with_sizing_behavior(ListSizingBehavior::Auto)
                            .track_scroll(vertical_scroll_handle)
                            .into_any_element()
                        }),
                    ))
                }
            });

        let state = self.state.clone();
        div()
            .size_full()
            .when(self.border, |this| {
                this.rounded(cx.theme().radius)
                    .border_1()
                    .border_color(cx.theme().border)
            })
            .bg(cx.theme().table)
            .when(loading, |this| {
                this.child((self.render_context.render_loading)(self.size, window, cx))
            })
            .when(!loading, |this| {
                this.child(inner_table)
                    .child(ScrollableMask::new(
                        self.state.entity_id(),
                        Axis::Horizontal,
                        &horizontal_scroll_handle,
                    ))
                    .when(right_clicked_row.is_some(), |this| {
                        this.on_mouse_down_out(window.listener_for(&state, |this, _, _, cx| {
                            this.right_clicked_row = None;
                            cx.notify();
                        }))
                    })
            })
            .child(canvas(
                move |bounds, _, cx| state.update(cx, |r, _| r.bounds = bounds),
                |_, _, _, _| {},
            ))
            .when(!window.is_inspector_picking(cx), |this| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .size_full()
                        .when(self.scrollbar_visible.bottom, |this| {
                            this.child(self.render_horizontal_scrollbar(window, cx))
                        })
                        .when(self.scrollbar_visible.right && rows_count > 0, |this| {
                            this.children(self.render_vertical_scrollbar(self.size, window, cx))
                        }),
                )
            })
    }
}
