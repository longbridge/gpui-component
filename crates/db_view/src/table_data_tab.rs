use crate::table_data::data_grid::{DataGrid, DataGridConfig};
use futures::channel::oneshot;
use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, SharedString, Styled, Task, Window,
};
use gpui_component::button::Button;
use gpui_component::{Icon, IconName, WindowExt, button::ButtonVariants, v_flex};
use one_core::tab_container::{TabContent, TabContentEvent};
use rust_i18n::t;
use std::sync::{Arc, Mutex};

pub struct TableDataTabContent {
    pub data_grid: Entity<DataGrid>,
    database_name: String,
    table_name: String,
    focus_handle: FocusHandle,
}

impl TableDataTabContent {
    pub fn new(
        database_name: String,
        schema_name: Option<String>,
        table_name: String,
        connection_id: impl Into<String>,
        database_type: one_core::storage::DatabaseType,
        editable: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut config = DataGridConfig::new(
            database_name.clone(),
            table_name.clone(),
            connection_id,
            database_type,
        )
        .editable(editable)
        .show_toolbar(true);

        if let Some(schema) = schema_name {
            config = config.with_schema(schema);
        }

        let data_grid = cx.new(|cx| DataGrid::new(config, window, cx));
        let focus_handle = cx.focus_handle();

        Self {
            data_grid,
            database_name,
            table_name,
            focus_handle,
        }
    }
}

impl Render for TableDataTabContent {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().size_full().child(self.data_grid.clone())
    }
}

impl Focusable for TableDataTabContent {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<TabContentEvent> for TableDataTabContent {}

impl TabContent for TableDataTabContent {
    fn content_key(&self) -> &'static str {
        "TableData"
    }

    fn title(&self, _cx: &App) -> SharedString {
        format!("{}.{} - Data", self.database_name, self.table_name).into()
    }

    fn icon(&self, _cx: &App) -> Option<Icon> {
        Some(IconName::TableData.color())
    }

    fn closeable(&self, _cx: &App) -> bool {
        true
    }

    fn try_close(
        &mut self,
        _tab_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        let has_changes = self.data_grid.read(cx).has_unsaved_changes(cx);
        if !has_changes {
            return Task::ready(true);
        }

        let table_name = format!("{}.{}", self.database_name, self.table_name);
        let data_grid = self.data_grid.clone();

        let (tx, rx) = oneshot::channel::<bool>();
        let tx = Arc::new(Mutex::new(Some(tx)));

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let tx_save = tx.clone();
            let tx_discard = tx.clone();
            let tx_cancel = tx.clone();
            let data_grid = data_grid.clone();

            dialog
                .title(format!("{} {}", t!("Common.close"), table_name))
                .overlay_closable(false)
                .close_button(false)
                .footer(move |_ok, _cancel, _window, _cx| {
                    let data_grid = data_grid.clone();
                    let tx_save = tx_save.clone();
                    let tx_discard = tx_discard.clone();
                    let tx_cancel = tx_cancel.clone();

                    vec![
                        Button::new("cancel")
                            .label(t!("Common.cancel"))
                            .on_click(move |_, window: &mut Window, cx| {
                                window.close_dialog(cx);
                                if let Some(tx) = tx_cancel.lock().ok().and_then(|mut g| g.take()) {
                                    let _ = tx.send(false);
                                }
                            })
                            .into_any_element(),
                        Button::new("discard")
                            .label(t!("Common.discard"))
                            .on_click(move |_, window: &mut Window, cx| {
                                window.close_dialog(cx);
                                if let Some(tx) = tx_discard.lock().ok().and_then(|mut g| g.take())
                                {
                                    let _ = tx.send(true);
                                }
                            })
                            .into_any_element(),
                        Button::new("save")
                            .label(t!("Common.save"))
                            .primary()
                            .on_click(move |_, window: &mut Window, cx| {
                                window.close_dialog(cx);
                                data_grid.update(cx, |grid, cx| {
                                    grid.save_changes(window, cx);
                                });
                                if let Some(tx) = tx_save.lock().ok().and_then(|mut g| g.take()) {
                                    let _ = tx.send(true);
                                }
                            })
                            .into_any_element(),
                    ]
                })
                .child(t!("Table.unsaved_changes_prompt").to_string())
        });

        cx.spawn(async move |_handle, _cx| rx.await.unwrap_or(false))
    }
}

impl Clone for TableDataTabContent {
    fn clone(&self) -> Self {
        Self {
            data_grid: self.data_grid.clone(),
            database_name: self.database_name.clone(),
            table_name: self.table_name.clone(),
            focus_handle: self.focus_handle.clone(),
        }
    }
}
