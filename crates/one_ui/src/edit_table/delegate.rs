use std::{collections::HashSet, ops::Range};

use gpui::{div, px, AnyElement, App, Context, Div, Entity, InteractiveElement as _, IntoElement, ParentElement as _, SharedString, Stateful, StatefulInteractiveElement, Styled as _, Subscription, Window};
use gpui_component::{
    h_flex, input::{Input, InputState}, menu::PopupMenu, ActiveTheme as _, Icon,
    IconName,
    Size,
};
use gpui_component::date_picker::{DatePicker, DatePickerState};
use gpui_component::datetime_picker::{DateTimePicker, DateTimePickerState};
use gpui_component::time_picker::{TimePicker, TimePickerState};
use crate::edit_table::{loading::Loading, Column, ColumnSort, EditTableState};
use crate::edit_table::filter_panel::FilterValue;

pub enum CellEditor {
    Input(Entity<InputState>),
    DatePicker(Entity<DatePickerState>),
    DateTimePicker(Entity<DateTimePickerState>),
    TimePicker(Entity<TimePickerState>),
    DatePickerInput {
        input: Entity<InputState>,
        picker: Entity<DatePickerState>,
    },
    DateTimePickerInput {
        input: Entity<InputState>,
        picker: Entity<DateTimePickerState>,
    },
    TimePickerInput {
        input: Entity<InputState>,
        picker: Entity<TimePickerState>,
    },
}

impl CellEditor {
    pub fn render(&self, _window: &mut Window, cx: &mut App) -> AnyElement {
        match self {
            CellEditor::Input(input) => Input::new(input)
                .w_full()
                .h_full()
                .text_base()
                .appearance(false)
                .into_any_element(),
            CellEditor::DatePicker(picker) => DatePicker::new(picker)
                .w_full()
                .appearance(false)
                .cleanable(true)
                .into_any_element(),
            CellEditor::DateTimePicker(picker) => DateTimePicker::new(picker)
                .w_full()
                .appearance(false)
                .cleanable(true)
                .into_any_element(),
            CellEditor::TimePicker(picker) => TimePicker::new(picker)
                .w_full()
                .appearance(false)
                .cleanable(true)
                .into_any_element(),
            CellEditor::DatePickerInput { input, picker } => {
                let input_handle = input.clone();
                let picker_handle = picker.clone();
                h_flex()
                    .w_full()
                    .h_full()
                    .items_center()
                    .gap_1()
                    .child(
                        Input::new(input)
                            .flex_1()
                            .h_full()
                            .text_base()
                            .appearance(false),
                    )
                    .child(
                        div()
                            .id(SharedString::from("date-picker-popup"))
                            .px_1()
                            .cursor_pointer()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .hover(|this| this.text_color(cx.theme().foreground))
                            .on_click(move |_, window, cx| {
                                cx.stop_propagation();
                                let picker_handle = picker_handle.clone();
                                let input_handle = input_handle.clone();
                                window.defer(cx, move |window, cx| {
                                    let is_open = picker_handle.read(cx).is_open();
                                    picker_handle.update(cx, |state, cx| {
                                        state.set_open(!is_open, cx);
                                    });
                                    input_handle.update(cx, |state, cx| {
                                        state.focus(window, cx);
                                    });
                                });
                            })
                            .child("..."),
                    )
                    .child(
                        div()
                            .w_0()
                            .h(px(0.))
                            .overflow_hidden()
                            .child(
                                DatePicker::new(picker)
                                    .appearance(false)
                                    .cleanable(true),
                            ),
                    )
                    .into_any_element()
            }
            CellEditor::DateTimePickerInput { input, picker } => {
                let input_handle = input.clone();
                let picker_handle = picker.clone();
                h_flex()
                    .w_full()
                    .h_full()
                    .items_center()
                    .gap_1()
                    .child(
                        Input::new(input)
                            .flex_1()
                            .h_full()
                            .text_base()
                            .appearance(false),
                    )
                    .child(
                        div()
                            .id(SharedString::from("date-time-picker-popup"))
                            .px_1()
                            .cursor_pointer()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .hover(|this| this.text_color(cx.theme().foreground))
                            .on_click(move |_, window, cx| {
                                cx.stop_propagation();
                                let picker_handle = picker_handle.clone();
                                let input_handle = input_handle.clone();
                                window.defer(cx, move |window, cx| {
                                    picker_handle.update(cx, |state, cx| {
                                        state.set_open(true, window, cx);
                                    });
                                    input_handle.update(cx, |state, cx| {
                                        state.focus(window, cx);
                                    });
                                });
                            })
                            .child("..."),
                    )
                    .child(
                        div()
                            .w_0()
                            .h(px(0.))
                            .overflow_hidden()
                            .child(
                                DateTimePicker::new(picker)
                                    .appearance(false)
                                    .cleanable(true),
                            ),
                    )
                    .into_any_element()
            }
            CellEditor::TimePickerInput { input, picker } => {
                let input_handle = input.clone();
                let picker_handle = picker.clone();
                h_flex()
                    .w_full()
                    .h_full()
                    .items_center()
                    .gap_1()
                    .child(
                        Input::new(input)
                            .flex_1()
                            .h_full()
                            .text_base()
                            .appearance(false),
                    )
                    .child(
                        div()
                            .id(SharedString::from("time-picker-popup"))
                            .px_1()
                            .cursor_pointer()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .hover(|this| this.text_color(cx.theme().foreground))
                            .on_click(move |_, window, cx| {
                                cx.stop_propagation();
                                let picker_handle = picker_handle.clone();
                                let input_handle = input_handle.clone();
                                window.defer(cx, move |window, cx| {
                                    picker_handle.update(cx, |state, cx| {
                                        state.set_open(true, window, cx);
                                    });
                                    input_handle.update(cx, |state, cx| {
                                        state.focus(window, cx);
                                    });
                                });
                            })
                            .child("..."),
                    )
                    .child(
                        div()
                            .w_0()
                            .h(px(0.))
                            .overflow_hidden()
                            .child(
                                TimePicker::new(picker)
                                    .appearance(false)
                                    .cleanable(true),
                            ),
                    )
                    .into_any_element()
            }
        }
    }

    pub fn get_value(&self, cx: &App) -> String {
        match self {
            CellEditor::Input(input) => input.read(cx).text().to_string(),
            CellEditor::DatePicker(picker) => picker
                .read(cx)
                .date()
                .format("%Y-%m-%d")
                .unwrap_or_default()
                .to_string(),
            CellEditor::DateTimePicker(picker) => picker
                .read(cx)
                .datetime()
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default(),
            CellEditor::TimePicker(picker) => picker
                .read(cx)
                .time()
                .map(|t| t.format("%H:%M:%S").to_string())
                .unwrap_or_default(),
            CellEditor::DatePickerInput { input, .. } => input.read(cx).text().to_string(),
            CellEditor::DateTimePickerInput { input, .. } => input.read(cx).text().to_string(),
            CellEditor::TimePickerInput { input, .. } => input.read(cx).text().to_string(),
        }
    }
}

#[allow(unused)]
pub trait EditTableDelegate: Sized + 'static {
    fn cell_edit_enabled(&self, cx: &App) -> bool {
        false
    }

    fn row_number_enabled(&self, cx: &App) -> bool {
        false
    }

    fn columns_count(&self, cx: &App) -> usize;

    fn rows_count(&self, cx: &App) -> usize;

    fn column(&self, col_ix: usize, cx: &App) -> Column;

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) {
    }

    fn render_header(
        &mut self,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> Stateful<Div> {
        div().id("header")
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .child(self.column(col_ix, cx).name.clone())
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> Stateful<Div> {
        div().id(("row", row_ix))
    }

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: PopupMenu,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> PopupMenu {
        menu
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> impl IntoElement;

    fn move_column(
        &mut self,
        col_ix: usize,
        to_ix: usize,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) {
    }

    fn render_empty(
        &mut self,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> impl IntoElement {
        h_flex()
            .size_full()
            .justify_center()
            .text_color(cx.theme().muted_foreground.opacity(0.6))
            .child(Icon::new(IconName::Inbox).size_12())
            .into_any_element()
    }

    fn loading(&self, cx: &App) -> bool {
        false
    }

    fn render_loading(
        &mut self,
        size: Size,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> impl IntoElement {
        Loading::new().size(size)
    }

    fn has_more(&self, cx: &App) -> bool {
        false
    }

    fn load_more_threshold(&self) -> usize {
        20
    }

    fn load_more(&mut self, window: &mut Window, cx: &mut Context<EditTableState<Self>>) {}

    fn render_last_empty_col(
        &mut self,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> impl IntoElement {
        h_flex().w_3().h_full().flex_shrink_0()
    }

    fn visible_rows_changed(
        &mut self,
        visible_range: Range<usize>,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) {
    }

    fn visible_columns_changed(
        &mut self,
        visible_range: Range<usize>,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) {
    }

    fn build_input(
        &self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> Option<(CellEditor, Vec<Subscription>)> {
        None
    }

    fn on_cell_edited(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        new_value: String,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> bool {
        true
    }

    fn is_cell_modified(&self, row_ix: usize, col_ix: usize, cx: &App) -> bool {
        false
    }

    fn is_row_deleted(&self, row_ix: usize, cx: &App) -> bool {
        false
    }

    fn is_row_added(&self, row_ix: usize, cx: &App) -> bool {
        false
    }

    fn on_row_added(&mut self, window: &mut Window, cx: &mut Context<EditTableState<Self>>) -> usize {
        0
    }

    fn on_row_deleted(
        &mut self,
        row_ix: usize,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) {
    }

    fn column_filter_enabled(&self, cx: &App) -> bool {
        false
    }

    fn get_column_filter_values(&self, col_ix: usize, cx: &App) -> Vec<FilterValue> {
        vec![]
    }

    fn is_column_filtered(&self, col_ix: usize, cx: &App) -> bool {
        false
    }

    fn on_column_filter_changed(
        &mut self,
        col_ix: usize,
        selected_values: HashSet<String>,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) {
    }

    fn on_column_filter_cleared(
        &mut self,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) {
    }

    /// 是否支持多选
    fn multi_select_enabled(&self, _cx: &App) -> bool {
        true
    }

    /// 获取单元格的原始字符串值（用于复制）
    fn get_cell_value(&self, _row_ix: usize, _col_ix: usize, _cx: &App) -> String {
        String::new()
    }

    /// 批量设置单元格值（用于粘贴）
    /// 返回 true 表示成功，false 表示失败或不支持
    fn set_cell_values(
        &mut self,
        _changes: Vec<(usize, usize, String)>,
        _window: &mut Window,
        _cx: &mut Context<EditTableState<Self>>,
    ) -> bool {
        false
    }

    /// 复制选中单元格时的回调
    fn on_copy(
        &mut self,
        _data: Vec<Vec<String>>,
        _window: &mut Window,
        _cx: &mut Context<EditTableState<Self>>,
    ) {
    }

    /// 粘贴数据时的回调
    fn on_paste(
        &mut self,
        _data: Vec<Vec<String>>,
        _start: (usize, usize),
        _window: &mut Window,
        _cx: &mut Context<EditTableState<Self>>,
    ) {
    }

    /// 获取指定列的名称
    fn get_column_name(&self, col_ix: usize, cx: &App) -> SharedString {
        self.column(col_ix, cx).name.clone()
    }
}
