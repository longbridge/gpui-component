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
pub struct CharsetSelectItem {
    pub name: String,
    pub description: String,
}

impl CharsetSelectItem {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

impl SelectItem for CharsetSelectItem {
    type Value = String;

    fn title(&self) -> gpui::SharedString {
        format!("{} - {}", self.name, self.description).into()
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }
}

pub struct OracleDatabaseForm {
    focus_handle: FocusHandle,
    name_input: Entity<InputState>,
    charset_select: Entity<SelectState<Vec<CharsetSelectItem>>>,
    tablespace_input: Entity<InputState>,
    is_edit_mode: bool,
    _subscriptions: Vec<Subscription>,
}

impl OracleDatabaseForm {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入用户名/Schema"));

        let charset_items = vec![
            CharsetSelectItem::new("AL32UTF8", "Unicode UTF-8 (推荐)"),
            CharsetSelectItem::new("UTF8", "Unicode UTF-8 (旧版)"),
            CharsetSelectItem::new("ZHS16GBK", "简体中文 GBK"),
            CharsetSelectItem::new("ZHT16MSWIN950", "繁体中文 Big5"),
            CharsetSelectItem::new("JA16SJIS", "日文 Shift-JIS"),
            CharsetSelectItem::new("KO16MSWIN949", "韩文"),
            CharsetSelectItem::new("US7ASCII", "US ASCII"),
            CharsetSelectItem::new("WE8ISO8859P1", "Western European ISO 8859-1"),
            CharsetSelectItem::new("WE8MSWIN1252", "Western European Windows"),
            CharsetSelectItem::new("AL16UTF16", "Unicode UTF-16"),
        ];

        let charset_select =
            cx.new(|cx| SelectState::new(charset_items, Some(IndexPath::new(0)), window, cx));

        let tablespace_input = cx.new(|cx| InputState::new(window, cx).placeholder("USERS (可选)"));

        let name_sub = cx.observe(&name_input, |this, _, cx| {
            this.trigger_form_changed(cx);
        });

        let charset_sub = cx.subscribe_in(
            &charset_select,
            window,
            |this, _select, _event: &SelectEvent<Vec<CharsetSelectItem>>, _window, cx| {
                this.trigger_form_changed(cx);
            },
        );

        let tablespace_sub = cx.observe(&tablespace_input, |this, _, cx| {
            this.trigger_form_changed(cx);
        });

        Self {
            focus_handle,
            name_input,
            charset_select,
            tablespace_input,
            is_edit_mode: false,
            _subscriptions: vec![name_sub, charset_sub, tablespace_sub],
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

        let charset = self
            .charset_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "AL32UTF8".to_string());

        let tablespace = self.tablespace_input.read(cx).text().to_string();

        field_values.insert("name".to_string(), db_name.clone());
        field_values.insert("charset".to_string(), charset);
        if !tablespace.is_empty() {
            field_values.insert("tablespace".to_string(), tablespace);
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

impl EventEmitter<DatabaseFormEvent> for OracleDatabaseForm {}

impl Focusable for OracleDatabaseForm {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for OracleDatabaseForm {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_4().p_4().size_full().child(
            h_form()
                .with_size(Size::Small)
                .columns(1)
                .label_width(px(100.))
                .child(
                    field()
                        .label("用户名/Schema")
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
                        .label("字符集")
                        .items_center()
                        .label_justify_end()
                        .child(Select::new(&self.charset_select).w_full()),
                )
                .child(
                    field()
                        .label("默认表空间")
                        .items_center()
                        .label_justify_end()
                        .child(Input::new(&self.tablespace_input).w_full()),
                ),
        )
    }
}
