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
use rust_i18n::t;

use crate::DatabaseFormEvent;
use db::plugin::DatabaseOperationRequest;

#[derive(Clone, Debug)]
pub struct EncodingSelectItem {
    pub name: String,
    pub description: String,
}

impl EncodingSelectItem {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

impl SelectItem for EncodingSelectItem {
    type Value = String;

    fn title(&self) -> gpui::SharedString {
        format!("{} - {}", self.name, self.description).into()
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }
}

pub struct PostgreSqlDatabaseForm {
    focus_handle: FocusHandle,
    name_input: Entity<InputState>,
    encoding_select: Entity<SelectState<Vec<EncodingSelectItem>>>,
    is_edit_mode: bool,
    _subscriptions: Vec<Subscription>,
}

impl PostgreSqlDatabaseForm {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("Database.enter_database_name").to_string())
        });

        let encoding_items = vec![
            EncodingSelectItem::new("UTF8", "UTF-8 Unicode"),
            EncodingSelectItem::new("SQL_ASCII", "ASCII"),
            EncodingSelectItem::new("LATIN1", "ISO 8859-1 Western European"),
            EncodingSelectItem::new("LATIN2", "ISO 8859-2 Central European"),
            EncodingSelectItem::new("LATIN3", "ISO 8859-3 South European"),
            EncodingSelectItem::new("LATIN4", "ISO 8859-4 North European"),
            EncodingSelectItem::new("LATIN5", "ISO 8859-9 Turkish"),
            EncodingSelectItem::new("LATIN6", "ISO 8859-10 Nordic"),
            EncodingSelectItem::new("LATIN7", "ISO 8859-13 Baltic"),
            EncodingSelectItem::new("LATIN8", "ISO 8859-14 Celtic"),
            EncodingSelectItem::new("LATIN9", "ISO 8859-15 LATIN1 with Euro"),
            EncodingSelectItem::new("ISO_8859_5", "ISO 8859-5 Cyrillic"),
            EncodingSelectItem::new("ISO_8859_6", "ISO 8859-6 Arabic"),
            EncodingSelectItem::new("ISO_8859_7", "ISO 8859-7 Greek"),
            EncodingSelectItem::new("ISO_8859_8", "ISO 8859-8 Hebrew"),
            EncodingSelectItem::new("EUC_JP", "EUC Japanese"),
            EncodingSelectItem::new("EUC_CN", "EUC Simplified Chinese"),
            EncodingSelectItem::new("EUC_KR", "EUC Korean"),
            EncodingSelectItem::new("EUC_TW", "EUC Traditional Chinese"),
            EncodingSelectItem::new("WIN1250", "Windows CP1250 Central European"),
            EncodingSelectItem::new("WIN1251", "Windows CP1251 Cyrillic"),
            EncodingSelectItem::new("WIN1252", "Windows CP1252 Western European"),
            EncodingSelectItem::new("WIN1253", "Windows CP1253 Greek"),
            EncodingSelectItem::new("WIN1254", "Windows CP1254 Turkish"),
            EncodingSelectItem::new("WIN1255", "Windows CP1255 Hebrew"),
            EncodingSelectItem::new("WIN1256", "Windows CP1256 Arabic"),
            EncodingSelectItem::new("WIN1257", "Windows CP1257 Baltic"),
            EncodingSelectItem::new("WIN1258", "Windows CP1258 Vietnamese"),
            EncodingSelectItem::new("WIN866", "Windows CP866 Russian"),
            EncodingSelectItem::new("KOI8R", "KOI8-R Russian"),
            EncodingSelectItem::new("KOI8U", "KOI8-U Ukrainian"),
        ];

        let encoding_select =
            cx.new(|cx| SelectState::new(encoding_items, Some(IndexPath::new(0)), window, cx));

        let name_sub = cx.observe(&name_input, |this, _, cx| {
            this.trigger_form_changed(cx);
        });

        let encoding_sub = cx.subscribe_in(
            &encoding_select,
            window,
            |this, _select, _event: &SelectEvent<Vec<EncodingSelectItem>>, _window, cx| {
                this.trigger_form_changed(cx);
            },
        );

        Self {
            focus_handle,
            name_input,
            encoding_select,
            is_edit_mode: false,
            _subscriptions: vec![name_sub, encoding_sub],
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

        let encoding = self
            .encoding_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "UTF8".to_string());

        field_values.insert("name".to_string(), db_name.clone());
        field_values.insert("encoding".to_string(), encoding);

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

impl EventEmitter<DatabaseFormEvent> for PostgreSqlDatabaseForm {}

impl Focusable for PostgreSqlDatabaseForm {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PostgreSqlDatabaseForm {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_4().p_4().size_full().child(
            h_form()
                .with_size(Size::Small)
                .columns(1)
                .label_width(px(100.))
                .child(
                    field()
                        .label(t!("Database.database_name").to_string())
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
                        .label(t!("Database.encoding").to_string())
                        .items_center()
                        .label_justify_end()
                        .child(Select::new(&self.encoding_select).w_full()),
                ),
        )
    }
}
