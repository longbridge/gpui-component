use std::{collections::HashSet, ops::Range, rc::Rc, time::Duration};

use super::filter_state::FilterState;
use super::selection::{CellCoord, TableSelection};
use super::*;
use crate::edit_table::filter_panel::FilterPanel;
use gpui::{
    AppContext, Axis, Bounds, ClickEvent, ClipboardItem, Context, Div, DragMoveEvent, ElementId,
    Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, IsZero,
    ListSizingBehavior, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Render,
    ScrollStrategy, ScrollWheelEvent, SharedString, Stateful, StatefulInteractiveElement as _,
    Styled, Subscription, Task, UniformListScrollHandle, Window, canvas, div,
    prelude::FluentBuilder, px, uniform_list,
};
use gpui_component::list::{List, ListState};
use gpui_component::scroll::ScrollbarHandle;
use gpui_component::{
    ActiveTheme, Icon, IconName, StyleSized as _, StyledExt, VirtualListScrollHandle, h_flex,
    input::{IndentInline, OutdentInline},
    menu::{ContextMenuExt, PopupMenu},
    scroll::{ScrollableMask, Scrollbar},
    v_flex,
};

const SCROLLBAR_WIDTH: Pixels = px(16.);

gpui::actions!(
    edit_table_internal,
    [
        Cancel,
        Confirm,
        SelectDown,
        SelectUp,
        SelectFirst,
        SelectLast,
        SelectPageUp,
        SelectPageDown,
        Copy,
        Paste,
        SelectAll
    ]
);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SelectionState {
    Column,
    Row,
    Cell,
}

#[derive(Clone)]
pub enum EditTableEvent {
    SelectRow(usize),
    DoubleClickedCell(usize, usize),
    SelectColumn(usize),
    SelectCell(usize, usize),
    ColumnWidthsChanged(Vec<Pixels>),
    MoveColumn(usize, usize),
    CellEditing(usize, usize),
    CellEdited(usize, usize),
    RowAdded,
    RowDeleted(usize),
    /// 选区变化事件
    SelectionChanged(TableSelection),
    /// 复制数据事件
    CopyData(Vec<Vec<String>>),
    /// 粘贴数据事件
    PasteData {
        data: Vec<Vec<String>>,
        start: CellCoord,
    },
}

#[derive(Debug, Default)]
pub struct TableVisibleRange {
    rows: Range<usize>,
    cols: Range<usize>,
}

impl TableVisibleRange {
    pub fn rows(&self) -> &Range<usize> {
        &self.rows
    }

    pub fn cols(&self) -> &Range<usize> {
        &self.cols
    }
}

pub struct EditTableState<D: EditTableDelegate> {
    focus_handle: FocusHandle,
    delegate: D,
    pub(super) options: TableOptions,
    bounds: Bounds<Pixels>,
    fixed_head_cols_bounds: Bounds<Pixels>,

    col_groups: Vec<ColGroup>,

    pub loop_selection: bool,
    pub col_selectable: bool,
    pub row_selectable: bool,
    pub sortable: bool,
    pub col_resizable: bool,
    pub col_movable: bool,
    pub col_fixed: bool,
    pub col_filterable: bool,

    pub vertical_scroll_handle: UniformListScrollHandle,
    pub horizontal_scroll_handle: VirtualListScrollHandle,

    selected_row: Option<usize>,
    selection_state: SelectionState,
    right_clicked_row: Option<usize>,
    selected_col: Option<usize>,
    selected_cell: Option<(usize, usize)>,
    resizing_col: Option<usize>,

    editing_cell: Option<(usize, usize)>,
    editing_input: Option<CellEditor>,
    _subscriptions: Vec<Subscription>,

    visible_range: TableVisibleRange,

    filter_state: FilterState,
    filter_list: Option<Entity<ListState<FilterPanel>>>,
    active_filter_col: Option<usize>,

    _measure: Vec<Duration>,
    _load_more_task: Task<()>,

    /// 多选区状态
    selection: TableSelection,
    /// 是否正在拖选
    is_selecting: bool,
    /// 拖选结束的单元格位置（用于跳过该单元格的 on_click 事件）
    drag_end_cell: Option<(usize, usize)>,
    /// 拖选起始位置和是否添加到选区
    drag_start: Option<(usize, usize, bool)>,
    /// 单元格边界缓存（用于拖选时的命中测试）
    cell_bounds: std::collections::HashMap<(usize, usize), Bounds<Pixels>>,
}

impl<D> EditTableState<D>
where
    D: EditTableDelegate,
{
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
            selected_cell: None,
            resizing_col: None,
            editing_cell: None,
            editing_input: None,
            bounds: Bounds::default(),
            fixed_head_cols_bounds: Bounds::default(),
            visible_range: TableVisibleRange::default(),
            filter_state: FilterState::new(),
            filter_list: None,
            active_filter_col: None,
            loop_selection: true,
            col_selectable: true,
            row_selectable: true,
            sortable: true,
            col_movable: true,
            col_resizable: true,
            col_fixed: true,
            col_filterable: true,
            _load_more_task: Task::ready(()),
            _measure: Vec::new(),
            _subscriptions: Vec::new(),
            selection: TableSelection::new(),
            is_selecting: false,
            drag_end_cell: None,
            drag_start: None,
            cell_bounds: std::collections::HashMap::new(),
        };

        this.prepare_col_groups(cx);
        this
    }

    pub fn delegate(&self) -> &D {
        &self.delegate
    }

    pub fn delegate_mut(&mut self) -> &mut D {
        &mut self.delegate
    }

    pub fn loop_selection(mut self, loop_selection: bool) -> Self {
        self.loop_selection = loop_selection;
        self
    }

    pub fn col_movable(mut self, col_movable: bool) -> Self {
        self.col_movable = col_movable;
        self
    }

    pub fn col_resizable(mut self, col_resizable: bool) -> Self {
        self.col_resizable = col_resizable;
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    pub fn row_selectable(mut self, row_selectable: bool) -> Self {
        self.row_selectable = row_selectable;
        self
    }

    pub fn col_selectable(mut self, col_selectable: bool) -> Self {
        self.col_selectable = col_selectable;
        self
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.prepare_col_groups(cx);
    }

    pub fn scroll_to_row(&mut self, row_ix: usize, cx: &mut Context<Self>) {
        self.vertical_scroll_handle
            .scroll_to_item(row_ix, ScrollStrategy::Top);
        cx.notify();
    }

    pub fn scroll_to_col(&mut self, col_ix: usize, cx: &mut Context<Self>) {
        let col_ix = col_ix.saturating_sub(self.fixed_left_cols_count());

        self.horizontal_scroll_handle
            .scroll_to_item(col_ix, ScrollStrategy::Top);
        cx.notify();
    }

    fn handle_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let line_height = window.line_height();
        let delta = event.delta.pixel_delta(line_height);
        if delta.y.is_zero() {
            return;
        }

        let base_handle = &self.vertical_scroll_handle.0.borrow().base_handle;
        let offset_y = base_handle.offset().y;
        let max_offset_y = -base_handle.max_offset().height;
        if max_offset_y.is_zero() {
            return;
        }

        let tolerance = px(0.5);
        let at_top = offset_y >= -tolerance;
        let at_bottom = offset_y <= max_offset_y + tolerance;

        if (delta.y > Pixels::ZERO && !at_top) || (delta.y < Pixels::ZERO && !at_bottom) {
            cx.stop_propagation();
        }
    }

    pub fn selected_row(&self) -> Option<usize> {
        self.selected_row
    }

    pub fn set_selected_row(&mut self, row_ix: usize, cx: &mut Context<Self>) {
        let is_down = match self.selected_row {
            Some(selected_row) => row_ix > selected_row,
            None => true,
        };

        self.selection_state = SelectionState::Row;
        self.right_clicked_row = None;
        self.selected_row = Some(row_ix);
        self.selected_col = None;
        self.selected_cell = None;
        // 设置选区为整行，支持复制功能
        // 列范围从 row_number_offset 开始（跳过行号列），到 col_groups.len() - 1 结束
        let row_number_offset = if self.delegate.row_number_enabled(cx) {
            1
        } else {
            0
        };
        let end_col = self.col_groups.len().saturating_sub(1);
        self.selection
            .select_row(row_ix, row_number_offset, end_col);
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
        cx.emit(EditTableEvent::SelectRow(row_ix));
        cx.notify();
    }

    pub fn selected_col(&self) -> Option<usize> {
        self.selected_col
    }

    pub fn set_selected_col(&mut self, col_ix: usize, cx: &mut Context<Self>) {
        self.selection_state = SelectionState::Column;
        self.selected_col = Some(col_ix);
        self.selected_row = None;
        self.selected_cell = None;
        // 清除多选区，确保列选中和单元格选中互斥
        self.selection.clear();
        if let Some(col_ix) = self.selected_col {
            self.scroll_to_col(col_ix, cx);
        }
        cx.emit(EditTableEvent::SelectColumn(col_ix));
        cx.notify();
    }

    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.selection_state = SelectionState::Row;
        self.selected_row = None;
        self.selected_col = None;
        self.selected_cell = None;
        // 同时清除多选区
        self.selection.clear();
        cx.notify();
    }

    pub fn selected_cell(&self) -> Option<(usize, usize)> {
        self.selected_cell
    }

    pub fn set_selected_cell(&mut self, row_ix: usize, col_ix: usize, cx: &mut Context<Self>) {
        self.selection_state = SelectionState::Cell;
        self.selected_cell = Some((row_ix, col_ix));
        self.selected_col = None;
        self.selected_row = None;
        self.scroll_to_col(col_ix, cx);
        cx.emit(EditTableEvent::SelectCell(row_ix, col_ix));
        cx.notify();
    }

    fn has_cell_selection(&self) -> bool {
        self.selection_state == SelectionState::Cell
    }

    fn current_cell_for_navigation(&self) -> Option<(usize, usize)> {
        self.selection.active.or(self.selected_cell)
    }

    fn first_data_col_ix(&self, cx: &App) -> usize {
        if self.delegate.row_number_enabled(cx) {
            1
        } else {
            0
        }
    }

    fn last_data_col_ix(&self, cx: &App) -> Option<usize> {
        let first_col_ix = self.first_data_col_ix(cx);
        (self.col_groups.len() > first_col_ix).then_some(self.col_groups.len() - 1)
    }

    fn select_cell_for_navigation(&mut self, row_ix: usize, col_ix: usize, cx: &mut Context<Self>) {
        self.select_cell(row_ix, col_ix, cx);
        self.vertical_scroll_handle
            .scroll_to_item(row_ix, ScrollStrategy::Center);
    }

    fn move_to_prev_cell(&mut self, cx: &mut Context<Self>) {
        let rows_count = self.delegate.rows_count(cx);
        let Some(last_col_ix) = self.last_data_col_ix(cx) else {
            return;
        };
        if rows_count == 0 {
            return;
        }

        let first_col_ix = self.first_data_col_ix(cx);
        if let Some((row_ix, col_ix)) = self.current_cell_for_navigation() {
            let current_col_ix = col_ix.clamp(first_col_ix, last_col_ix);
            let new_col_ix = if current_col_ix > first_col_ix {
                current_col_ix.saturating_sub(1)
            } else if self.loop_selection {
                last_col_ix
            } else {
                current_col_ix
            };
            self.select_cell_for_navigation(row_ix, new_col_ix, cx);
        } else {
            self.select_cell_for_navigation(0, first_col_ix, cx);
        }
    }

    fn move_to_next_cell(&mut self, cx: &mut Context<Self>) {
        let rows_count = self.delegate.rows_count(cx);
        let Some(last_col_ix) = self.last_data_col_ix(cx) else {
            return;
        };
        if rows_count == 0 {
            return;
        }

        let first_col_ix = self.first_data_col_ix(cx);
        if let Some((row_ix, col_ix)) = self.current_cell_for_navigation() {
            let current_col_ix = col_ix.clamp(first_col_ix, last_col_ix);
            let new_col_ix = if current_col_ix < last_col_ix {
                current_col_ix + 1
            } else if self.loop_selection {
                first_col_ix
            } else {
                current_col_ix
            };
            self.select_cell_for_navigation(row_ix, new_col_ix, cx);
        } else {
            self.select_cell_for_navigation(0, first_col_ix, cx);
        }
    }

    fn commit_and_move_to_prev_cell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing_cell.is_none() {
            cx.propagate();
            return;
        }

        self.commit_cell_edit(window, cx);
        self.focus_handle.focus(window, cx);
        self.move_to_prev_cell(cx);
    }

    fn commit_and_move_to_next_cell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing_cell.is_none() {
            cx.propagate();
            return;
        }

        self.commit_cell_edit(window, cx);
        self.focus_handle.focus(window, cx);
        self.move_to_next_cell(cx);
    }

    // ==================== 多选区方法 ====================

    /// 获取当前选区
    pub fn selection(&self) -> &TableSelection {
        &self.selection
    }

    /// 获取可变选区引用
    pub fn selection_mut(&mut self) -> &mut TableSelection {
        &mut self.selection
    }

    /// 选择单个单元格（替换现有选区）
    pub fn select_cell(&mut self, row_ix: usize, col_ix: usize, cx: &mut Context<Self>) {
        self.selection.select_single((row_ix, col_ix));
        self.sync_legacy_selection(cx);
        cx.emit(EditTableEvent::SelectCell(row_ix, col_ix));
        cx.emit(EditTableEvent::SelectionChanged(self.selection.clone()));
        cx.notify();
    }

    /// 扩展选区到指定单元格（Shift+Click）
    pub fn extend_selection_to(&mut self, row_ix: usize, col_ix: usize, cx: &mut Context<Self>) {
        tracing::debug!(
            "extend_selection_to: row={}, col={}, anchor={:?}",
            row_ix,
            col_ix,
            self.selection.anchor
        );
        self.selection.extend_to((row_ix, col_ix));
        tracing::debug!(
            "selection after extend: ranges={:?}, active={:?}",
            self.selection.ranges.len(),
            self.selection.active
        );
        self.sync_legacy_selection(cx);
        cx.emit(EditTableEvent::SelectionChanged(self.selection.clone()));
        cx.notify();
    }

    /// 添加单元格到选区（Ctrl+Click）
    pub fn add_to_selection(&mut self, row_ix: usize, col_ix: usize, cx: &mut Context<Self>) {
        tracing::debug!("add_to_selection: row={}, col={}", row_ix, col_ix);
        self.selection.add((row_ix, col_ix));
        tracing::debug!(
            "selection after add: ranges={:?}, active={:?}",
            self.selection.ranges.len(),
            self.selection.active
        );
        self.sync_legacy_selection(cx);
        cx.emit(EditTableEvent::SelectionChanged(self.selection.clone()));
        cx.notify();
    }

    /// 清除选区
    pub fn clear_cell_selection(&mut self, cx: &mut Context<Self>) {
        self.selection.clear();
        self.sync_legacy_selection(cx);
        cx.notify();
    }

    /// 选择所有单元格
    pub fn select_all_cells(&mut self, cx: &mut Context<Self>) {
        let row_count = self.delegate.rows_count(cx);
        let col_count = self.col_groups.len();
        let row_number_offset = if self.delegate.row_number_enabled(cx) {
            1
        } else {
            0
        };
        self.selection
            .select_all(row_count, col_count.saturating_sub(row_number_offset));
        self.sync_legacy_selection(cx);
        cx.emit(EditTableEvent::SelectionChanged(self.selection.clone()));
        cx.notify();
    }

    /// 检查单元格是否在选区内
    pub fn is_cell_in_selection(&self, row_ix: usize, col_ix: usize) -> bool {
        self.selection.contains(row_ix, col_ix)
    }

    /// 获取活动单元格
    pub fn active_cell(&self) -> Option<CellCoord> {
        self.selection.active
    }

    /// 同步选区状态到旧的单选字段（向后兼容）
    fn sync_legacy_selection(&mut self, cx: &mut Context<Self>) {
        self.selection_state = SelectionState::Cell;
        self.selected_cell = self.selection.active;
        self.selected_row = None;
        self.selected_col = None;

        // 如果有活动单元格，滚动到可见
        if let Some((_, col_ix)) = self.selection.active {
            self.scroll_to_col(col_ix, cx);
        }
    }

    /// 准备拖选（记录起始位置，但不立即开始选区）
    pub fn prepare_drag_selection(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        add_to_selection: bool,
        _cx: &mut Context<Self>,
    ) {
        self.drag_start = Some((row_ix, col_ix, add_to_selection));
    }

    /// 开始拖选
    pub fn start_drag_selection(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        add_to_selection: bool,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;
        self.drag_start = None;
        self.selection
            .start_drag((row_ix, col_ix), add_to_selection);
        self.sync_legacy_selection(cx);
        cx.notify();
    }

    /// 更新拖选
    pub fn update_drag_selection(&mut self, row_ix: usize, col_ix: usize, cx: &mut Context<Self>) {
        // 如果有待处理的拖选起始位置（备用路径，正常情况下 on_drag 已经消耗了）
        if let Some((start_row, start_col, add_to_selection)) = self.drag_start.take() {
            tracing::debug!(
                "update_drag_selection: starting drag from ({},{}) to ({},{})",
                start_row,
                start_col,
                row_ix,
                col_ix
            );
            self.is_selecting = true;
            self.selection
                .start_drag((start_row, start_col), add_to_selection);
            // 如果拖动到了不同单元格，更新选区范围
            if start_row != row_ix || start_col != col_ix {
                self.selection.update_drag((row_ix, col_ix));
            }
            self.sync_legacy_selection(cx);
            cx.notify();
        } else if self.is_selecting {
            tracing::debug!("update_drag_selection: updating to ({},{})", row_ix, col_ix);
            self.selection.update_drag((row_ix, col_ix));
            self.sync_legacy_selection(cx);
            cx.notify();
        }
    }

    /// 结束拖选
    pub fn end_drag_selection(&mut self, cx: &mut Context<Self>) {
        self.drag_start = None;
        if self.is_selecting {
            self.is_selecting = false;
            // 记录拖选结束的单元格位置，用于跳过该单元格的点击事件
            self.drag_end_cell = self.selection.active;
            self.sync_legacy_selection(cx);
            cx.emit(EditTableEvent::SelectionChanged(self.selection.clone()));
            cx.notify();
        }
    }

    pub fn visible_range(&self) -> &TableVisibleRange {
        &self.visible_range
    }

    pub fn filter_state(&self) -> &FilterState {
        &self.filter_state
    }

    pub fn filter_state_mut(&mut self) -> &mut FilterState {
        &mut self.filter_state
    }

    pub fn set_column_filter(
        &mut self,
        col_ix: usize,
        selected_values: HashSet<String>,
        cx: &mut Context<Self>,
    ) {
        self.filter_state.set_filter(col_ix, selected_values);
        cx.notify();
    }

    pub fn set_column_filter_with_all_values(
        &mut self,
        col_ix: usize,
        selected_values: HashSet<String>,
        cx: &mut Context<Self>,
    ) {
        let filter_values = self.delegate.get_column_filter_values(col_ix, cx);
        let all_values: HashSet<String> = filter_values
            .iter()
            .map(|fv| fv.value.to_string())
            .collect();

        self.filter_state
            .set_filter_with_all_values(col_ix, selected_values, all_values);
        cx.notify();
    }

    pub fn clear_column_filter(&mut self, col_ix: usize, cx: &mut Context<Self>) {
        self.filter_state.clear_filter(col_ix);
        cx.notify();
    }

    pub fn clear_all_filters(&mut self, cx: &mut Context<Self>) {
        self.filter_state.clear_all();
        cx.notify();
    }

    pub fn open_filter_panel(
        &mut self,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let filter_values = self.delegate.get_column_filter_values(col_ix, cx);
        let current_filter = self.filter_state.get_filter(col_ix);
        let values = filter_values
            .into_iter()
            .map(|mut fv| {
                let checked = current_filter
                    .map(|f| f.selected_values.contains(&fv.value))
                    .unwrap_or(false);
                fv.checked = checked;
                fv
            })
            .collect();

        let table_entity = cx.entity().clone();
        let filter_panel = FilterPanel::new(values).on_toggle(move |value, window, cx| {
            table_entity.update(cx, |table, cx| {
                table.toggle_filter_value_realtime(col_ix, value, window, cx);
            });
        });

        self.filter_list =
            Some(cx.new(|cx| ListState::new(filter_panel, window, cx).searchable(true)));
        self.active_filter_col = Some(col_ix);
        cx.notify();
    }

    pub fn close_filter_panel(&mut self, cx: &mut Context<Self>) {
        self.active_filter_col = None;
        self.filter_list = None;
        cx.notify();
    }

    pub fn toggle_filter_value_realtime(
        &mut self,
        col_ix: usize,
        value: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.modify_filter_panel_realtime(col_ix, window, cx, |panel| {
            panel.toggle_value(value);
        });
    }

    pub fn filter_panel_select_all_realtime(
        &mut self,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.modify_filter_panel_realtime(col_ix, window, cx, |panel| {
            panel.select_all();
        });
    }

    pub fn filter_panel_deselect_all_realtime(
        &mut self,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.modify_filter_panel_realtime(col_ix, window, cx, |panel| {
            panel.deselect_all();
        });
    }

    pub fn filter_panel_reset_realtime(
        &mut self,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_filter_panel(cx, |panel| {
            panel.select_all();
        });
        self.apply_filter_realtime(col_ix, window, cx);
    }

    fn apply_filter_realtime(
        &mut self,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(selected) =
            self.read_filter_panel_with_ctx(cx, |panel| panel.get_selected_values())
        {
            if selected.is_empty() {
                self.clear_column_filter(col_ix, cx);
                self.delegate.on_column_filter_cleared(col_ix, window, cx);
            } else {
                self.set_column_filter_with_all_values(col_ix, selected.clone(), cx);
                self.delegate
                    .on_column_filter_changed(col_ix, selected, window, cx);
            }

            self.refresh(cx);
        }
    }

    pub fn toggle_filter_value(&mut self, value: &str, cx: &mut Context<Self>) {
        self.update_filter_panel(cx, |panel| {
            panel.toggle_value(value);
        });
    }

    pub fn filter_panel_select_all(&mut self, cx: &mut Context<Self>) {
        self.update_filter_panel(cx, |panel| {
            panel.select_all();
        });
    }

    pub fn filter_panel_deselect_all(&mut self, cx: &mut Context<Self>) {
        self.update_filter_panel(cx, |panel| {
            panel.deselect_all();
        });
    }

    pub fn filter_panel_reset(&mut self, cx: &mut Context<Self>) {
        self.update_filter_panel(cx, |panel| {
            panel.select_all();
        });
        cx.notify();
    }

    pub fn confirm_filter_panel(&mut self, cx: &mut Context<Self>) {
        if let Some(col_ix) = self.active_filter_col {
            if let Some(selected) =
                self.read_filter_panel_with_ctx(cx, |panel| panel.get_selected_values())
            {
                if selected.is_empty() {
                    self.clear_column_filter(col_ix, cx);
                } else {
                    self.set_column_filter(col_ix, selected, cx);
                }
            }

            self.close_filter_panel(cx);
        }
    }

    pub fn filter_panel_selected_count(&self, cx: &gpui::App) -> usize {
        self.read_filter_panel_with_app(cx, |panel| panel.get_selected_values().len())
            .unwrap_or(0)
    }

    pub fn filter_panel_total_count(&self, cx: &gpui::App) -> usize {
        self.read_filter_panel_with_app(cx, |panel| panel.values.len())
            .unwrap_or(0)
    }

    fn modify_filter_panel_realtime<F>(
        &mut self,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: F,
    ) where
        F: FnOnce(&mut FilterPanel),
    {
        let mut action = Some(f);
        self.update_filter_panel(cx, move |panel| {
            if let Some(func) = action.take() {
                func(panel);
            }
        });

        self.apply_filter_realtime(col_ix, window, cx);
    }

    fn update_filter_panel<F>(&mut self, cx: &mut Context<Self>, mut f: F)
    where
        F: FnMut(&mut FilterPanel),
    {
        if let Some(filter_list) = &self.filter_list {
            filter_list.update(cx, |list_state, cx| {
                f(list_state.delegate_mut());
                cx.notify();
            });
        }
    }

    fn read_filter_panel_with_ctx<R, F>(&self, cx: &mut Context<Self>, f: F) -> Option<R>
    where
        F: FnOnce(&FilterPanel) -> R,
    {
        self.filter_list.as_ref().map(|filter_list| {
            filter_list.read_with(cx, |list_state: &ListState<FilterPanel>, _| {
                f(list_state.delegate())
            })
        })
    }

    fn read_filter_panel_with_app<R, F>(&self, cx: &gpui::App, f: F) -> Option<R>
    where
        F: FnOnce(&FilterPanel) -> R,
    {
        self.filter_list.as_ref().map(|filter_list| {
            filter_list.read_with(cx, |list_state: &ListState<FilterPanel>, _| {
                f(list_state.delegate())
            })
        })
    }

    fn prepare_col_groups(&mut self, cx: &mut Context<Self>) {
        let mut col_groups = Vec::new();

        if self.delegate.row_number_enabled(cx) {
            col_groups.push(ColGroup {
                width: px(60.),
                bounds: Bounds::default(),
                column: Column::new("__row_number__", " ")
                    .width(px(60.))
                    .resizable(false)
                    .movable(false)
                    .selectable(false)
                    .text_right(),
            });
        }

        col_groups.extend((0..self.delegate.columns_count(cx)).map(|col_ix| {
            let column = self.delegate().column(col_ix, cx);
            ColGroup {
                width: column.width,
                bounds: Bounds::default(),
                column: column.clone(),
            }
        }));

        self.col_groups = col_groups;
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

    fn page_item_count(&self) -> usize {
        let row_height = self.options.size.table_row_height();
        let height = self.bounds.size.height;
        let count = (height / row_height).floor() as usize;
        count.saturating_sub(1).max(1)
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
            cx.emit(EditTableEvent::DoubleClickedCell(row_ix, 0))
        }
    }

    fn on_cell_click(
        &mut self,
        e: &ClickEvent,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 如果是拖选刚结束在同一个单元格，跳过这次点击事件
        if let Some(drag_end) = self.drag_end_cell.take() {
            if drag_end == (row_ix, col_ix) {
                return;
            }
        }

        let edit_enabled = self.delegate.cell_edit_enabled(cx);
        let multi_select_enabled = self.delegate.multi_select_enabled(cx);
        let is_row_number_col = self.delegate.row_number_enabled(cx) && col_ix == 0;

        // 处理正在编辑的单元格
        if edit_enabled {
            if let Some((edit_row, edit_col)) = self.editing_cell {
                if edit_row != row_ix || edit_col != col_ix {
                    self.commit_cell_edit(window, cx);
                }
            }
        }

        // 双击处理
        if e.click_count() == 2 {
            if edit_enabled && !is_row_number_col {
                self.start_editing(row_ix, col_ix, window, cx);
            }
            cx.emit(EditTableEvent::DoubleClickedCell(row_ix, col_ix));
            return;
        }

        // 单击处理 - 多选逻辑
        if is_row_number_col {
            // 点击行号列，选择整行
            self.set_selected_row(row_ix, cx);
            return;
        }

        let shift_pressed = e.modifiers().shift;
        let ctrl_pressed = e.modifiers().secondary();

        tracing::debug!(
            "on_cell_click: row={}, col={}, shift={}, ctrl/cmd={}, multi_select_enabled={}",
            row_ix,
            col_ix,
            shift_pressed,
            ctrl_pressed,
            multi_select_enabled
        );

        if multi_select_enabled && shift_pressed {
            // Shift+Click: 扩展选区
            self.extend_selection_to(row_ix, col_ix, cx);
        } else if multi_select_enabled && ctrl_pressed {
            // Ctrl/Cmd+Click: 添加到选区
            self.add_to_selection(row_ix, col_ix, cx);
        } else {
            // 单击编辑模式（类似 Navicat）：
            // 如果单元格已被选中，再次单击则进入编辑模式
            if edit_enabled
                && self.delegate.single_click_to_edit(cx)
                && self.selected_cell == Some((row_ix, col_ix))
                && self.editing_cell.is_none()
            {
                self.start_editing(row_ix, col_ix, window, cx);
                return;
            }

            // 普通点击: 选择单个单元格
            self.select_cell(row_ix, col_ix, cx);
        }
    }

    pub fn start_editing(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editing_cell == Some((row_ix, col_ix)) {
            return;
        }
        let delegate_col_ix = if self.delegate.row_number_enabled(cx) {
            col_ix.saturating_sub(1)
        } else {
            col_ix
        };

        let input = self
            .delegate
            .build_input(row_ix, delegate_col_ix, window, cx);
        if input.is_some() {
            self.editing_cell = Some((row_ix, col_ix));
            let (input, subscriptions) = input.unwrap();
            self.editing_input = Some(input);
            self._subscriptions = subscriptions;
            cx.emit(EditTableEvent::CellEditing(row_ix, col_ix));
            cx.notify();
        }
    }

    pub fn commit_cell_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((row_ix, col_ix)) = self.editing_cell {
            let delegate_col_ix = if self.delegate.row_number_enabled(cx) {
                col_ix.saturating_sub(1)
            } else {
                col_ix
            };

            let new_value = self
                .editing_input
                .as_ref()
                .map(|editor| editor.get_value(cx))
                .unwrap_or_default();

            let accepted =
                self.delegate
                    .on_cell_edited(row_ix, delegate_col_ix, new_value, window, cx);
            if accepted {
                cx.emit(EditTableEvent::CellEdited(row_ix, col_ix));
            }
            self.editing_cell = None;
            self.editing_input = None;
            self._subscriptions.clear();
            cx.notify();
        }
    }

    pub fn cancel_cell_edit(&mut self, cx: &mut Context<Self>) {
        if self.editing_cell.is_some() {
            self.editing_cell = None;
            self.editing_input = None;
            self._subscriptions.clear();
            cx.notify();
        }
    }

    pub fn editing_input(&self) -> Option<&CellEditor> {
        self.editing_input.as_ref()
    }

    pub fn editing_cell(&self) -> Option<(usize, usize)> {
        self.editing_cell
    }

    pub fn add_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let row_ix = self.delegate.on_row_added(window, cx);
        self.scroll_to_row(row_ix, cx);
        cx.emit(EditTableEvent::RowAdded);
        self.refresh(cx);
        cx.notify();
    }

    pub fn delete_row(&mut self, row_ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.delegate.on_row_deleted(row_ix, window, cx);
        cx.emit(EditTableEvent::RowDeleted(row_ix));
        if self.selected_row == Some(row_ix) {
            self.selected_row = None;
        }
        self.refresh(cx);
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

    pub(super) fn action_confirm(
        &mut self,
        _: &Confirm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editing_cell.is_some() {
            self.commit_cell_edit(window, cx);
        }
    }

    pub(super) fn action_cancel(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        if self.editing_cell.is_some() {
            self.cancel_cell_edit(cx);
            return;
        }

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

        if self.has_cell_selection() {
            let first_col_ix = self.first_data_col_ix(cx);
            if let Some((row_ix, col_ix)) = self.current_cell_for_navigation() {
                let new_row_ix = if row_ix > 0 {
                    row_ix.saturating_sub(1)
                } else if self.loop_selection {
                    rows_count.saturating_sub(1)
                } else {
                    row_ix
                };
                self.select_cell_for_navigation(new_row_ix, col_ix.max(first_col_ix), cx);
            } else {
                self.select_cell_for_navigation(0, first_col_ix, cx);
            }
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

        if self.has_cell_selection() {
            let first_col_ix = self.first_data_col_ix(cx);
            if let Some((row_ix, col_ix)) = self.current_cell_for_navigation() {
                let new_row_ix = if row_ix < rows_count.saturating_sub(1) {
                    row_ix + 1
                } else if self.loop_selection {
                    0
                } else {
                    row_ix
                };
                self.select_cell_for_navigation(new_row_ix, col_ix.max(first_col_ix), cx);
            } else {
                self.select_cell_for_navigation(0, first_col_ix, cx);
            }
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

    pub(super) fn action_select_first_column(
        &mut self,
        _: &SelectFirst,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.last_data_col_ix(cx).is_none() {
            return;
        }
        let first_col_ix = self.first_data_col_ix(cx);
        let rows_count = self.delegate.rows_count(cx);

        if self.has_cell_selection() {
            if rows_count == 0 {
                return;
            }

            if let Some((row_ix, _)) = self.current_cell_for_navigation() {
                self.select_cell_for_navigation(row_ix, first_col_ix, cx);
            } else {
                self.select_cell_for_navigation(0, first_col_ix, cx);
            }
            return;
        }

        self.set_selected_col(first_col_ix, cx);
    }

    pub(super) fn action_select_last_column(
        &mut self,
        _: &SelectLast,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(last_col_ix) = self.last_data_col_ix(cx) else {
            return;
        };
        let rows_count = self.delegate.rows_count(cx);

        if self.has_cell_selection() {
            if rows_count == 0 {
                return;
            }

            if let Some((row_ix, _)) = self.current_cell_for_navigation() {
                self.select_cell_for_navigation(row_ix, last_col_ix, cx);
            } else {
                self.select_cell_for_navigation(0, last_col_ix, cx);
            }
            return;
        }

        self.set_selected_col(last_col_ix, cx);
    }

    pub(super) fn action_select_page_up(
        &mut self,
        _: &SelectPageUp,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rows_count = self.delegate.rows_count(cx);
        if rows_count < 1 {
            return;
        }

        let step = self.page_item_count();
        if self.has_cell_selection() {
            let first_col_ix = self.first_data_col_ix(cx);
            if let Some((row_ix, col_ix)) = self.current_cell_for_navigation() {
                let target_row_ix = row_ix.saturating_sub(step);
                self.select_cell_for_navigation(target_row_ix, col_ix.max(first_col_ix), cx);
            } else {
                self.select_cell_for_navigation(0, first_col_ix, cx);
            }
            return;
        }

        let current_row_ix = self.selected_row.unwrap_or(0);
        let target_row_ix = current_row_ix.saturating_sub(step);
        self.set_selected_row(target_row_ix, cx);
    }

    pub(super) fn action_select_page_down(
        &mut self,
        _: &SelectPageDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rows_count = self.delegate.rows_count(cx);
        if rows_count < 1 {
            return;
        }

        let step = self.page_item_count();
        if self.has_cell_selection() {
            let first_col_ix = self.first_data_col_ix(cx);
            if let Some((row_ix, col_ix)) = self.current_cell_for_navigation() {
                let max_row_ix = rows_count.saturating_sub(1);
                let target_row_ix = (row_ix + step).min(max_row_ix);
                self.select_cell_for_navigation(target_row_ix, col_ix.max(first_col_ix), cx);
            } else {
                self.select_cell_for_navigation(0, first_col_ix, cx);
            }
            return;
        }

        let current_row_ix = self.selected_row.unwrap_or(0);
        let max_row_ix = rows_count.saturating_sub(1);
        let target_row_ix = (current_row_ix + step).min(max_row_ix);
        self.set_selected_row(target_row_ix, cx);
    }

    pub(super) fn action_select_prev_col(
        &mut self,
        _: &SelectPrevColumn,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.has_cell_selection() {
            self.move_to_prev_cell(cx);
            return;
        }

        let Some(last_col_ix) = self.last_data_col_ix(cx) else {
            return;
        };
        let first_col_ix = self.first_data_col_ix(cx);
        let mut selected_col_ix = self
            .selected_col
            .unwrap_or(first_col_ix)
            .clamp(first_col_ix, last_col_ix);
        if selected_col_ix > first_col_ix {
            selected_col_ix = selected_col_ix.saturating_sub(1);
        } else if self.loop_selection {
            selected_col_ix = last_col_ix;
        }
        self.set_selected_col(selected_col_ix, cx);
    }

    pub(super) fn action_select_next_col(
        &mut self,
        _: &SelectNextColumn,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.has_cell_selection() {
            self.move_to_next_cell(cx);
            return;
        }

        let Some(last_col_ix) = self.last_data_col_ix(cx) else {
            return;
        };
        let first_col_ix = self.first_data_col_ix(cx);
        let mut selected_col_ix = self
            .selected_col
            .unwrap_or(first_col_ix)
            .clamp(first_col_ix, last_col_ix);
        if selected_col_ix < last_col_ix {
            selected_col_ix += 1;
        } else if self.loop_selection {
            selected_col_ix = first_col_ix;
        }

        self.set_selected_col(selected_col_ix, cx);
    }

    pub(super) fn action_editing_select_prev_col(
        &mut self,
        _: &OutdentInline,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit_and_move_to_prev_cell(window, cx);
    }

    pub(super) fn action_editing_select_next_col(
        &mut self,
        _: &IndentInline,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit_and_move_to_next_cell(window, cx);
    }

    // ==================== 复制/粘贴/全选 Actions ====================

    /// 获取选中区域的数据（供业务层使用）
    pub fn get_selection_data(&self, cx: &Context<Self>) -> Option<Vec<Vec<String>>> {
        if self.selection.is_empty() {
            return None;
        }

        let Some(range) = self.selection.first_range() else {
            return None;
        };

        let row_number_offset = if self.delegate.row_number_enabled(cx) {
            1
        } else {
            0
        };
        let ((min_row, min_col), (max_row, max_col)) = range.normalized();

        let mut data: Vec<Vec<String>> = Vec::new();
        for row in min_row..=max_row {
            let mut row_data: Vec<String> = Vec::new();
            for col in min_col..=max_col {
                let delegate_col = col.saturating_sub(row_number_offset);
                let value = self.delegate.get_cell_value(row, delegate_col, cx);
                row_data.push(value);
            }
            data.push(row_data);
        }

        if data.is_empty() { None } else { Some(data) }
    }

    /// 获取选中区域的列名（供业务层使用）
    pub fn get_selection_columns(&self, cx: &Context<Self>) -> Vec<SharedString> {
        let Some(range) = self.selection.first_range() else {
            return Vec::new();
        };

        let row_number_offset = if self.delegate.row_number_enabled(cx) {
            1
        } else {
            0
        };
        let ((_, min_col), (_, max_col)) = range.normalized();

        (min_col..=max_col)
            .map(|col| {
                let delegate_col = col.saturating_sub(row_number_offset);
                self.delegate.get_column_name(delegate_col, cx)
            })
            .collect()
    }

    /// 复制选中单元格 (Ctrl+C / Cmd+C)
    pub(super) fn action_copy(&mut self, _: &Copy, window: &mut Window, cx: &mut Context<Self>) {
        if self.selection.is_empty() {
            return;
        }

        let Some(range) = self.selection.first_range() else {
            return;
        };

        let row_number_offset = if self.delegate.row_number_enabled(cx) {
            1
        } else {
            0
        };
        let ((min_row, min_col), (max_row, max_col)) = range.normalized();

        // 收集选中单元格的值
        let mut data: Vec<Vec<String>> = Vec::new();
        for row in min_row..=max_row {
            let mut row_data: Vec<String> = Vec::new();
            for col in min_col..=max_col {
                // 转换为 delegate 的列索引（去除行号列偏移）
                let delegate_col = col.saturating_sub(row_number_offset);
                let value = self.delegate.get_cell_value(row, delegate_col, cx);
                row_data.push(value);
            }
            data.push(row_data);
        }

        if data.is_empty() {
            return;
        }

        // 转换为 TSV 格式（Tab 分隔，与 Excel 兼容）
        let text = data
            .iter()
            .map(|row| row.join("\t"))
            .collect::<Vec<_>>()
            .join("\n");

        // 写入剪贴板
        cx.write_to_clipboard(ClipboardItem::new_string(text));

        // 通知 delegate
        self.delegate.on_copy(data.clone(), window, cx);

        // 发射事件
        cx.emit(EditTableEvent::CopyData(data));
    }

    /// 粘贴内容 (Ctrl+V / Cmd+V)
    pub(super) fn action_paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        // 获取活动单元格作为粘贴起点
        let Some(start) = self.selection.active else {
            return;
        };

        // 从剪贴板读取
        let Some(clipboard) = cx.read_from_clipboard() else {
            return;
        };
        let Some(text) = clipboard.text() else {
            return;
        };

        // 解析 TSV 数据
        let data: Vec<Vec<String>> = text
            .lines()
            .map(|line| line.split('\t').map(|s| s.to_string()).collect())
            .collect();

        if data.is_empty() {
            return;
        }

        let row_number_offset = if self.delegate.row_number_enabled(cx) {
            1
        } else {
            0
        };

        // 构建变更列表
        let mut changes: Vec<(usize, usize, String)> = Vec::new();

        // 如果剪贴板是单值且有多选区域，则将该值填充到所有选中的单元格
        let is_single_value =
            data.len() == 1 && data[0].len() == 1 && self.selection.cell_count() > 1;

        if is_single_value {
            let value = &data[0][0];
            for (row, col) in self.selection.all_cells() {
                let delegate_col = col.saturating_sub(row_number_offset);
                changes.push((row, delegate_col, value.clone()));
            }
        } else {
            for (row_offset, row_data) in data.iter().enumerate() {
                for (col_offset, value) in row_data.iter().enumerate() {
                    let target_row = start.0 + row_offset;
                    let target_col = start.1 + col_offset;
                    // 转换为 delegate 的列索引
                    let delegate_col = target_col.saturating_sub(row_number_offset);
                    changes.push((target_row, delegate_col, value.clone()));
                }
            }
        }

        // 通过 delegate 应用变更
        if self.delegate.set_cell_values(changes, window, cx) {
            // 通知 delegate
            self.delegate.on_paste(data.clone(), start, window, cx);

            // 发射事件
            cx.emit(EditTableEvent::PasteData { data, start });

            // 刷新表格
            self.refresh(cx);
        }
    }

    /// 全选 (Ctrl+A / Cmd+A)
    pub(super) fn action_select_all(
        &mut self,
        _: &SelectAll,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.delegate.multi_select_enabled(cx) {
            return;
        }
        self.select_all_cells(cx);
    }

    fn scroll_table_by_col_resizing(
        &mut self,
        mouse_position: Point<Pixels>,
        col_group: &ColGroup,
    ) {
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
        if changed_width > px(-1.0) && changed_width < px(1.0) {
            return;
        }
        col_group.width = new_width.min(MAX_WIDTH);

        cx.notify();
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

        let delegate_col_ix = if self.delegate.row_number_enabled(cx) {
            col_ix.saturating_sub(1)
        } else {
            col_ix
        };

        self.delegate_mut()
            .perform_sort(delegate_col_ix, sort, window, cx);

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

        let row_number_offset = if self.delegate.row_number_enabled(cx) {
            1
        } else {
            0
        };
        if row_number_offset > 0 && (col_ix == 0 || to_ix == 0) {
            return;
        }

        let delegate_col_ix = col_ix.saturating_sub(row_number_offset);
        let delegate_to_ix = to_ix.saturating_sub(row_number_offset);

        self.delegate
            .move_column(delegate_col_ix, delegate_to_ix, window, cx);
        let col_group = self.col_groups.remove(col_ix);
        self.col_groups.insert(to_ix, col_group);

        cx.emit(EditTableEvent::MoveColumn(col_ix, to_ix));
        cx.notify();
    }

    fn load_more_if_need(
        &mut self,
        rows_count: usize,
        visible_end: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let threshold = self.delegate.load_more_threshold();
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
        if visible_range.len() <= 1 {
            return;
        }

        if axis == Axis::Vertical {
            if self.visible_range.rows == visible_range {
                return;
            }
            // 清理不可见行的单元格边界缓存
            self.cell_bounds
                .retain(|(row, _), _| visible_range.contains(row));
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

    fn render_cell(
        &self,
        col_ix: usize,
        row_ix: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let Some(col_group) = self.col_groups.get(col_ix) else {
            return div().id("empty-cell");
        };

        // 检查是否在多选区内（行选中时不显示单元格选区效果）
        let is_cell_selection_mode = self.selection_state == SelectionState::Cell;
        let is_in_selection = is_cell_selection_mode
            && row_ix
                .map(|r| self.selection.contains(r, col_ix))
                .unwrap_or(false);

        // 检查是否是活动单元格
        let is_active_cell = is_cell_selection_mode
            && row_ix
                .map(|r| self.selection.active == Some((r, col_ix)))
                .unwrap_or(false);

        // 是否为多选状态（选区包含多个单元格）
        let is_multi_selection = is_cell_selection_mode
            && (self.selection.ranges.len() > 1
                || self.selection.ranges.iter().any(|r| !r.is_single()));

        // 计算选区边框（只在选区边界显示，且仅限单元格选择模式）
        let (border_top, border_bottom, border_left, border_right) =
            if is_in_selection && row_ix.is_some() {
                let r = row_ix.unwrap();
                let top = r == 0 || !self.selection.contains(r - 1, col_ix);
                let bottom = !self.selection.contains(r + 1, col_ix);
                let left = col_ix == 0 || !self.selection.contains(r, col_ix - 1);
                let right = !self.selection.contains(r, col_ix + 1);
                (top, bottom, left, right)
            } else {
                (false, false, false, false)
            };

        // 旧的单选逻辑（向后兼容）
        let is_select_cell = match self.selected_cell {
            None => false,
            Some(cell) => row_ix.is_some() && row_ix.unwrap() == cell.0 && col_ix == cell.1,
        };

        let col_width = col_group.width;
        let col_padding = col_group.column.paddings;

        let is_row_number_col = self.delegate.row_number_enabled(cx) && col_ix == 0;
        let is_modified = if is_row_number_col {
            false
        } else {
            row_ix
                .map(|r| {
                    let delegate_col_ix = if self.delegate.row_number_enabled(cx) {
                        col_ix - 1
                    } else {
                        col_ix
                    };
                    self.delegate.is_cell_modified(r, delegate_col_ix, cx)
                })
                .unwrap_or(false)
        };

        let cell_id = match row_ix {
            Some(r) => ("cell", r * 10000 + col_ix),
            None => ("cell-header", col_ix),
        };

        let is_editing = row_ix.is_some() && self.editing_cell == Some((row_ix.unwrap(), col_ix));
        let selection_border_color = cx.theme().table_active_border;

        let mut cell = div()
            .id(cell_id)
            .w(col_width)
            .h_full()
            .flex_shrink_0()
            .overflow_hidden()
            .whitespace_nowrap()
            // 选区内的所有单元格使用背景色
            .when(is_in_selection && !is_editing, |this| {
                this.bg(cx.theme().table_active)
            })
            // 选区边框 - 上边界
            .when(border_top, |this| {
                this.border_t_2().border_color(selection_border_color)
            })
            // 选区边框 - 下边界
            .when(border_bottom, |this| {
                this.border_b_2().border_color(selection_border_color)
            })
            // 选区边框 - 左边界
            .when(border_left, |this| {
                this.border_l_2().border_color(selection_border_color)
            })
            // 选区边框 - 右边界
            .when(border_right, |this| {
                this.border_r_2().border_color(selection_border_color)
            })
            // 活动单元格额外添加完整边框（仅在单选时显示）
            .when(
                (is_active_cell || is_select_cell) && !is_editing && !is_multi_selection,
                |this| this.border_2().border_color(selection_border_color),
            )
            // 编辑状态的单元格
            .when(is_editing, |this| {
                this.bg(cx.theme().background)
                    .border_2()
                    .border_color(cx.theme().ring)
            })
            .when(is_modified && !is_editing && !is_in_selection, |this| {
                this.bg(cx.theme().warning.opacity(0.15))
            });

        if is_editing {
            if let Some(editor) = &self.editing_input {
                cell = cell.child(editor.render(window, cx));
            }
        } else {
            cell = cell.table_cell_size(self.options.size);
            cell = match col_padding {
                Some(padding) => cell
                    .pl(padding.left)
                    .pr(padding.right)
                    .pt(padding.top)
                    .pb(padding.bottom),
                None => cell,
            };
        }

        cell
    }

    /// 渲染带有交互事件的单元格（拖选、点击等）
    fn render_interactive_cell(
        &mut self,
        col_ix: usize,
        row_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let entity_id = cx.entity_id();
        let is_editing = self.editing_cell == Some((row_ix, col_ix));
        let is_row_number_col = self.delegate.row_number_enabled(cx) && col_ix == 0;
        let view = cx.entity().clone();

        let cell = self
            .render_cell(col_ix, Some(row_ix), window, cx)
            // 点击事件
            .on_click(cx.listener(move |this, e, window, cx| {
                this.on_cell_click(e, row_ix, col_ix, window, cx);
            }))
            // 鼠标按下准备拖选
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, e: &MouseDownEvent, _window, cx| {
                    if is_row_number_col {
                        return;
                    }
                    let add_to_selection = e.modifiers.secondary();
                    this.prepare_drag_selection(row_ix, col_ix, add_to_selection, cx);
                }),
            )
            // 拖动事件 - 用于初始化拖动，立即开始选区
            .on_drag(DragSelectCell { entity_id }, {
                let view = view.clone();
                move |drag, _, _, cx| {
                    cx.stop_propagation();
                    // 开始拖动时立即开始选区
                    view.update(cx, |this, cx| {
                        if let Some((start_row, start_col, add_to_selection)) =
                            this.drag_start.take()
                        {
                            this.is_selecting = true;
                            this.selection
                                .start_drag((start_row, start_col), add_to_selection);
                            this.sync_legacy_selection(cx);
                            cx.notify();
                        }
                    });
                    cx.new(|_| drag.clone())
                }
            })
            // 拖动移动事件 - 更新选区（通过命中测试）
            .on_drag_move(cx.listener(
                move |this, e: &DragMoveEvent<DragSelectCell>, _window, cx| {
                    let drag = e.drag(cx);
                    if drag.entity_id != cx.entity_id() {
                        return;
                    }
                    // 行号列不参与拖选更新
                    if is_row_number_col {
                        return;
                    }
                    // 检查鼠标位置是否在当前单元格内
                    if let Some(bounds) = this.cell_bounds.get(&(row_ix, col_ix)) {
                        if bounds.contains(&e.event.position) {
                            // 更新拖选到当前单元格
                            this.update_drag_selection(row_ix, col_ix, cx);
                        }
                    }
                },
            ))
            // 鼠标释放结束拖选
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _, _window, cx| {
                    this.end_drag_selection(cx);
                }),
            )
            // 使用 canvas 追踪单元格边界
            .child({
                canvas(
                    move |bounds, _, cx| {
                        view.update(cx, |state, _| {
                            state.cell_bounds.insert((row_ix, col_ix), bounds);
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            });

        if is_editing {
            cell
        } else {
            cell.child(self.measure_render_td(row_ix, col_ix, window, cx))
        }
    }

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
                    cx.emit(EditTableEvent::ColumnWidthsChanged(new_widths));
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    fn render_sort_icon(
        &self,
        col_ix: usize,
        col_group: &ColGroup,
        _window: &mut Window,
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

    fn render_filter_icon(
        &self,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        if !self.col_filterable || !self.delegate.column_filter_enabled(cx) {
            return None;
        }

        let is_filtered = self.filter_state.is_column_filtered(col_ix);
        let is_open = self.active_filter_col == Some(col_ix);
        let table_entity = cx.entity().clone();

        use gpui_component::{
            Sizable, Size, button::Button, button::ButtonVariants, popover::Popover,
        };

        let filter_content = if is_open {
            Some(self.render_filter_panel_content(col_ix, _window, cx))
        } else {
            None
        };

        Some(
            Popover::new(("filter-popover", col_ix))
                .trigger(
                    Button::new(("filter-btn", col_ix))
                        .icon(IconName::Filter)
                        .ghost()
                        .with_size(Size::XSmall)
                        .when(is_filtered, |this| this.primary()),
                )
                .open(is_open)
                .on_open_change({
                    let entity = table_entity.clone();
                    move |open, window, cx| {
                        entity.update(cx, |table, cx| {
                            if *open {
                                table.open_filter_panel(col_ix, window, cx);
                            } else {
                                table.close_filter_panel(cx);
                            }
                        });
                    }
                })
                .p_0()
                .children(filter_content),
        )
    }

    fn render_filter_panel_content(
        &self,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        use gpui_component::{Sizable, Size, button::Button, button::ButtonVariants};

        let table_entity = cx.entity().clone();
        let selected_count = self.filter_panel_selected_count(cx);
        let total_count = self.filter_panel_total_count(cx);

        let filter_list = match &self.filter_list {
            Some(list) => list.clone(),
            None => return div().into_any_element(),
        };

        v_flex()
            .w(px(280.))
            .max_h(px(400.))
            .gap_2()
            .p_2()
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Selected {} / {}", selected_count, total_count)),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("filter-select-all")
                                    .label("Select All")
                                    .ghost()
                                    .with_size(Size::XSmall)
                                    .on_click({
                                        let entity = table_entity.clone();
                                        move |_, window, cx| {
                                            entity.update(cx, |table, cx| {
                                                table.filter_panel_select_all_realtime(
                                                    col_ix, window, cx,
                                                );
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Button::new("filter-deselect-all")
                                    .label("Clear")
                                    .ghost()
                                    .with_size(Size::XSmall)
                                    .on_click({
                                        let entity = table_entity.clone();
                                        move |_, window, cx| {
                                            entity.update(cx, |table, cx| {
                                                table.filter_panel_deselect_all_realtime(
                                                    col_ix, window, cx,
                                                );
                                            });
                                        }
                                    }),
                            ),
                    ),
            )
            .child(div().h(px(1.)).w_full().bg(cx.theme().border))
            .child(
                List::new(&filter_list)
                    .max_h(px(200.))
                    .p(px(8.))
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius),
            )
            .into_any_element()
    }

    fn render_th(&mut self, col_ix: usize, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let entity_id = cx.entity_id();
        let col_group = self.col_groups.get(col_ix).expect("BUG: invalid col index");

        let is_row_number_col = self.delegate.row_number_enabled(cx) && col_ix == 0;
        let movable = self.col_movable && col_group.column.movable && !is_row_number_col;
        let paddings = col_group.column.paddings;
        let name = col_group.column.name.clone();
        let delegate_col_ix = if self.delegate.row_number_enabled(cx) && col_ix > 0 {
            col_ix - 1
        } else {
            col_ix
        };

        h_flex()
            .h_full()
            .child(
                self.render_cell(col_ix, None, window, cx)
                    .id(("col-header", col_ix))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.on_col_head_click(col_ix, window, cx);
                    }))
                    .child(
                        h_flex()
                            .size_full()
                            .justify_between()
                            .items_center()
                            .child(if is_row_number_col {
                                div()
                                    .size_full()
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .child(col_group.column.name.clone())
                                    .into_any_element()
                            } else {
                                self.delegate
                                    .render_th(delegate_col_ix, window, cx)
                                    .into_any_element()
                            })
                            .when_some(paddings, |this, paddings| {
                                let offset_pr =
                                    self.options.size.table_cell_padding().right - paddings.right;
                                this.pr(offset_pr.max(px(0.)))
                            })
                            .when(!is_row_number_col, |this| {
                                this.children(self.render_filter_icon(delegate_col_ix, window, cx))
                            })
                            .when(!is_row_number_col, |this| {
                                this.children(self.render_sort_icon(col_ix, &col_group, window, cx))
                            }),
                    )
                    .when(movable, |this| {
                        this.on_drag(
                            DragColumn {
                                entity_id,
                                col_ix: delegate_col_ix,
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
                                if drag.entity_id != cx.entity_id() {
                                    return;
                                }

                                table.move_column(drag.col_ix, delegate_col_ix, window, cx);
                            },
                        ))
                    }),
            )
            .child(self.render_resize_handle(col_ix, window, cx))
            .child({
                let view = cx.entity().clone();
                canvas(
                    move |bounds, _, cx| {
                        view.update(cx, |r, _| r.col_groups[col_ix].bounds = bounds)
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
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
                                    view.update(cx, |r, _| r.fixed_head_cols_bounds = bounds)
                                },
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .size_full(),
                        ),
                )
            })
            .child(
                h_flex()
                    .id("table-head")
                    .size_full()
                    .overflow_hidden()
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
        _col_sizes: Rc<Vec<gpui::Size<Pixels>>>,
        columns_count: usize,
        is_filled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let horizontal_scroll_handle = self.horizontal_scroll_handle.clone();
        let is_stripe_row = self.options.stripe && row_ix % 2 != 0;
        let is_selected = self.selected_row == Some(row_ix);
        let is_row_deleted = self.delegate.is_row_deleted(row_ix, cx);
        let is_row_added = self.delegate.is_row_added(row_ix, cx);
        let _view = cx.entity().clone();
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
                .when(is_stripe_row && !is_row_deleted, |this| {
                    this.bg(cx.theme().table_even)
                })
                .when(is_row_deleted, |this| {
                    this.opacity(0.5)
                        .line_through()
                        .bg(cx.theme().warning.opacity(0.1))
                })
                .when(is_row_added && !is_row_deleted, |this| {
                    this.bg(cx.theme().success.opacity(0.1))
                })
                .refine_style(&style)
                .hover(|this| {
                    if is_row_deleted {
                        this
                    } else if is_selected || self.right_clicked_row == Some(row_ix) {
                        this
                    } else {
                        this.bg(cx.theme().table_hover)
                    }
                })
                .when(left_columns_count > 0, |this| {
                    this.child(
                        h_flex()
                            .relative()
                            .h_full()
                            .children({
                                let mut items = Vec::with_capacity(left_columns_count);

                                (0..left_columns_count).for_each(|col_ix| {
                                    items.push(self.render_col_wrap(col_ix, window, cx).child(
                                        self.render_interactive_cell(col_ix, row_ix, window, cx),
                                    ));
                                });

                                items
                            })
                            .child(
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
                        .id(("table-row-cols", row_ix))
                        .flex_1()
                        .h_full()
                        .overflow_hidden()
                        .relative()
                        .track_scroll(&horizontal_scroll_handle)
                        .children({
                            let columns_count = self.col_groups.len();
                            let mut items = Vec::with_capacity(columns_count - left_columns_count);

                            (left_columns_count..columns_count).for_each(|col_ix| {
                                let el = self.render_col_wrap(col_ix, window, cx).child(
                                    self.render_interactive_cell(col_ix, row_ix, window, cx),
                                );
                                items.push(el);
                            });

                            items
                        })
                        .child(self.delegate.render_last_empty_col(window, cx)),
                )
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
                                    .border_2()
                                    .border_color(cx.theme().table_active_border),
                            )
                        },
                    )
                })
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
        } else {
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
                        .child(self.render_cell(col_ix, Some(row_ix), window, cx))
                }))
                .child(self.delegate.render_last_empty_col(window, cx))
        }
    }

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
        let is_row_number_col = self.delegate.row_number_enabled(cx) && col_ix == 0;

        if is_row_number_col {
            return div()
                .id(ElementId::Name(format!("row-number-{}", row_ix).into()))
                .size_full()
                .flex()
                .items_center()
                .justify_end()
                .text_color(cx.theme().muted_foreground)
                .child((row_ix + 1).to_string())
                .on_click(cx.listener(move |this, e, window, cx| {
                    this.on_row_left_click(e, row_ix, window, cx);
                }))
                .into_any_element();
        }

        let delegate_col_ix = if self.delegate.row_number_enabled(cx) {
            col_ix - 1
        } else {
            col_ix
        };

        self.delegate
            .render_td(row_ix, delegate_col_ix, window, cx)
            .into_any_element()
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
                .w(SCROLLBAR_WIDTH)
                .child(Scrollbar::vertical(&self.vertical_scroll_handle)),
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
            .h(SCROLLBAR_WIDTH)
            .child(Scrollbar::horizontal(&self.horizontal_scroll_handle))
    }
}

impl<D> Focusable for EditTableState<D>
where
    D: EditTableDelegate,
{
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl<D> EventEmitter<EditTableEvent> for EditTableState<D> where D: EditTableDelegate {}

impl<D> Render for EditTableState<D>
where
    D: EditTableDelegate,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .key_context("EditTable")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::action_copy))
            .on_action(cx.listener(Self::action_paste))
            .on_action(cx.listener(Self::action_select_all))
            .on_action(cx.listener(Self::action_confirm))
            .on_action(cx.listener(Self::action_cancel))
            .on_action(cx.listener(Self::action_select_prev))
            .on_action(cx.listener(Self::action_select_next))
            .on_action(cx.listener(Self::action_select_prev_col))
            .on_action(cx.listener(Self::action_select_next_col))
            .on_action(cx.listener(Self::action_select_first_column))
            .on_action(cx.listener(Self::action_select_last_column))
            .on_action(cx.listener(Self::action_select_page_up))
            .on_action(cx.listener(Self::action_select_page_down))
            .on_action(cx.listener(Self::action_editing_select_prev_col))
            .on_action(cx.listener(Self::action_editing_select_next_col))
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
                        h_flex()
                            .id("table-body")
                            .flex_grow()
                            .size_full()
                            .when(self.options.scrollbar_visible.bottom, |this| {
                                this.pb(SCROLLBAR_WIDTH)
                            })
                            .child(
                                uniform_list(
                                    "table-uniform-list",
                                    render_rows_count,
                                    cx.processor(
                                        move |table, visible_range: Range<usize>, window, cx| {
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
                                                visible_range
                                                    .end
                                                    .saturating_sub(visible_range.start),
                                            );

                                            visible_range.for_each(|row_ix| {
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
            .on_scroll_wheel(cx.listener(Self::handle_scroll_wheel))
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
            .child(canvas(
                {
                    let state = cx.entity();
                    move |bounds, _, cx| state.update(cx, |state, _| state.bounds = bounds)
                },
                |_, _, _, _| {},
            ))
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
