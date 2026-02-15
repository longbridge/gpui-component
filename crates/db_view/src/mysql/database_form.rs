use std::collections::HashMap;

use db::mysql::MySqlPlugin;
use db::plugin::{DatabaseOperationRequest, DatabasePlugin};
use db::types::{CharsetInfo, CollationInfo};
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

#[derive(Clone, Debug)]
pub struct CharsetSelectItem {
    pub info: CharsetInfo,
}

impl CharsetSelectItem {
    pub fn new(info: CharsetInfo) -> Self {
        Self { info }
    }
}

impl SelectItem for CharsetSelectItem {
    type Value = String;

    fn title(&self) -> gpui::SharedString {
        format!("{} - {}", self.info.name, self.info.description).into()
    }

    fn value(&self) -> &Self::Value {
        &self.info.name
    }
}

#[derive(Clone, Debug)]
pub struct CollationSelectItem {
    pub info: CollationInfo,
}

impl CollationSelectItem {
    pub fn new(info: CollationInfo) -> Self {
        Self { info }
    }
}

impl SelectItem for CollationSelectItem {
    type Value = String;

    fn title(&self) -> gpui::SharedString {
        if self.info.is_default {
            format!("{} (default)", self.info.name).into()
        } else {
            self.info.name.clone().into()
        }
    }

    fn value(&self) -> &Self::Value {
        &self.info.name
    }
}

pub struct MySqlDatabaseForm {
    focus_handle: FocusHandle,
    name_input: Entity<InputState>,
    charset_select: Entity<SelectState<Vec<CharsetSelectItem>>>,
    collation_select: Entity<SelectState<Vec<CollationSelectItem>>>,
    is_edit_mode: bool,
    plugin: MySqlPlugin,
    _subscriptions: Vec<Subscription>,
}

impl MySqlDatabaseForm {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let plugin = MySqlPlugin::new();

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入数据库名称"));

        let charset_items: Vec<CharsetSelectItem> = plugin
            .get_charsets()
            .into_iter()
            .map(CharsetSelectItem::new)
            .collect();

        let charset_select =
            cx.new(|cx| SelectState::new(charset_items, Some(IndexPath::new(0)), window, cx));

        let default_charset = plugin
            .get_charsets()
            .first()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "utf8mb4".to_string());

        let collation_items: Vec<CollationSelectItem> = plugin
            .get_collations(&default_charset)
            .into_iter()
            .map(CollationSelectItem::new)
            .collect();

        let default_collation_index = collation_items
            .iter()
            .position(|c| c.info.is_default)
            .unwrap_or(0);

        let collation_select = cx.new(|cx| {
            SelectState::new(
                collation_items,
                Some(IndexPath::new(default_collation_index)),
                window,
                cx,
            )
        });

        let name_sub = cx.observe(&name_input, |this, _, cx| {
            this.trigger_form_changed(cx);
        });

        let charset_select_clone = charset_select.clone();
        let collation_select_clone = collation_select.clone();
        let charset_sub = cx.subscribe_in(
            &charset_select,
            window,
            move |this, _select, _event: &SelectEvent<Vec<CharsetSelectItem>>, window, cx| {
                this.on_charset_changed(&charset_select_clone, &collation_select_clone, window, cx);
            },
        );

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
            charset_select,
            collation_select,
            is_edit_mode: false,
            plugin,
            _subscriptions: vec![name_sub, charset_sub, collation_sub],
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

    fn on_charset_changed(
        &mut self,
        charset_select: &Entity<SelectState<Vec<CharsetSelectItem>>>,
        collation_select: &Entity<SelectState<Vec<CollationSelectItem>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selected_charset = charset_select.read(cx).selected_value().cloned();

        if let Some(charset) = selected_charset {
            let collations = self.plugin.get_collations(&charset);
            let collation_items: Vec<CollationSelectItem> = collations
                .into_iter()
                .map(CollationSelectItem::new)
                .collect();

            let default_index = collation_items
                .iter()
                .position(|c| c.info.is_default)
                .unwrap_or(0);

            collation_select.update(cx, |state, cx| {
                state.set_items(collation_items, window, cx);
                state.set_selected_index(Some(IndexPath::new(default_index)), window, cx);
            });

            self.trigger_form_changed(cx);
        }
    }

    fn build_request(&self, cx: &App) -> DatabaseOperationRequest {
        let mut field_values = HashMap::new();

        let db_name = self.name_input.read(cx).text().to_string();

        let charset = self
            .charset_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "utf8mb4".to_string());

        let collation = self
            .collation_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "utf8mb4_general_ci".to_string());

        field_values.insert("name".to_string(), db_name.clone());
        field_values.insert("charset".to_string(), charset);
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

impl EventEmitter<DatabaseFormEvent> for MySqlDatabaseForm {}

impl Focusable for MySqlDatabaseForm {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MySqlDatabaseForm {
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
                        .label("字符集")
                        .items_center()
                        .label_justify_end()
                        .child(Select::new(&self.charset_select).w_full()),
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
