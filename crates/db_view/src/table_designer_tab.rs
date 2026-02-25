use futures::channel::oneshot;
use gpui::prelude::*;
use gpui::{
    AnyElement, App, AsyncApp, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ListSizingBehavior, MouseButton, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Task, UniformListScrollHandle,
    Window, div, px, uniform_list,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, Sizable, Size, WindowExt,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    form::{field, h_form},
    h_flex,
    highlighter::Language,
    input::{Input, InputEvent, InputState},
    scroll::Scrollbar,
    select::{Select, SelectItem, SelectState},
    tab::{Tab, TabBar},
    v_flex,
};
use std::ops::Range;
use std::sync::{Arc, Mutex};

use crate::database_view_plugin::{ColumnEditorCapabilities, DatabaseViewPluginRegistry};
use db::GlobalDbState;
use db::types::{
    CharsetInfo, CollationInfo, ColumnDefinition, ColumnInfo, IndexDefinition, IndexInfo,
    TableDesign, TableOptions,
};
use gpui_component::select::SearchableVec;
use one_core::storage::DatabaseType;
use one_core::tab_container::{TabContainer, TabContent, TabContentEvent};
use rust_i18n::t;

#[derive(Clone, Debug, PartialEq)]
pub enum DesignerTab {
    Columns,
    Indexes,
    Options,
    SqlPreview,
}

#[derive(Clone, Debug)]
pub enum TableDesignerEvent {
    Saved {
        connection_id: String,
        database_name: String,
        schema_name: Option<String>,
        table_name: String,
        is_new_table: bool,
        tab_id: Option<String>,
    },
}

#[derive(Clone)]
pub struct TableDesignerConfig {
    pub connection_id: String,
    pub database_name: String,
    pub schema_name: Option<String>,
    pub database_type: DatabaseType,
    pub table_name: Option<String>,
    pub tab_id: Option<String>,
}

impl TableDesignerConfig {
    pub fn new(
        connection_id: impl Into<String>,
        database_name: impl Into<String>,
        database_type: DatabaseType,
    ) -> Self {
        Self {
            connection_id: connection_id.into(),
            database_name: database_name.into(),
            schema_name: None,
            database_type,
            table_name: None,
            tab_id: None,
        }
    }

    pub fn with_schema_name(mut self, name: impl Into<String>) -> Self {
        self.schema_name = Some(name.into());
        self
    }

    pub fn with_table_name(mut self, name: impl Into<String>) -> Self {
        self.table_name = Some(name.into());
        self
    }

    pub fn with_tab_id(mut self, id: impl Into<String>) -> Self {
        self.tab_id = Some(id.into());
        self
    }
}

pub struct TableDesigner {
    title: SharedString,
    focus_handle: FocusHandle,
    config: TableDesignerConfig,
    active_tab: DesignerTab,
    table_name_input: Entity<InputState>,
    table_comment_input: Entity<InputState>,
    engine_select: Entity<SelectState<Vec<EngineSelectItem>>>,
    charset_select: Entity<SelectState<Vec<CharsetSelectItem>>>,
    collation_select: Entity<SelectState<Vec<CollationSelectItem>>>,
    auto_increment_input: Entity<InputState>,
    columns_editor: Entity<ColumnsEditor>,
    indexes_editor: Entity<IndexesEditor>,
    _charsets: Vec<CharsetInfo>,
    sql_preview_input: Entity<InputState>,
    original_design: Option<TableDesign>,
    _subscriptions: Vec<Subscription>,
}

impl TableDesigner {
    pub fn new(
        title: impl Into<SharedString>,
        config: TableDesignerConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title = title.into();
        let focus_handle = cx.focus_handle();

        let table_name_input = cx.new(|cx| {
            let mut input =
                InputState::new(window, cx).placeholder(t!("Table.enter_table_name").to_string());
            if let Some(name) = &config.table_name {
                input.set_value(name.clone(), window, cx);
            }
            input
        });

        let table_comment_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("Table.table_comment").to_string())
        });

        let auto_increment_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("Table.auto_increment").to_string())
        });

        let (engines, column_editor_capabilities): (
            Vec<EngineSelectItem>,
            ColumnEditorCapabilities,
        ) = {
            let registry = cx.global::<DatabaseViewPluginRegistry>();
            if let Some(view_plugin) = registry.get(&config.database_type) {
                let engines = view_plugin
                    .get_engines()
                    .into_iter()
                    .map(|name| EngineSelectItem { name })
                    .collect();
                let capabilities = view_plugin.get_column_editor_capabilities();
                (engines, capabilities)
            } else {
                (vec![], ColumnEditorCapabilities::default())
            }
        };

        let engine_select = cx.new(|cx| {
            if engines.is_empty() {
                SelectState::new(vec![], None, window, cx)
            } else {
                SelectState::new(engines, Some(IndexPath::new(0)), window, cx)
            }
        });

        let charsets = Self::get_charsets(&config.database_type, cx);
        let charset_items: Vec<CharsetSelectItem> = charsets
            .iter()
            .cloned()
            .map(|info| CharsetSelectItem { info })
            .collect();
        let charset_select =
            cx.new(|cx| SelectState::new(charset_items, Some(IndexPath::new(0)), window, cx));

        let default_charset = charsets
            .first()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "utf8mb4".to_string());
        let collations = Self::get_collations(&config.database_type, &default_charset, cx);
        let collation_items: Vec<CollationSelectItem> = collations
            .iter()
            .cloned()
            .map(|info| CollationSelectItem { info })
            .collect();
        let default_coll_idx = collation_items
            .iter()
            .position(|c| c.info.is_default)
            .unwrap_or(0);
        let collation_select = cx.new(|cx| {
            SelectState::new(
                collation_items,
                Some(IndexPath::new(default_coll_idx)),
                window,
                cx,
            )
        });

        let columns_editor = cx.new(|cx| {
            ColumnsEditor::new(
                config.database_type,
                charsets.clone(),
                column_editor_capabilities,
                window,
                cx,
            )
        });
        let indexes_editor = cx.new(|cx| IndexesEditor::new(window, cx));

        let sql_preview_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(Language::from_str("sql"))
                .line_number(false)
                .multi_line(true)
        });

        let name_sub = cx.subscribe_in(
            &table_name_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::Change = event {
                    this.update_sql_preview(window, cx);
                }
            },
        );

        let comment_sub = cx.subscribe_in(
            &table_comment_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::Change = event {
                    this.update_sql_preview(window, cx);
                }
            },
        );

        let auto_inc_sub = cx.subscribe_in(
            &auto_increment_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::Change = event {
                    this.update_sql_preview(window, cx);
                }
            },
        );

        let engine_sub = cx.observe_in(&engine_select, window, |this, _, window, cx| {
            this.update_sql_preview(window, cx);
        });

        let charset_select_clone = charset_select.clone();
        let charset_sub = cx.observe_in(&charset_select, window, move |this, _, window, cx| {
            this.update_sql_preview(window, cx);
            this.update_collations_for_charset(&charset_select_clone, window, cx);
        });

        let collation_sub = cx.observe_in(&collation_select, window, |this, _, window, cx| {
            this.update_sql_preview(window, cx);
        });

        let cols_sub = cx.subscribe_in(
            &columns_editor,
            window,
            |this, _, _: &ColumnsEditorEvent, window, cx| {
                this.update_sql_preview(window, cx);
            },
        );

        let idx_sub = cx.subscribe_in(
            &indexes_editor,
            window,
            |this, _, _: &IndexesEditorEvent, window, cx| {
                this.update_sql_preview(window, cx);
            },
        );

        let mut designer = Self {
            title,
            focus_handle,
            config,
            active_tab: DesignerTab::Columns,
            table_name_input,
            table_comment_input,
            engine_select,
            charset_select,
            collation_select,
            auto_increment_input,
            columns_editor,
            indexes_editor,
            _charsets: charsets,
            sql_preview_input,
            original_design: None,
            _subscriptions: vec![
                name_sub,
                comment_sub,
                auto_inc_sub,
                engine_sub,
                charset_sub,
                collation_sub,
                cols_sub,
                idx_sub,
            ],
        };

        designer.update_sql_preview(window, cx);

        if designer.config.table_name.is_some() {
            designer.load_table_structure(cx);
        }

        designer
    }

    fn get_charsets(database_type: &DatabaseType, cx: &App) -> Vec<CharsetInfo> {
        let global_state = cx.global::<GlobalDbState>();
        if let Ok(plugin) = global_state.db_manager.get_plugin(database_type) {
            plugin.get_charsets()
        } else {
            vec![CharsetInfo {
                name: "utf8mb4".to_string(),
                description: "UTF-8 Unicode".to_string(),
                default_collation: "utf8mb4_general_ci".to_string(),
            }]
        }
    }

    fn get_collations(database_type: &DatabaseType, charset: &str, cx: &App) -> Vec<CollationInfo> {
        let global_state = cx.global::<GlobalDbState>();
        if let Ok(plugin) = global_state.db_manager.get_plugin(database_type) {
            plugin.get_collations(charset)
        } else {
            vec![CollationInfo {
                name: "utf8mb4_general_ci".to_string(),
                charset: "utf8mb4".to_string(),
                is_default: true,
            }]
        }
    }

    fn update_collations_for_charset(
        &mut self,
        charset_select: &Entity<SelectState<Vec<CharsetSelectItem>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selected_charset = charset_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "utf8mb4".to_string());

        let collations = Self::get_collations(&self.config.database_type, &selected_charset, cx);
        let collation_items: Vec<CollationSelectItem> = collations
            .iter()
            .cloned()
            .map(|info| CollationSelectItem { info })
            .collect();
        let default_idx = collation_items
            .iter()
            .position(|c| c.info.is_default)
            .unwrap_or(0);

        self.collation_select.update(cx, |state, inner_cx| {
            state.set_items(collation_items, window, inner_cx);
            state.set_selected_index(Some(IndexPath::new(default_idx)), window, inner_cx);
        });
    }

    fn collect_design(&self, cx: &App) -> TableDesign {
        let table_name = self.table_name_input.read(cx).text().to_string();
        let table_comment = self.table_comment_input.read(cx).text().to_string();
        let columns = self.columns_editor.read(cx).get_columns(cx);
        let indexes = self.indexes_editor.read(cx).get_indexes(cx);

        let engine = self.engine_select.read(cx).selected_value().cloned();
        let charset = self.charset_select.read(cx).selected_value().cloned();
        let collation = self.collation_select.read(cx).selected_value().cloned();
        let auto_increment_str = self.auto_increment_input.read(cx).text().to_string();
        let auto_increment = auto_increment_str.parse::<u64>().ok();

        let options = TableOptions {
            engine,
            charset,
            collation,
            comment: table_comment,
            auto_increment,
        };

        TableDesign {
            database_name: self.config.database_name.clone(),
            table_name,
            columns,
            indexes,
            foreign_keys: vec![],
            options,
        }
    }

    fn update_sql_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let design = self.collect_design(cx);
        let global_state = cx.global::<GlobalDbState>().clone();

        let sql = if let Ok(plugin) = global_state
            .db_manager
            .get_plugin(&self.config.database_type)
        {
            if let Some(original) = &self.original_design {
                plugin.build_alter_table_sql(original, &design)
            } else {
                plugin.build_create_table_sql(&design)
            }
        } else {
            String::new()
        };

        self.sql_preview_input.update(cx, |state, cx| {
            state.set_value(sql, window, cx);
        });
        cx.notify();
    }

    pub fn has_unsaved_changes(&self, cx: &App) -> bool {
        let sql = self.sql_preview_input.read(cx).text().to_string();
        let trimmed = sql.trim();
        let no_changes_localized = t!("SqlEditor.no_changes").to_string();
        !trimmed.is_empty()
            && !trimmed.starts_with("-- No changes")
            && !trimmed.starts_with(no_changes_localized.as_str())
    }

    pub fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.handle_execute(&gpui::ClickEvent::default(), window, cx);
    }

    pub fn save_and_close(
        &mut self,
        tab_container: Entity<TabContainer>,
        tab_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let design = self.collect_design(cx);
        if design.table_name.is_empty() {
            window.push_notification(t!("Table.please_enter_table_name").to_string(), cx);
            return;
        }
        if design.columns.is_empty() {
            window.push_notification(t!("Table.please_add_column").to_string(), cx);
            return;
        }

        let global_state = cx.global::<GlobalDbState>().clone();
        let connection_id = self.config.connection_id.clone();
        let database_name = self.config.database_name.clone();
        let database_type = self.config.database_type;
        let schema = self.config.schema_name.clone();
        let original_design = self.original_design.clone();
        let is_new_table = original_design.is_none();
        let table_name = design.table_name.clone();
        let config_tab_id = self.config.tab_id.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let sql = {
                let plugin_result = cx.update(|cx: &mut App| {
                    let global_state = cx.global::<GlobalDbState>().clone();
                    global_state.db_manager.get_plugin(&database_type)
                });
                match plugin_result {
                    Ok(plugin) => {
                        if let Some(original) = &original_design {
                            plugin.build_alter_table_sql(original, &design)
                        } else {
                            plugin.build_create_table_sql(&design)
                        }
                    }
                    _ => return,
                }
            };

            let no_changes_localized = t!("SqlEditor.no_changes").to_string();
            if sql.trim().is_empty()
                || sql.starts_with("-- No changes")
                || sql.starts_with(no_changes_localized.as_str())
            {
                let _ = cx.update(|cx: &mut App| {
                    if let Some(window_id) = cx.active_window() {
                        let _ = cx.update_window(window_id, |_, _window, cx| {
                            tab_container.update(cx, |container: &mut TabContainer, cx| {
                                container.force_close_tab_by_id(&tab_id, cx);
                            });
                        });
                    }
                });
                return;
            }

            let result = global_state
                .execute_script(
                    cx,
                    connection_id.clone(),
                    sql,
                    Some(database_name.clone()),
                    schema.clone(),
                    None,
                )
                .await;

            let _ = cx.update(|cx: &mut App| {
                if let Some(window_id) = cx.active_window() {
                    let _ = cx.update_window(window_id, |_, window, cx| match &result {
                        Ok(results) => {
                            let has_error = results.iter().any(|r| r.is_error());
                            if has_error {
                                let error_msg = results
                                    .iter()
                                    .filter_map(|r| {
                                        if let db::executor::SqlResult::Error(err) = r {
                                            Some(err.message.clone())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join("; ");
                                window.push_notification(
                                    format!(
                                        "{}: {}",
                                        &t!("Table.execute_failed").to_string(),
                                        error_msg
                                    ),
                                    cx,
                                );
                            } else {
                                let msg = if is_new_table {
                                    t!("Table.create_success").to_string()
                                } else {
                                    t!("Table.modify_success").to_string()
                                };
                                window.push_notification(msg, cx);
                                let _ = this.update(cx, |_designer, cx| {
                                    cx.emit(TableDesignerEvent::Saved {
                                        connection_id: connection_id.clone(),
                                        database_name: database_name.clone(),
                                        schema_name: schema.clone(),
                                        table_name: table_name.clone(),
                                        is_new_table,
                                        tab_id: config_tab_id.clone(),
                                    });
                                });
                                tab_container.update(cx, |container: &mut TabContainer, cx| {
                                    container.force_close_tab_by_id(&tab_id, cx);
                                });
                            }
                        }
                        Err(e) => {
                            let msg = if is_new_table {
                                t!("Table.create_failed").to_string()
                            } else {
                                t!("Table.modify_failed").to_string()
                            };
                            window.push_notification(format!("{}: {}", msg, e), cx);
                        }
                    });
                }
            });
        })
        .detach();
    }

    pub fn load_table_structure(&mut self, cx: &mut Context<Self>) {
        let Some(table_name) = self.config.table_name.clone() else {
            return;
        };

        let global_state = cx.global::<GlobalDbState>().clone();
        let connection_id = self.config.connection_id.clone();
        let database_name = self.config.database_name.clone();
        let schema_name = self.config.schema_name.clone();
        let columns_editor = self.columns_editor.clone();
        let indexes_editor = self.indexes_editor.clone();
        let sql_preview_input = self.sql_preview_input.clone();
        let database_type = self.config.database_type;

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let columns_result = global_state
                .list_columns(
                    cx,
                    connection_id.clone(),
                    database_name.clone(),
                    schema_name.clone(),
                    table_name.clone(),
                )
                .await;

            let indexes_result = global_state
                .list_indexes(
                    cx,
                    connection_id.clone(),
                    database_name.clone(),
                    schema_name.clone(),
                    table_name.clone(),
                )
                .await;

            let _ = cx.update(|cx| {
                if let Some(window_id) = cx.active_window() {
                    cx.update_window(window_id, |_entity, window, cx| {
                        let columns = columns_result.ok();
                        let indexes = indexes_result.ok();

                        if let Some(ref cols) = columns {
                            columns_editor.update(cx, |editor, cx| {
                                editor.load_columns(cols.clone(), window, cx);
                            });
                        }

                        if let Some(ref idxs) = indexes {
                            indexes_editor.update(cx, |editor, cx| {
                                editor.load_indexes(idxs.clone(), window, cx);
                            });
                        }

                        let sql = this
                            .update(cx, |designer, cx| {
                                let original_design = designer.build_original_design(
                                    columns.unwrap_or_default(),
                                    indexes.unwrap_or_default(),
                                );
                                designer.original_design = Some(original_design);

                                let design = designer.collect_design(cx);
                                let global_state = cx.global::<GlobalDbState>().clone();
                                if let Ok(plugin) =
                                    global_state.db_manager.get_plugin(&database_type)
                                {
                                    if let Some(original) = &designer.original_design {
                                        plugin.build_alter_table_sql(original, &design)
                                    } else {
                                        plugin.build_create_table_sql(&design)
                                    }
                                } else {
                                    String::new()
                                }
                            })
                            .ok()
                            .unwrap_or_default();

                        sql_preview_input.update(cx, |state, cx| {
                            state.set_value(sql, window, cx);
                        });
                    })
                } else {
                    Err(anyhow::anyhow!("No active window"))
                }
            });
        })
        .detach();
    }

    fn build_original_design(
        &self,
        columns: Vec<ColumnInfo>,
        indexes: Vec<IndexInfo>,
    ) -> TableDesign {
        let is_sqlite = self.config.database_type == DatabaseType::SQLite;
        let column_defs: Vec<ColumnDefinition> = columns
            .iter()
            .map(|col| {
                let (data_type, length) = Self::parse_data_type(&col.data_type);
                let scale = Self::extract_scale_from_type_str(&col.data_type);
                let is_auto_increment = if is_sqlite {
                    col.is_primary_key && data_type.to_uppercase() == "INTEGER"
                } else {
                    col.data_type.to_uppercase().contains("AUTO_INCREMENT")
                };
                ColumnDefinition {
                    name: col.name.clone(),
                    data_type,
                    length,
                    precision: None,
                    scale,
                    is_nullable: col.is_nullable,
                    is_primary_key: col.is_primary_key,
                    is_auto_increment,
                    is_unsigned: col.data_type.to_uppercase().contains("UNSIGNED"),
                    default_value: col.default_value.clone(),
                    comment: col.comment.clone().unwrap_or_default(),
                    charset: None,
                    collation: None,
                }
            })
            .collect();

        let index_defs: Vec<IndexDefinition> = indexes
            .iter()
            .filter(|idx| idx.name.to_uppercase() != "PRIMARY")
            .map(|idx| IndexDefinition {
                name: idx.name.clone(),
                columns: idx.columns.clone(),
                is_unique: idx.is_unique,
                is_primary: false,
                index_type: idx.index_type.clone(),
                comment: String::new(),
            })
            .collect();

        TableDesign {
            database_name: self.config.database_name.clone(),
            table_name: self.config.table_name.clone().unwrap_or_default(),
            columns: column_defs,
            indexes: index_defs,
            foreign_keys: vec![],
            options: TableOptions::default(),
        }
    }

    fn parse_data_type(data_type: &str) -> (String, Option<u32>) {
        if let Some(start) = data_type.find('(') {
            if let Some(end) = data_type.find(')') {
                let base_type = data_type[..start].trim().to_string();
                let len_str = &data_type[start + 1..end];
                if let Some(comma) = len_str.find(',') {
                    let length = len_str[..comma].trim().parse().ok();
                    return (base_type, length);
                }
                let length = len_str.trim().parse().ok();
                return (base_type, length);
            }
        }
        (data_type.to_string(), None)
    }

    fn extract_scale_from_type_str(data_type: &str) -> Option<u32> {
        if let Some(start) = data_type.find('(') {
            if let Some(end) = data_type.find(')') {
                let len_str = &data_type[start + 1..end];
                if let Some(comma) = len_str.find(',') {
                    return len_str[comma + 1..].trim().parse().ok();
                }
            }
        }
        None
    }

    fn handle_execute(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let design = self.collect_design(cx);
        if design.table_name.is_empty() {
            window.push_notification(t!("Table.please_enter_table_name").to_string(), cx);
            return;
        }
        if design.columns.is_empty() {
            window.push_notification(t!("Table.please_add_column").to_string(), cx);
            return;
        }

        let global_state = cx.global::<GlobalDbState>().clone();
        let connection_id = self.config.connection_id.clone();
        let database_name = self.config.database_name.clone();
        let database_type = self.config.database_type;
        let schema = self.config.schema_name.clone();
        let original_design = self.original_design.clone();
        let is_new_table = original_design.is_none();
        let table_name = design.table_name.clone();
        let tab_id = self.config.tab_id.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let sql = {
                let plugin_result = cx.update(|cx: &mut App| {
                    let global_state = cx.global::<GlobalDbState>().clone();
                    global_state.db_manager.get_plugin(&database_type)
                });
                match plugin_result {
                    Ok(plugin) => {
                        if let Some(original) = &original_design {
                            plugin.build_alter_table_sql(original, &design)
                        } else {
                            plugin.build_create_table_sql(&design)
                        }
                    }
                    _ => return,
                }
            };

            let no_changes_localized = t!("SqlEditor.no_changes").to_string();
            if sql.trim().is_empty()
                || sql.starts_with("-- No changes")
                || sql.starts_with(no_changes_localized.as_str())
            {
                let _ = cx.update(|cx: &mut App| {
                    if let Some(window_id) = cx.active_window() {
                        let _ = cx.update_window(window_id, |_, window, cx| {
                            window.push_notification(t!("Table.no_changes").to_string(), cx);
                        });
                    }
                });
                return;
            }

            let result = global_state
                .execute_script(
                    cx,
                    connection_id.clone(),
                    sql,
                    Some(database_name.clone()),
                    schema.clone(),
                    None,
                )
                .await;

            let _ = cx.update(|cx: &mut App| {
                if let Some(window_id) = cx.active_window() {
                    let _ = cx.update_window(window_id, |_, window, cx| match &result {
                        Ok(results) => {
                            let has_error = results.iter().any(|r| r.is_error());
                            if has_error {
                                let error_msg = results
                                    .iter()
                                    .filter_map(|r| {
                                        if let db::executor::SqlResult::Error(err) = r {
                                            Some(err.message.clone())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join("; ");
                                window.push_notification(
                                    format!(
                                        "{}: {}",
                                        &t!("Table.execute_failed").to_string(),
                                        error_msg
                                    ),
                                    cx,
                                );
                            } else {
                                let msg = if is_new_table {
                                    t!("Table.create_success").to_string()
                                } else {
                                    t!("Table.modify_success").to_string()
                                };
                                window.push_notification(msg, cx);
                                let _ = this.update(cx, |designer, cx| {
                                    cx.emit(TableDesignerEvent::Saved {
                                        connection_id: connection_id.clone(),
                                        database_name: database_name.clone(),
                                        schema_name: schema.clone(),
                                        table_name: table_name.clone(),
                                        is_new_table,
                                        tab_id: tab_id.clone(),
                                    });
                                    if is_new_table {
                                        designer.config.table_name = Some(table_name.clone());
                                    }
                                    designer.load_table_structure(cx);
                                });
                            }
                        }
                        Err(e) => {
                            let msg = if is_new_table {
                                t!("Table.create_failed").to_string()
                            } else {
                                t!("Table.modify_failed").to_string()
                            };
                            window.push_notification(format!("{}: {}", msg, e), cx);
                        }
                    });
                }
            });
        })
        .detach();
    }

    fn render_toolbar(&self, cx: &Context<Self>) -> AnyElement {
        h_flex()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .gap_4()
            .items_center()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("Table.table_name").to_string()),
                    )
                    .child(Input::new(&self.table_name_input).w(px(200.)).small()),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("Table.comment").to_string()),
                    )
                    .child(Input::new(&self.table_comment_input).w(px(300.)).small()),
            )
            .child(div().flex_1())
            .child(
                Button::new("execute")
                    .small()
                    .primary()
                    .label(t!("Common.save").to_string())
                    .on_click(cx.listener(Self::handle_execute)),
            )
            .into_any_element()
    }

    fn render_tabs(&self, cx: &Context<Self>) -> AnyElement {
        let active_idx = match self.active_tab {
            DesignerTab::Columns => 0,
            DesignerTab::Indexes => 1,
            DesignerTab::Options => 2,
            DesignerTab::SqlPreview => 3,
        };

        h_flex()
            .w_full()
            .justify_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                TabBar::new("designer-tabs")
                    .underline()
                    .with_size(Size::Small)
                    .selected_index(active_idx)
                    .on_click(cx.listener(|this, ix: &usize, _window, cx| {
                        this.active_tab = match ix {
                            0 => DesignerTab::Columns,
                            1 => DesignerTab::Indexes,
                            2 => DesignerTab::Options,
                            3 => DesignerTab::SqlPreview,
                            _ => DesignerTab::Columns,
                        };
                        cx.notify();
                    }))
                    .child(Tab::new().label(t!("Table.columns").to_string()))
                    .child(Tab::new().label(t!("Table.indexes").to_string()))
                    .child(Tab::new().label(t!("Table.options").to_string()))
                    .child(Tab::new().label(t!("Table.sql_preview").to_string())),
            )
            .into_any_element()
    }

    fn render_active_tab(&self, _window: &mut Window, cx: &Context<Self>) -> AnyElement {
        match self.active_tab {
            DesignerTab::Columns => self.columns_editor.clone().into_any_element(),
            DesignerTab::Indexes => self.indexes_editor.clone().into_any_element(),
            DesignerTab::Options => self.render_options(cx),
            DesignerTab::SqlPreview => self.render_sql_preview(cx),
        }
    }

    fn render_options(&self, cx: &Context<Self>) -> AnyElement {
        let registry = cx.global::<DatabaseViewPluginRegistry>();
        let capabilities = registry
            .get(&self.config.database_type)
            .map(|plugin| plugin.get_table_designer_capabilities())
            .unwrap_or_default();

        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .when(capabilities.supports_engine, |this| {
                this.child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(
                            div()
                                .w(px(80.))
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("Table.engine").to_string()),
                        )
                        .child(Select::new(&self.engine_select).w(px(200.)).small()),
                )
            })
            .when(capabilities.supports_charset, |this| {
                this.child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(
                            div()
                                .w(px(80.))
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("Table.charset").to_string()),
                        )
                        .child(Select::new(&self.charset_select).w(px(200.)).small()),
                )
            })
            .when(capabilities.supports_collation, |this| {
                this.child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(
                            div()
                                .w(px(80.))
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("Table.collation").to_string()),
                        )
                        .child(Select::new(&self.collation_select).w(px(200.)).small()),
                )
            })
            .when(capabilities.supports_auto_increment, |this| {
                this.child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(
                            div()
                                .w(px(80.))
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("Table.auto_increment").to_string()),
                        )
                        .child(Input::new(&self.auto_increment_input).w(px(200.)).small()),
                )
            })
            .into_any_element()
    }

    fn render_sql_preview(&self, _cx: &Context<Self>) -> AnyElement {
        v_flex()
            .size_full()
            .p_4()
            .child(
                Input::new(&self.sql_preview_input)
                    .size_full()
                    .disabled(true),
            )
            .into_any_element()
    }
}

impl Focusable for TableDesigner {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TableDesigner {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(self.render_toolbar(cx))
            .child(self.render_tabs(cx))
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(self.render_active_tab(window, cx)),
            )
    }
}

impl EventEmitter<TableDesignerEvent> for TableDesigner {}

impl EventEmitter<TabContentEvent> for TableDesigner {}

impl TabContent for TableDesigner {
    fn content_key(&self) -> &'static str {
        "TableDesigner"
    }

    fn title(&self, _cx: &App) -> SharedString {
        self.title.clone()
    }

    fn icon(&self, _cx: &App) -> Option<Icon> {
        Some(IconName::TableDesign.color())
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
        let has_changes = self.has_unsaved_changes(cx);
        if !has_changes {
            return Task::ready(true);
        }

        let title = self.title.clone();
        let designer_entity = cx.entity().clone();

        let (tx, rx) = oneshot::channel::<bool>();
        let tx = Arc::new(Mutex::new(Some(tx)));

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let tx_save = tx.clone();
            let tx_discard = tx.clone();
            let tx_cancel = tx.clone();
            let designer_entity = designer_entity.clone();

            dialog
                .title(format!("{} {}", t!("Common.close"), title))
                .overlay_closable(false)
                .close_button(false)
                .footer(move |_ok, _cancel, _window, _cx| {
                    let tx_save = tx_save.clone();
                    let tx_discard = tx_discard.clone();
                    let tx_cancel = tx_cancel.clone();
                    let designer_entity = designer_entity.clone();

                    vec![
                        Button::new("cancel")
                            .label(t!("Common.cancel").to_string())
                            .on_click(move |_, window, cx| {
                                window.close_dialog(cx);
                                if let Some(tx) = tx_cancel.lock().ok().and_then(|mut g| g.take()) {
                                    let _ = tx.send(false);
                                }
                            })
                            .into_any_element(),
                        Button::new("discard")
                            .label(t!("Common.discard").to_string())
                            .on_click(move |_, window, cx| {
                                window.close_dialog(cx);
                                if let Some(tx) = tx_discard.lock().ok().and_then(|mut g| g.take())
                                {
                                    let _ = tx.send(true);
                                }
                            })
                            .into_any_element(),
                        Button::new("save")
                            .label(t!("Common.save").to_string())
                            .primary()
                            .on_click(move |_, window, cx| {
                                window.close_dialog(cx);
                                designer_entity.update(cx, |designer, cx| {
                                    designer.save(window, cx);
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

// === Columns Editor ===

pub enum ColumnsEditorEvent {
    Changed,
}

/// 拖拽列时的视觉反馈
#[derive(Clone)]
struct DragColumn {
    index: usize,
    name: String,
}

impl DragColumn {
    fn new(index: usize, name: String) -> Self {
        Self { index, name }
    }
}

impl Render for DragColumn {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("drag-column")
            .cursor_grabbing()
            .py_1()
            .px_2()
            .rounded_md()
            .bg(cx.theme().primary.opacity(0.9))
            .text_color(cx.theme().primary_foreground)
            .text_sm()
            .child(if self.name.is_empty() {
                t!("Table.column_number", index = self.index + 1).to_string()
            } else {
                self.name.clone()
            })
    }
}

pub struct ColumnsEditor {
    focus_handle: FocusHandle,
    columns: Vec<ColumnEditorRow>,
    selected_index: Option<usize>,
    data_types: Vec<String>,
    charsets: Vec<CharsetInfo>,
    database_type: DatabaseType,
    column_editor_capabilities: ColumnEditorCapabilities,
    scroll_handle: UniformListScrollHandle,
    search_input: Entity<InputState>,
    search_query: String,
    filtered_indices: Vec<usize>,
    _search_subscription: Subscription,
    _subscriptions: Vec<Subscription>,
}

struct ColumnEditorRow {
    name_input: Entity<InputState>,
    type_select: Entity<SelectState<SearchableVec<String>>>,
    length_input: Entity<InputState>,
    scale_input: Entity<InputState>,
    nullable: bool,
    is_pk: bool,
    auto_increment: bool,
    default_input: Entity<InputState>,
    comment_input: Entity<InputState>,
    charset_select: Entity<SelectState<Vec<CharsetSelectItem>>>,
    collation_select: Entity<SelectState<Vec<CollationSelectItem>>>,
    enum_values_input: Entity<InputState>,
}

impl ColumnsEditor {
    pub fn new(
        database_type: DatabaseType,
        charsets: Vec<CharsetInfo>,
        column_editor_capabilities: ColumnEditorCapabilities,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let data_types = Self::get_data_types(&database_type, cx);
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("Table.search_column").to_string())
        });

        let search_sub = cx.subscribe_in(
            &search_input,
            window,
            |this, _, event: &InputEvent, _window, cx| {
                if let InputEvent::Change = event {
                    this.update_filtered_indices(cx);
                }
            },
        );

        Self {
            focus_handle,
            columns: vec![],
            selected_index: None,
            data_types,
            charsets,
            database_type,
            column_editor_capabilities,
            scroll_handle: UniformListScrollHandle::default(),
            search_input,
            search_query: String::new(),
            filtered_indices: vec![],
            _search_subscription: search_sub,
            _subscriptions: vec![],
        }
    }

    fn update_filtered_indices(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).text().to_string().to_lowercase();
        self.search_query = query.clone();

        if query.is_empty() {
            self.filtered_indices = (0..self.columns.len()).collect();
        } else {
            self.filtered_indices = self
                .columns
                .iter()
                .enumerate()
                .filter(|(_, row)| {
                    let name = row.name_input.read(cx).text().to_string().to_lowercase();
                    name.contains(&query)
                })
                .map(|(idx, _)| idx)
                .collect();

            if !self.filtered_indices.is_empty() {
                self.scroll_handle
                    .scroll_to_item(0, gpui::ScrollStrategy::Top);
                self.selected_index = Some(self.filtered_indices[0]);
            }
        }

        cx.notify();
    }

    fn get_data_types(database_type: &DatabaseType, cx: &App) -> Vec<String> {
        let global_state = cx.global::<GlobalDbState>();
        if let Ok(plugin) = global_state.db_manager.get_plugin(database_type) {
            plugin
                .get_data_types()
                .iter()
                .map(|(name, _)| name.to_string())
                .collect()
        } else {
            vec![
                "INT".to_string(),
                "VARCHAR".to_string(),
                "TEXT".to_string(),
                "DATE".to_string(),
                "DATETIME".to_string(),
            ]
        }
    }

    fn add_column(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("Table.column_name").to_string())
        });
        let type_items = SearchableVec::new(self.data_types.clone());
        let type_select = cx.new(|cx| {
            SelectState::new(type_items, Some(IndexPath::new(0)), window, cx).searchable(true)
        });
        let length_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("Table.length").to_string()));
        let scale_input =
            cx.new(|cx| {
                InputState::new(window, cx).placeholder(t!("Table.decimal_places").to_string())
            });
        let default_input =
            cx.new(|cx| {
                InputState::new(window, cx).placeholder(t!("Table.default_value").to_string())
            });
        let comment_input =
            cx.new(|cx| {
                InputState::new(window, cx).placeholder(t!("Table.comment").to_string())
            });

        let charset_items: Vec<CharsetSelectItem> = std::iter::once(CharsetSelectItem {
            info: CharsetInfo {
                name: "".to_string(),
                description: t!("Table.default").to_string(),
                default_collation: "".to_string(),
            },
        })
        .chain(
            self.charsets
                .iter()
                .cloned()
                .map(|info| CharsetSelectItem { info }),
        )
        .collect();
        let charset_select =
            cx.new(|cx| SelectState::new(charset_items, Some(IndexPath::new(0)), window, cx));

        let collation_select = cx.new(|cx| {
            let items = vec![CollationSelectItem {
                info: CollationInfo {
                    name: "".to_string(),
                    charset: "".to_string(),
                    is_default: true,
                },
            }];
            SelectState::new(items, Some(IndexPath::new(0)), window, cx)
        });

        let enum_values_input =
            cx.new(|cx| {
                InputState::new(window, cx).placeholder(t!("Table.value_list_hint").to_string())
            });

        let name_sub = cx.subscribe_in(
            &name_input,
            window,
            |_this, _, event: &InputEvent, _window, cx| {
                if let InputEvent::Change = event {
                    cx.emit(ColumnsEditorEvent::Changed);
                }
            },
        );
        let length_sub = cx.subscribe_in(
            &length_input,
            window,
            |_this, _, event: &InputEvent, _window, cx| {
                if let InputEvent::Change = event {
                    cx.emit(ColumnsEditorEvent::Changed);
                }
            },
        );
        let scale_sub = cx.subscribe_in(
            &scale_input,
            window,
            |_this, _, event: &InputEvent, _window, cx| {
                if let InputEvent::Change = event {
                    cx.emit(ColumnsEditorEvent::Changed);
                }
            },
        );
        let default_sub = cx.subscribe_in(
            &default_input,
            window,
            |_this, _, event: &InputEvent, _window, cx| {
                if let InputEvent::Change = event {
                    cx.emit(ColumnsEditorEvent::Changed);
                }
            },
        );
        let comment_sub = cx.subscribe_in(
            &comment_input,
            window,
            |_this, _, event: &InputEvent, _window, cx| {
                if let InputEvent::Change = event {
                    cx.emit(ColumnsEditorEvent::Changed);
                }
            },
        );
        let type_sub = cx.observe(&type_select, |_this, _, cx| {
            cx.emit(ColumnsEditorEvent::Changed);
        });
        let charset_sub = cx.observe(&charset_select, |_this, _, cx| {
            cx.emit(ColumnsEditorEvent::Changed);
        });
        let collation_sub = cx.observe(&collation_select, |_this, _, cx| {
            cx.emit(ColumnsEditorEvent::Changed);
        });
        let enum_values_sub = cx.subscribe_in(
            &enum_values_input,
            window,
            |_this, _, event: &InputEvent, _window, cx| {
                if let InputEvent::Change = event {
                    cx.emit(ColumnsEditorEvent::Changed);
                }
            },
        );

        self._subscriptions.extend([
            name_sub,
            length_sub,
            scale_sub,
            default_sub,
            comment_sub,
            type_sub,
            charset_sub,
            collation_sub,
            enum_values_sub,
        ]);

        self.columns.push(ColumnEditorRow {
            name_input,
            type_select,
            length_input,
            scale_input,
            nullable: true,
            is_pk: false,
            auto_increment: false,
            default_input,
            comment_input,
            charset_select,
            collation_select,
            enum_values_input,
        });

        let new_index = self.columns.len() - 1;
        self.selected_index = Some(new_index);
        self.update_filtered_indices(cx);

        if let Some(pos) = self.filtered_indices.iter().position(|&i| i == new_index) {
            self.scroll_handle
                .scroll_to_item(pos, gpui::ScrollStrategy::Top);
        }

        cx.emit(ColumnsEditorEvent::Changed);
        cx.notify();
    }

    fn remove_column(&mut self, cx: &mut Context<Self>) {
        if let Some(idx) = self.selected_index {
            if idx < self.columns.len() {
                self.columns.remove(idx);
                self.selected_index = None;
                self.update_filtered_indices(cx);
                cx.emit(ColumnsEditorEvent::Changed);
                cx.notify();
            }
        }
    }

    fn toggle_nullable(&mut self, idx: usize, cx: &mut Context<Self>) {
        if let Some(col) = self.columns.get_mut(idx) {
            col.nullable = !col.nullable;
            cx.emit(ColumnsEditorEvent::Changed);
            cx.notify();
        }
    }

    fn toggle_pk(&mut self, idx: usize, cx: &mut Context<Self>) {
        if let Some(col) = self.columns.get_mut(idx) {
            col.is_pk = !col.is_pk;
            cx.emit(ColumnsEditorEvent::Changed);
            cx.notify();
        }
    }

    fn toggle_auto_increment(&mut self, idx: usize, cx: &mut Context<Self>) {
        if let Some(col) = self.columns.get_mut(idx) {
            col.auto_increment = !col.auto_increment;
            cx.emit(ColumnsEditorEvent::Changed);
            cx.notify();
        }
    }

    fn move_column(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if from == to || from >= self.columns.len() || to >= self.columns.len() {
            return;
        }
        let column = self.columns.remove(from);
        self.columns.insert(to, column);

        if let Some(selected) = self.selected_index {
            if selected == from {
                self.selected_index = Some(to);
            } else if from < selected && selected <= to {
                self.selected_index = Some(selected - 1);
            } else if to <= selected && selected < from {
                self.selected_index = Some(selected + 1);
            }
        }

        cx.emit(ColumnsEditorEvent::Changed);
        cx.notify();
    }

    pub fn get_columns(&self, cx: &App) -> Vec<ColumnDefinition> {
        self.columns
            .iter()
            .map(|row| {
                let name = row.name_input.read(cx).text().to_string();
                let base_type = row
                    .type_select
                    .read(cx)
                    .selected_value()
                    .cloned()
                    .unwrap_or_else(|| "VARCHAR".to_string());
                let length_str = row.length_input.read(cx).text().to_string();
                let length = length_str.parse::<u32>().ok();
                let scale_str = row.scale_input.read(cx).text().to_string();
                let scale = scale_str.parse::<u32>().ok();

                let data_type = if self.column_editor_capabilities.supports_enum_values
                    && self.is_enum_type(&base_type, cx)
                {
                    let enum_values = row.enum_values_input.read(cx).text().to_string();
                    if enum_values.is_empty() {
                        base_type
                    } else {
                        format!("{}({})", base_type, enum_values)
                    }
                } else {
                    base_type
                };

                let default_value = {
                    let val = row.default_input.read(cx).text().to_string();
                    if val.is_empty() { None } else { Some(val) }
                };
                let comment = row.comment_input.read(cx).text().to_string();
                let charset = row
                    .charset_select
                    .read(cx)
                    .selected_value()
                    .cloned()
                    .filter(|s| !s.is_empty());
                let collation = row
                    .collation_select
                    .read(cx)
                    .selected_value()
                    .cloned()
                    .filter(|s| !s.is_empty());

                ColumnDefinition {
                    name,
                    data_type,
                    length,
                    precision: None,
                    scale,
                    is_nullable: row.nullable,
                    is_primary_key: row.is_pk,
                    is_auto_increment: row.auto_increment,
                    is_unsigned: false,
                    default_value,
                    comment,
                    charset,
                    collation,
                }
            })
            .collect()
    }

    fn select_row(&mut self, idx: usize, cx: &mut Context<Self>) {
        self.selected_index = Some(idx);
        cx.notify();
    }

    pub fn load_columns(
        &mut self,
        columns: Vec<ColumnInfo>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.columns.clear();
        self._subscriptions.clear();

        let global_state = cx.global::<GlobalDbState>();
        let plugin = global_state.db_manager.get_plugin(&self.database_type).ok();

        for col in columns {
            let name_input = cx.new(|cx| {
                let mut input =
                    InputState::new(window, cx).placeholder(t!("Table.column_name").to_string());
                input.set_value(col.name.clone(), window, cx);
                input
            });

            let select_type_items = SearchableVec::new(self.data_types.clone());
            let base_type = plugin
                .as_ref()
                .map(|p| p.parse_column_type(&col.data_type).base_type)
                .unwrap_or_else(|| col.data_type.clone());
            let type_idx = self
                .data_types
                .iter()
                .position(|t| t.to_uppercase() == base_type.to_uppercase())
                .unwrap_or(0);
            let type_select = cx.new(|cx| {
                SelectState::new(
                    select_type_items,
                    Some(IndexPath::new(type_idx)),
                    window,
                    cx,
                )
            });

            let length_input = cx.new(|cx| {
                let mut input =
                    InputState::new(window, cx).placeholder(t!("Table.length").to_string());
                if let Some(len) = Self::extract_length_from_type(&col.data_type) {
                    input.set_value(len.to_string(), window, cx);
                }
                input
            });

            let scale_input = cx.new(|cx| {
                let mut input =
                    InputState::new(window, cx).placeholder(t!("Table.decimal_places").to_string());
                if let Some(scale) = Self::extract_scale_from_type(&col.data_type) {
                    input.set_value(scale.to_string(), window, cx);
                }
                input
            });

            let default_input = cx.new(|cx| {
                let mut input =
                    InputState::new(window, cx).placeholder(t!("Table.default_value").to_string());
                if let Some(ref default) = col.default_value {
                    input.set_value(default.clone(), window, cx);
                }
                input
            });

            let comment_input = cx.new(|cx| {
                let mut input =
                    InputState::new(window, cx).placeholder(t!("Table.comment").to_string());
                if let Some(ref comment) = col.comment {
                    input.set_value(comment.clone(), window, cx);
                }
                input
            });

            let charset_items: Vec<CharsetSelectItem> = std::iter::once(CharsetSelectItem {
                info: CharsetInfo {
                    name: "".to_string(),
                    description: t!("Table.default").to_string(),
                    default_collation: "".to_string(),
                },
            })
            .chain(
                self.charsets
                    .iter()
                    .cloned()
                    .map(|info| CharsetSelectItem { info }),
            )
            .collect();
            let charset_select =
                cx.new(|cx| SelectState::new(charset_items, Some(IndexPath::new(0)), window, cx));

            let collation_select = cx.new(|cx| {
                let items = vec![CollationSelectItem {
                    info: CollationInfo {
                        name: "".to_string(),
                        charset: "".to_string(),
                        is_default: true,
                    },
                }];
                SelectState::new(items, Some(IndexPath::new(0)), window, cx)
            });

            let enum_values_input = cx.new(|cx| {
                let mut input =
                    InputState::new(window, cx).placeholder(t!("Table.value_list_hint").to_string());
                if let Some(values) = Self::extract_enum_values(&col.data_type) {
                    input.set_value(values, window, cx);
                }
                input
            });

            let name_sub = cx.subscribe_in(
                &name_input,
                window,
                |_this, _, event: &InputEvent, _window, cx| {
                    if let InputEvent::Change = event {
                        cx.emit(ColumnsEditorEvent::Changed);
                    }
                },
            );
            let length_sub = cx.subscribe_in(
                &length_input,
                window,
                |_this, _, event: &InputEvent, _window, cx| {
                    if let InputEvent::Change = event {
                        cx.emit(ColumnsEditorEvent::Changed);
                    }
                },
            );
            let scale_sub = cx.subscribe_in(
                &scale_input,
                window,
                |_this, _, event: &InputEvent, _window, cx| {
                    if let InputEvent::Change = event {
                        cx.emit(ColumnsEditorEvent::Changed);
                    }
                },
            );
            let default_sub = cx.subscribe_in(
                &default_input,
                window,
                |_this, _, event: &InputEvent, _window, cx| {
                    if let InputEvent::Change = event {
                        cx.emit(ColumnsEditorEvent::Changed);
                    }
                },
            );
            let comment_sub = cx.subscribe_in(
                &comment_input,
                window,
                |_this, _, event: &InputEvent, _window, cx| {
                    if let InputEvent::Change = event {
                        cx.emit(ColumnsEditorEvent::Changed);
                    }
                },
            );
            let type_sub = cx.observe(&type_select, |_this, _, cx| {
                cx.emit(ColumnsEditorEvent::Changed);
            });
            let charset_sub = cx.observe(&charset_select, |_this, _, cx| {
                cx.emit(ColumnsEditorEvent::Changed);
            });
            let collation_sub = cx.observe(&collation_select, |_this, _, cx| {
                cx.emit(ColumnsEditorEvent::Changed);
            });
            let enum_values_sub = cx.subscribe_in(
                &enum_values_input,
                window,
                |_this, _, event: &InputEvent, _window, cx| {
                    if let InputEvent::Change = event {
                        cx.emit(ColumnsEditorEvent::Changed);
                    }
                },
            );

            self._subscriptions.extend([
                name_sub,
                length_sub,
                scale_sub,
                default_sub,
                comment_sub,
                type_sub,
                charset_sub,
                collation_sub,
                enum_values_sub,
            ]);

            self.columns.push(ColumnEditorRow {
                name_input,
                type_select,
                length_input,
                scale_input,
                nullable: col.is_nullable,
                is_pk: col.is_primary_key,
                auto_increment: col.data_type.to_uppercase().contains("AUTO_INCREMENT"),
                default_input,
                comment_input,
                charset_select,
                collation_select,
                enum_values_input,
            });
        }

        self.update_filtered_indices(cx);
        cx.emit(ColumnsEditorEvent::Changed);
        cx.notify();
    }

    fn extract_length_from_type(data_type: &str) -> Option<u32> {
        if let Some(start) = data_type.find('(') {
            if let Some(end) = data_type.find(')') {
                let len_str = &data_type[start + 1..end];
                if let Some(comma) = len_str.find(',') {
                    return len_str[..comma].trim().parse().ok();
                }
                return len_str.trim().parse().ok();
            }
        }
        None
    }

    fn extract_scale_from_type(data_type: &str) -> Option<u32> {
        if let Some(start) = data_type.find('(') {
            if let Some(end) = data_type.find(')') {
                let len_str = &data_type[start + 1..end];
                if let Some(comma) = len_str.find(',') {
                    return len_str[comma + 1..].trim().parse().ok();
                }
            }
        }
        None
    }

    fn extract_enum_values(data_type: &str) -> Option<String> {
        let upper = data_type.to_uppercase();
        if !upper.starts_with("ENUM") && !upper.starts_with("SET") {
            return None;
        }
        if let Some(start) = data_type.find('(') {
            if let Some(end) = data_type.rfind(')') {
                let values = &data_type[start + 1..end];
                return Some(values.to_string());
            }
        }
        None
    }

    fn is_enum_type(&self, data_type: &str, cx: &App) -> bool {
        let global_state = cx.global::<GlobalDbState>();
        if let Ok(plugin) = global_state.db_manager.get_plugin(&self.database_type) {
            plugin.is_enum_type(data_type)
        } else {
            false
        }
    }

    fn render_header(&self, cx: &Context<Self>) -> AnyElement {
        h_flex()
            .gap_1()
            .items_center()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("add-col")
                    .small()
                    .icon(IconName::Plus)
                    .ghost()
                    .tooltip(t!("Table.add_column").to_string())
                    .on_click(cx.listener(|this, _, window, cx| this.add_column(window, cx))),
            )
            .child(
                Button::new("remove-col")
                    .small()
                    .icon(IconName::Minus)
                    .ghost()
                    .tooltip(t!("Table.delete_column").to_string())
                    .on_click(cx.listener(|this, _, _window, cx| this.remove_column(cx))),
            )
            .child(div().flex_1())
            .child(
                Input::new(&self.search_input)
                    .small()
                    .w(px(200.))
                    .prefix(
                        Icon::new(IconName::Search)
                            .with_size(Size::Small)
                            .text_color(cx.theme().muted_foreground),
                    )
                    .cleanable(true),
            )
            .into_any_element()
    }

    fn render_table_header(&self, cx: &Context<Self>) -> AnyElement {
        h_flex()
            .gap_3()
            .px_3()
            .py_2()
            .bg(cx.theme().muted.opacity(0.5))
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().w(px(24.)))
            .child(
                div()
                    .w(px(160.))
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("Table.column_name").to_string()),
            )
            .child(
                div()
                    .w(px(140.))
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("Table.type").to_string()),
            )
            .child(
                div()
                    .w(px(60.))
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("Table.length").to_string()),
            )
            .child(
                div()
                    .w(px(60.))
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("Table.decimal_places").to_string()),
            )
            .child(
                div()
                    .w(px(50.))
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .text_center()
                    .child(t!("Table.nullable").to_string()),
            )
            .child(
                div()
                    .w(px(50.))
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .text_center()
                    .child(t!("Table.primary_key").to_string()),
            )
            .child(
                div()
                    .w(px(50.))
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .text_center()
                    .child(t!("Table.auto_increment_column").to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("Table.comment").to_string()),
            )
            .into_any_element()
    }

    fn render_row(&self, idx: usize, row: &ColumnEditorRow, cx: &Context<Self>) -> AnyElement {
        let is_selected = self.selected_index == Some(idx);
        let name = row.name_input.read(cx).text().to_string();
        let drag_border_color = cx.theme().primary;

        h_flex()
            .id(("col-row", idx))
            .w_full()
            .gap_3()
            .px_3()
            .py_1p5()
            .when(is_selected, |this| this.bg(cx.theme().primary.opacity(0.1)))
            .hover(|this| this.bg(cx.theme().muted.opacity(0.3)))
            .border_b_1()
            .border_color(cx.theme().border.opacity(0.5))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _window, cx| {
                    this.select_row(idx, cx);
                }),
            )
            .on_drag(DragColumn::new(idx, name), |drag, _, _, cx| {
                cx.new(|_| drag.clone())
            })
            .drag_over::<DragColumn>(move |el, _, _, _cx| {
                el.border_t_2().border_color(drag_border_color)
            })
            .on_drop(cx.listener(move |this, drag: &DragColumn, _window, cx| {
                this.move_column(drag.index, idx, cx);
            }))
            .child(
                div()
                    .w(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_grab()
                    .child(
                        Icon::new(IconName::Menu)
                            .with_size(Size::Small)
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            .child(
                div()
                    .w(px(160.))
                    .child(Input::new(&row.name_input).w_full().small()),
            )
            .child(
                div()
                    .w(px(140.))
                    .child(Select::new(&row.type_select).w_full().small()),
            )
            .child(
                div()
                    .w(px(60.))
                    .child(Input::new(&row.length_input).w_full().small()),
            )
            .child(
                div()
                    .w(px(60.))
                    .child(Input::new(&row.scale_input).w_full().small()),
            )
            .child(
                div().w(px(50.)).flex().justify_center().child(
                    Checkbox::new(("null", idx))
                        .checked(row.nullable)
                        .small()
                        .on_click(
                            cx.listener(move |this, _, _window, cx| this.toggle_nullable(idx, cx)),
                        ),
                ),
            )
            .child(
                div().w(px(50.)).flex().justify_center().child(
                    Checkbox::new(("pk", idx))
                        .checked(row.is_pk)
                        .small()
                        .on_click(cx.listener(move |this, _, _window, cx| this.toggle_pk(idx, cx))),
                ),
            )
            .child(
                div().w(px(50.)).flex().justify_center().child(
                    Checkbox::new(("ai", idx))
                        .checked(row.auto_increment)
                        .small()
                        .on_click(cx.listener(move |this, _, _window, cx| {
                            this.toggle_auto_increment(idx, cx)
                        })),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .child(Input::new(&row.comment_input).w_full().small()),
            )
            .into_any_element()
    }

    fn render_detail_panel(&self, cx: &Context<Self>) -> AnyElement {
        let Some(idx) = self.selected_index else {
            return v_flex()
                .items_center()
                .justify_center()
                .h(px(180.))
                .border_t_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().background)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t!("Table.select_column_hint").to_string()),
                )
                .into_any_element();
        };

        let Some(row) = self.columns.get(idx) else {
            return div().into_any_element();
        };

        let selected_type = row
            .type_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_default();
        let is_enum_or_set = self.column_editor_capabilities.supports_enum_values
            && self.is_enum_type(&selected_type, cx);
        let show_charset = self.column_editor_capabilities.show_charset_in_detail;
        let show_collation = self.column_editor_capabilities.show_collation_in_detail;

        v_flex()
            .w_full()
            .items_center()
            .justify_center()
            .h(px(180.))
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .p_3()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .w(px(70.))
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{}:", t!("Table.default_value"))),
                    )
                    .child(Input::new(&row.default_input).w(px(200.)).small()),
            )
            .when(show_charset, |this| {
                this.child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .w(px(70.))
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{}:", t!("Table.charset"))),
                        )
                        .child(Select::new(&row.charset_select).w(px(200.)).small()),
                )
            })
            .when(show_collation, |this| {
                this.child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .w(px(70.))
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{}:", t!("Table.collation"))),
                        )
                        .child(Select::new(&row.collation_select).w(px(200.)).small()),
                )
            })
            .when(is_enum_or_set, |this| {
                this.child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{}:", t!("Table.value"))),
                        )
                        .child(Input::new(&row.enum_values_input).w(px(400.)).small()),
                )
            })
            .into_any_element()
    }
}

impl EventEmitter<ColumnsEditorEvent> for ColumnsEditor {}

impl Focusable for ColumnsEditor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ColumnsEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let filtered_count = self.filtered_indices.len();
        let scroll_handle = self.scroll_handle.clone();

        v_flex()
            .size_full()
            .child(self.render_header(cx))
            .child(self.render_table_header(cx))
            .child(
                div()
                    .id("columns-list-container")
                    .flex_1()
                    .overflow_hidden()
                    .relative()
                    .child(
                        uniform_list("columns-list", filtered_count, {
                            cx.processor(move |editor, visible_range: Range<usize>, _window, cx| {
                                visible_range
                                    .filter_map(|pos| {
                                        let actual_idx =
                                            editor.filtered_indices.get(pos).copied()?;
                                        let row = editor.columns.get(actual_idx)?;
                                        Some(editor.render_row(actual_idx, row, cx))
                                    })
                                    .collect::<Vec<_>>()
                            })
                        })
                        .flex_grow()
                        .size_full()
                        .track_scroll(&scroll_handle)
                        .with_sizing_behavior(ListSizingBehavior::Auto)
                        .into_any_element(),
                    )
                    .child(Scrollbar::vertical(&self.scroll_handle)),
            )
            .child(self.render_detail_panel(cx))
    }
}

// === Indexes Editor ===

pub enum IndexesEditorEvent {
    Changed,
}

pub struct IndexesEditor {
    focus_handle: FocusHandle,
    indexes: Vec<IndexEditorRow>,
    selected_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

struct IndexEditorRow {
    name_input: Entity<InputState>,
    columns_input: Entity<InputState>,
    is_unique: bool,
}

impl IndexesEditor {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        Self {
            focus_handle,
            indexes: vec![],
            selected_index: None,
            _subscriptions: vec![],
        }
    }

    fn add_index(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name_input =
            cx.new(|cx| {
                InputState::new(window, cx).placeholder(t!("Table.index_name").to_string())
            });
        let columns_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("Table.columns_comma_separated").to_string())
        });

        // Subscribe to input changes
        let name_sub = cx.subscribe_in(
            &name_input,
            window,
            |_this, _, event: &InputEvent, _window, cx| {
                if let InputEvent::Change = event {
                    cx.emit(IndexesEditorEvent::Changed);
                }
            },
        );
        let columns_sub = cx.subscribe_in(
            &columns_input,
            window,
            |_this, _, event: &InputEvent, _window, cx| {
                if let InputEvent::Change = event {
                    cx.emit(IndexesEditorEvent::Changed);
                }
            },
        );

        self._subscriptions.extend([name_sub, columns_sub]);

        self.indexes.push(IndexEditorRow {
            name_input,
            columns_input,
            is_unique: false,
        });

        cx.emit(IndexesEditorEvent::Changed);
        cx.notify();
    }

    fn remove_index(&mut self, cx: &mut Context<Self>) {
        if let Some(idx) = self.selected_index {
            if idx < self.indexes.len() {
                self.indexes.remove(idx);
                self.selected_index = None;
                cx.emit(IndexesEditorEvent::Changed);
                cx.notify();
            }
        }
    }

    fn toggle_unique(&mut self, idx: usize, cx: &mut Context<Self>) {
        if let Some(index) = self.indexes.get_mut(idx) {
            index.is_unique = !index.is_unique;
            cx.emit(IndexesEditorEvent::Changed);
            cx.notify();
        }
    }

    fn select_row(&mut self, idx: usize, cx: &mut Context<Self>) {
        self.selected_index = Some(idx);
        cx.notify();
    }

    pub fn get_indexes(&self, cx: &App) -> Vec<IndexDefinition> {
        self.indexes
            .iter()
            .map(|row| {
                let name = row.name_input.read(cx).text().to_string();
                let columns_str = row.columns_input.read(cx).text().to_string();
                let columns: Vec<String> = columns_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                IndexDefinition {
                    name,
                    columns,
                    is_unique: row.is_unique,
                    is_primary: false,
                    index_type: None,
                    comment: String::new(),
                }
            })
            .collect()
    }

    pub fn load_indexes(
        &mut self,
        indexes: Vec<IndexInfo>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.indexes.clear();
        self._subscriptions.clear();

        for idx in indexes {
            if idx.name.to_uppercase() == "PRIMARY" {
                continue;
            }

            let name_input = cx.new(|cx| {
                let mut input =
                    InputState::new(window, cx).placeholder(t!("Table.index_name").to_string());
                input.set_value(idx.name.clone(), window, cx);
                input
            });

            let columns_str = idx.columns.join(", ");
            let columns_input = cx.new(|cx| {
                let mut input =
                    InputState::new(window, cx)
                        .placeholder(t!("Table.columns_comma_separated").to_string());
                input.set_value(columns_str, window, cx);
                input
            });

            let name_sub = cx.subscribe_in(
                &name_input,
                window,
                |_this, _, event: &InputEvent, _window, cx| {
                    if let InputEvent::Change = event {
                        cx.emit(IndexesEditorEvent::Changed);
                    }
                },
            );
            let columns_sub = cx.subscribe_in(
                &columns_input,
                window,
                |_this, _, event: &InputEvent, _window, cx| {
                    if let InputEvent::Change = event {
                        cx.emit(IndexesEditorEvent::Changed);
                    }
                },
            );

            self._subscriptions.extend([name_sub, columns_sub]);

            self.indexes.push(IndexEditorRow {
                name_input,
                columns_input,
                is_unique: idx.is_unique,
            });
        }

        cx.emit(IndexesEditorEvent::Changed);
        cx.notify();
    }
}

impl EventEmitter<IndexesEditorEvent> for IndexesEditor {}

impl Focusable for IndexesEditor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for IndexesEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("add-idx")
                            .small()
                            .icon(IconName::Plus)
                            .ghost()
                            .tooltip(t!("Table.add_index").to_string())
                            .on_click(
                                cx.listener(|this, _, window, cx| this.add_index(window, cx)),
                            ),
                    )
                    .child(
                        Button::new("remove-idx")
                            .small()
                            .icon(IconName::Minus)
                            .ghost()
                            .tooltip(t!("Table.delete_index").to_string())
                            .on_click(cx.listener(|this, _, _window, cx| this.remove_index(cx))),
                    ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .bg(cx.theme().muted.opacity(0.5))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .w(px(160.))
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("Table.index_name").to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("Table.columns").to_string()),
                    )
                    .child(
                        div()
                            .w(px(60.))
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .text_center()
                            .child(t!("Table.unique").to_string()),
                    ),
            )
            .child(
                v_flex()
                    .id("indexes-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .children(self.indexes.iter().enumerate().map(|(idx, row)| {
                        let is_selected = self.selected_index == Some(idx);
                        h_flex()
                            .id(("idx-row", idx))
                            .gap_3()
                            .px_3()
                            .py_1p5()
                            .when(is_selected, |this| this.bg(cx.theme().primary.opacity(0.1)))
                            .hover(|this| this.bg(cx.theme().muted.opacity(0.3)))
                            .border_b_1()
                            .border_color(cx.theme().border.opacity(0.5))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _window, cx| {
                                    this.select_row(idx, cx);
                                }),
                            )
                            .child(
                                div()
                                    .w(px(160.))
                                    .child(Input::new(&row.name_input).w_full().small()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .child(Input::new(&row.columns_input).w_full().small()),
                            )
                            .child(
                                div().w(px(60.)).flex().justify_center().child(
                                    Checkbox::new(("unique", idx))
                                        .checked(row.is_unique)
                                        .small()
                                        .on_click(cx.listener(move |this, _, _window, cx| {
                                            this.toggle_unique(idx, cx)
                                        })),
                                ),
                            )
                    })),
            )
    }
}

// === Table Options Editor ===

pub enum TableOptionsEvent {
    Changed,
}

#[derive(Clone, Debug)]
pub struct EngineSelectItem {
    pub name: String,
}

impl SelectItem for EngineSelectItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.name.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }
}

#[derive(Clone, Debug)]
pub struct CharsetSelectItem {
    pub info: CharsetInfo,
}

impl SelectItem for CharsetSelectItem {
    type Value = String;

    fn title(&self) -> SharedString {
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

impl SelectItem for CollationSelectItem {
    type Value = String;

    fn title(&self) -> SharedString {
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

pub struct TableOptionsEditor {
    focus_handle: FocusHandle,
    _database_type: DatabaseType,
    engine_select: Entity<SelectState<Vec<EngineSelectItem>>>,
    charset_select: Entity<SelectState<Vec<CharsetSelectItem>>>,
    collation_select: Entity<SelectState<Vec<CollationSelectItem>>>,
    comment_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl TableOptionsEditor {
    pub fn new(database_type: DatabaseType, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let engines = vec![
            EngineSelectItem {
                name: "InnoDB".to_string(),
            },
            EngineSelectItem {
                name: "MyISAM".to_string(),
            },
            EngineSelectItem {
                name: "MEMORY".to_string(),
            },
        ];
        let engine_select =
            cx.new(|cx| SelectState::new(engines, Some(IndexPath::new(0)), window, cx));

        let charsets = Self::get_charsets(&database_type, cx);
        let charset_items: Vec<CharsetSelectItem> = charsets
            .iter()
            .cloned()
            .map(|info| CharsetSelectItem { info })
            .collect();
        let charset_select =
            cx.new(|cx| SelectState::new(charset_items, Some(IndexPath::new(0)), window, cx));

        let default_charset = charsets
            .first()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "utf8mb4".to_string());
        let collations = Self::get_collations(&database_type, &default_charset, cx);
        let collation_items: Vec<CollationSelectItem> = collations
            .iter()
            .cloned()
            .map(|info| CollationSelectItem { info })
            .collect();
        let default_coll_idx = collation_items
            .iter()
            .position(|c| c.info.is_default)
            .unwrap_or(0);
        let collation_select = cx.new(|cx| {
            SelectState::new(
                collation_items,
                Some(IndexPath::new(default_coll_idx)),
                window,
                cx,
            )
        });

        let comment_input =
            cx.new(|cx| {
                InputState::new(window, cx).placeholder(t!("Table.table_comment").to_string())
            });

        let engine_sub = cx.observe(&engine_select, |_this, _, cx| {
            cx.emit(TableOptionsEvent::Changed);
        });
        let charset_sub = cx.observe(&charset_select, |_this, _, cx| {
            cx.emit(TableOptionsEvent::Changed);
        });
        let collation_sub = cx.observe(&collation_select, |_this, _, cx| {
            cx.emit(TableOptionsEvent::Changed);
        });
        let comment_sub = cx.subscribe_in(
            &comment_input,
            window,
            |_this, _, event: &InputEvent, _window, cx| {
                if let InputEvent::Change = event {
                    cx.emit(TableOptionsEvent::Changed);
                }
            },
        );

        Self {
            focus_handle,
            _database_type: database_type,
            engine_select,
            charset_select,
            collation_select,
            comment_input,
            _subscriptions: vec![engine_sub, charset_sub, collation_sub, comment_sub],
        }
    }

    fn get_charsets(database_type: &DatabaseType, cx: &App) -> Vec<CharsetInfo> {
        let global_state = cx.global::<GlobalDbState>();
        if let Ok(plugin) = global_state.db_manager.get_plugin(database_type) {
            plugin.get_charsets()
        } else {
            vec![CharsetInfo {
                name: "utf8mb4".to_string(),
                description: "UTF-8 Unicode".to_string(),
                default_collation: "utf8mb4_general_ci".to_string(),
            }]
        }
    }

    fn get_collations(database_type: &DatabaseType, charset: &str, cx: &App) -> Vec<CollationInfo> {
        let global_state = cx.global::<GlobalDbState>();
        if let Ok(plugin) = global_state.db_manager.get_plugin(database_type) {
            plugin.get_collations(charset)
        } else {
            vec![CollationInfo {
                name: "utf8mb4_general_ci".to_string(),
                charset: "utf8mb4".to_string(),
                is_default: true,
            }]
        }
    }

    pub fn get_options(&self, cx: &App) -> TableOptions {
        let engine = self.engine_select.read(cx).selected_value().cloned();
        let charset = self.charset_select.read(cx).selected_value().cloned();
        let collation = self.collation_select.read(cx).selected_value().cloned();
        let comment = self.comment_input.read(cx).text().to_string();

        TableOptions {
            engine,
            charset,
            collation,
            comment,
            auto_increment: None,
        }
    }
}

impl EventEmitter<TableOptionsEvent> for TableOptionsEditor {}

impl Focusable for TableOptionsEditor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TableOptionsEditor {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().size_full().p_4().gap_4().child(
            h_form()
                .with_size(Size::Small)
                .columns(1)
                .label_width(px(80.))
                .child(
                    field()
                        .label(t!("Table.engine").to_string())
                        .items_center()
                        .label_justify_end()
                        .child(Select::new(&self.engine_select).w(px(200.))),
                )
                .child(
                    field()
                        .label(t!("Table.charset").to_string())
                        .items_center()
                        .label_justify_end()
                        .child(Select::new(&self.charset_select).w(px(200.))),
                )
                .child(
                    field()
                        .label(t!("Table.collation").to_string())
                        .items_center()
                        .label_justify_end()
                        .child(Select::new(&self.collation_select).w(px(200.))),
                )
                .child(
                    field()
                        .label(t!("Table.table_comment").to_string())
                        .items_center()
                        .label_justify_end()
                        .child(Input::new(&self.comment_input).w(px(300.))),
                ),
        )
    }
}
