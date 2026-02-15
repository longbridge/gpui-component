use std::collections::HashMap;

use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Subscription, Window, prelude::*, px,
};
use gpui_component::form::h_form;
use gpui_component::{
    IndexPath, Sizable, Size,
    form::field,
    input::{Input, InputState},
    select::{Select, SelectEvent, SelectItem, SelectState},
    v_flex,
};

use crate::DatabaseFormEvent;
use db::plugin::DatabaseOperationRequest;

#[derive(Clone, Debug)]
pub struct EngineSelectItem {
    pub name: String,
    pub description: String,
}

impl EngineSelectItem {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

impl SelectItem for EngineSelectItem {
    type Value = String;

    fn title(&self) -> gpui::SharedString {
        format!("{} - {}", self.name, self.description).into()
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }
}

pub struct ClickHouseDatabaseForm {
    focus_handle: FocusHandle,
    name_input: Entity<InputState>,
    engine_select: Entity<SelectState<Vec<EngineSelectItem>>>,
    comment_input: Entity<InputState>,
    is_edit_mode: bool,
    _subscriptions: Vec<Subscription>,
}

impl ClickHouseDatabaseForm {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入数据库名称"));

        let engine_items = vec![
            EngineSelectItem::new("Atomic", "原子数据库引擎 (默认, ClickHouse 20.5+)"),
            EngineSelectItem::new("Ordinary", "普通数据库引擎"),
            EngineSelectItem::new("Memory", "内存数据库引擎"),
            EngineSelectItem::new("Lazy", "懒加载数据库引擎 (日志表)"),
            EngineSelectItem::new("MySQL", "MySQL 引擎 (连接外部 MySQL)"),
            EngineSelectItem::new("PostgreSQL", "PostgreSQL 引擎 (连接外部 PostgreSQL)"),
        ];

        let engine_select =
            cx.new(|cx| SelectState::new(engine_items, Some(IndexPath::new(0)), window, cx));

        let comment_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("数据库注释 (可选)"));

        let name_sub = cx.observe(&name_input, |this, _, cx| {
            this.trigger_form_changed(cx);
        });

        let engine_sub = cx.subscribe_in(
            &engine_select,
            window,
            |this, _select, _event: &SelectEvent<Vec<EngineSelectItem>>, _window, cx| {
                this.trigger_form_changed(cx);
            },
        );

        let comment_sub = cx.observe(&comment_input, |this, _, cx| {
            this.trigger_form_changed(cx);
        });

        Self {
            focus_handle,
            name_input,
            engine_select,
            comment_input,
            is_edit_mode: false,
            _subscriptions: vec![name_sub, engine_sub, comment_sub],
        }
    }

    pub fn new_for_edit(database_name: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut form = Self::new(window, cx);
        form.is_edit_mode = true;
        form.name_input.update(cx, |input, cx| {
            input.set_value(database_name.to_string(), window, cx);
        });
        form
    }

    fn build_request(&self, cx: &App) -> DatabaseOperationRequest {
        let mut field_values = HashMap::new();

        let db_name = self.name_input.read(cx).text().to_string();

        let engine = self
            .engine_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "Atomic".to_string());

        let comment = self.comment_input.read(cx).text().to_string();

        field_values.insert("name".to_string(), db_name.clone());
        field_values.insert("engine".to_string(), engine);
        if !comment.is_empty() {
            field_values.insert("comment".to_string(), comment);
        }

        DatabaseOperationRequest {
            database_name: db_name,
            field_values,
        }
    }

    fn trigger_form_changed(&mut self, cx: &mut Context<Self>) {
        let request = self.build_request(cx);
        cx.emit(DatabaseFormEvent::FormChanged(request));
    }
}

impl EventEmitter<DatabaseFormEvent> for ClickHouseDatabaseForm {}

impl Focusable for ClickHouseDatabaseForm {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ClickHouseDatabaseForm {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_4().p_4().size_full().child(
            h_form()
                .with_size(Size::Small)
                .columns(1)
                .label_width(px(100.))
                .child(
                    field()
                        .label("数据库名称")
                        .required(true)
                        .items_center()
                        .label_justify_end()
                        .child(
                            Input::new(&self.name_input)
                                .w_full()
                                .disabled(self.is_edit_mode),
                        ),
                )
                .child(
                    field()
                        .label("数据库引擎")
                        .items_center()
                        .label_justify_end()
                        .child(Select::new(&self.engine_select).w_full()),
                )
                .child(
                    field()
                        .label("注释")
                        .items_center()
                        .label_justify_end()
                        .child(Input::new(&self.comment_input).w_full()),
                ),
        )
    }
}
