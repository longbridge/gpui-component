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
pub struct CollationSelectItem {
    pub name: String,
    pub description: String,
}

impl CollationSelectItem {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

impl SelectItem for CollationSelectItem {
    type Value = String;

    fn title(&self) -> gpui::SharedString {
        format!("{} - {}", self.name, self.description).into()
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }
}

pub struct MsSqlDatabaseForm {
    focus_handle: FocusHandle,
    name_input: Entity<InputState>,
    collation_select: Entity<SelectState<Vec<CollationSelectItem>>>,
    is_edit_mode: bool,
    _subscriptions: Vec<Subscription>,
}

impl MsSqlDatabaseForm {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入数据库名称"));

        let collation_items = vec![
            CollationSelectItem::new(
                "SQL_Latin1_General_CP1_CI_AS",
                "Latin1 General, case-insensitive (默认)",
            ),
            CollationSelectItem::new(
                "SQL_Latin1_General_CP1_CS_AS",
                "Latin1 General, case-sensitive",
            ),
            CollationSelectItem::new("Chinese_PRC_CI_AS", "简体中文, case-insensitive"),
            CollationSelectItem::new("Chinese_PRC_CS_AS", "简体中文, case-sensitive"),
            CollationSelectItem::new("Chinese_Taiwan_Stroke_CI_AS", "繁体中文, case-insensitive"),
            CollationSelectItem::new("Japanese_CI_AS", "日文, case-insensitive"),
            CollationSelectItem::new("Korean_Wansung_CI_AS", "韩文, case-insensitive"),
            CollationSelectItem::new("Latin1_General_CI_AS", "Latin1 General (Windows)"),
            CollationSelectItem::new(
                "Latin1_General_CS_AS",
                "Latin1 General (Windows), case-sensitive",
            ),
            CollationSelectItem::new(
                "Latin1_General_100_CI_AS_SC",
                "Latin1 General 100 (Unicode)",
            ),
        ];

        let collation_select =
            cx.new(|cx| SelectState::new(collation_items, Some(IndexPath::new(0)), window, cx));

        let name_sub = cx.observe(&name_input, |this, _, cx| {
            this.trigger_form_changed(cx);
        });

        let collation_sub = cx.subscribe_in(
            &collation_select,
            window,
            |this, _select, _event: &SelectEvent<Vec<CollationSelectItem>>, _window, cx| {
                this.trigger_form_changed(cx);
            },
        );

        Self {
            focus_handle,
            name_input,
            collation_select,
            is_edit_mode: false,
            _subscriptions: vec![name_sub, collation_sub],
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

        let collation = self
            .collation_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "SQL_Latin1_General_CP1_CI_AS".to_string());

        field_values.insert("name".to_string(), db_name.clone());
        field_values.insert("collation".to_string(), collation);

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

impl EventEmitter<DatabaseFormEvent> for MsSqlDatabaseForm {}

impl Focusable for MsSqlDatabaseForm {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MsSqlDatabaseForm {
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
                        .label("排序规则")
                        .items_center()
                        .label_justify_end()
                        .child(Select::new(&self.collation_select).w_full()),
                ),
        )
    }
}
