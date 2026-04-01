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
    clipboard::Clipboard,
    dialog::DialogButtonProps,
    form::{field, h_form},
    h_flex,
    highlighter::Language,
    input::{Input, InputEvent, InputState},
    scroll::Scrollbar,
    select::{Select, SelectEvent, SelectItem, SelectState},
    tab::{Tab, TabBar},
    v_flex,
};
use std::collections::HashSet;
use std::ops::Range;
use std::sync::{Arc, Mutex};

use crate::database_view_plugin::{ColumnEditorCapabilities, DatabaseViewPluginRegistry};
use db::GlobalDbState;
#[cfg(test)]
use db::duckdb::DuckDbPlugin;
use db::plugin::DatabasePlugin;
use db::types::{
    CharsetInfo, CollationInfo, ColumnDefinition, ColumnInfo, IndexDefinition, IndexInfo,
    ParsedColumnType, TableDesign, TableOptions,
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
    Ddl,
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

pub(crate) fn build_table_design_from_metadata(
    database_type: DatabaseType,
    database_name: String,
    table_name: String,
    columns: &[ColumnInfo],
    indexes: &[IndexInfo],
    plugin: Option<&dyn DatabasePlugin>,
) -> TableDesign {
    let column_defs: Vec<ColumnDefinition> = columns
        .iter()
        .map(|col| {
            let parsed = plugin
                .map(|plugin| plugin.parse_column_type(&col.data_type))
                .unwrap_or_else(|| fallback_parse_column_type(&col.data_type));
            column_info_to_definition(database_type, col, parsed)
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
        database_name,
        table_name,
        columns: column_defs,
        indexes: index_defs,
        foreign_keys: vec![],
        options: TableOptions::default(),
    }
}

fn column_info_to_definition(
    database_type: DatabaseType,
    col: &ColumnInfo,
    parsed: ParsedColumnType,
) -> ColumnDefinition {
    let base_type = parsed.base_type;
    let data_type = if let Some(enum_values) = parsed.enum_values {
        format!("{}({})", base_type, enum_values)
    } else {
        base_type.clone()
    };
    let is_auto_increment = if matches!(database_type, DatabaseType::SQLite) {
        col.is_primary_key && base_type.eq_ignore_ascii_case("INTEGER")
    } else {
        parsed.is_auto_increment
    };

    ColumnDefinition {
        name: col.name.clone(),
        data_type,
        length: parsed.length,
        precision: None,
        scale: parsed.scale,
        is_nullable: col.is_nullable,
        is_primary_key: col.is_primary_key,
        is_auto_increment,
        is_unsigned: parsed.is_unsigned,
        default_value: col.default_value.clone(),
        comment: col.comment.clone().unwrap_or_default(),
        charset: col.charset.clone(),
        collation: col.collation.clone(),
    }
}

fn fallback_parse_column_type(data_type: &str) -> ParsedColumnType {
    let (base_type, length) = parse_data_type(data_type);
    ParsedColumnType {
        base_type,
        length,
        scale: extract_scale_from_type_str(data_type),
        enum_values: None,
        is_unsigned: data_type.to_uppercase().contains("UNSIGNED"),
        is_auto_increment: data_type.to_uppercase().contains("AUTO_INCREMENT"),
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
    ddl_preview_input: Entity<InputState>,
    preview_refresh_state: PreviewRefreshScheduleState,
    original_design: Option<TableDesign>,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone)]
enum ExecuteSuccessBehavior {
    StayOpen {
        tab_id: Option<String>,
    },
    CloseTab {
        tab_container: Entity<TabContainer>,
        tab_id: String,
        emitted_tab_id: Option<String>,
    },
}

#[derive(Clone)]
struct TableDesignerExecutionRequest {
    connection_id: String,
    database_name: String,
    schema_name: Option<String>,
    sql: String,
    table_name: String,
    is_new_table: bool,
    success_behavior: ExecuteSuccessBehavior,
}

#[derive(Default)]
struct PreviewRefreshScheduleState {
    refresh_pending: bool,
}

impl PreviewRefreshScheduleState {
    fn request_refresh(&mut self) -> bool {
        if self.refresh_pending {
            return false;
        }
        self.refresh_pending = true;
        true
    }

    fn finish_refresh(&mut self) {
        self.refresh_pending = false;
    }
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
        let ddl_preview_input = cx.new(|cx| {
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
                    this.schedule_preview_refresh(window, cx);
                }
            },
        );

        let comment_sub = cx.subscribe_in(
            &table_comment_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::Change = event {
                    this.schedule_preview_refresh(window, cx);
                }
            },
        );

        let auto_inc_sub = cx.subscribe_in(
            &auto_increment_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::Change = event {
                    this.schedule_preview_refresh(window, cx);
                }
            },
        );

        let engine_sub = cx.observe_in(&engine_select, window, |this, _, window, cx| {
            this.schedule_preview_refresh(window, cx);
        });

        let charset_select_clone = charset_select.clone();
        let charset_sub = cx.observe_in(&charset_select, window, move |this, _, window, cx| {
            this.schedule_preview_refresh(window, cx);
            this.update_collations_for_charset(&charset_select_clone, window, cx);
        });

        let collation_sub = cx.observe_in(&collation_select, window, |this, _, window, cx| {
            this.schedule_preview_refresh(window, cx);
        });

        let cols_sub = cx.subscribe_in(
            &columns_editor,
            window,
            |this, _, _: &ColumnsEditorEvent, window, cx| {
                this.schedule_preview_refresh(window, cx);
            },
        );

        let idx_sub = cx.subscribe_in(
            &indexes_editor,
            window,
            |this, _, _: &IndexesEditorEvent, window, cx| {
                this.schedule_preview_refresh(window, cx);
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
            ddl_preview_input,
            preview_refresh_state: PreviewRefreshScheduleState::default(),
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

        designer.update_previews(window, cx);

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

    fn collect_column_renames(&self, cx: &App) -> Vec<(String, String)> {
        self.columns_editor.read(cx).get_column_renames(cx)
    }

    fn normalize_column_renames(
        original: &TableDesign,
        design: &TableDesign,
        column_renames: &[(String, String)],
    ) -> Vec<(String, String)> {
        let original_names: HashSet<&str> = original
            .columns
            .iter()
            .map(|col| col.name.as_str())
            .collect();
        let current_names: HashSet<&str> =
            design.columns.iter().map(|col| col.name.as_str()).collect();
        let mut seen_old = HashSet::new();
        let mut seen_new = HashSet::new();

        column_renames
            .iter()
            .filter_map(|(old_name, new_name)| {
                if old_name.is_empty() || new_name.is_empty() || old_name == new_name {
                    return None;
                }
                if !original_names.contains(old_name.as_str())
                    || !current_names.contains(new_name.as_str())
                {
                    return None;
                }
                if seen_old.contains(old_name) || seen_new.contains(new_name) {
                    return None;
                }
                seen_old.insert(old_name.clone());
                seen_new.insert(new_name.clone());
                Some((old_name.clone(), new_name.clone()))
            })
            .collect()
    }

    fn build_diff_preview_sql(
        &self,
        design: &TableDesign,
        column_renames: &[(String, String)],
        cx: &App,
    ) -> String {
        let global_state = cx.global::<GlobalDbState>().clone();

        if let Ok(plugin) = global_state
            .db_manager
            .get_plugin(&self.config.database_type)
        {
            if let Some(original) = &self.original_design {
                let normalized = Self::normalize_column_renames(original, design, column_renames);
                plugin.build_alter_table_sql_with_renames(original, design, &normalized)
            } else {
                plugin.build_create_table_sql(design)
            }
        } else {
            String::new()
        }
    }

    fn build_ddl_preview_sql(&self, design: &TableDesign, cx: &App) -> String {
        if design.table_name.trim().is_empty() || design.columns.is_empty() {
            return String::new();
        }

        let global_state = cx.global::<GlobalDbState>().clone();
        if let Ok(plugin) = global_state
            .db_manager
            .get_plugin(&self.config.database_type)
        {
            plugin.build_create_table_sql(design)
        } else {
            String::new()
        }
    }

    fn update_previews(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let design = self.collect_design(cx);
        let column_renames = self.collect_column_renames(cx);
        let sql = self.build_diff_preview_sql(&design, &column_renames, cx);
        let ddl = self.build_ddl_preview_sql(&design, cx);

        self.sql_preview_input.update(cx, |state, cx| {
            state.set_value(sql, window, cx);
        });
        self.ddl_preview_input.update(cx, |state, cx| {
            state.set_value(ddl, window, cx);
        });
        cx.notify();
    }

    fn schedule_preview_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 子编辑器事件会在同一个 effect cycle 内级联触发，延后到周期末再统一回读状态，
        // 避免预览读取到上一拍的聚合结果。
        if !self.preview_refresh_state.request_refresh() {
            return;
        }

        cx.defer_in(window, |this, window, cx| {
            this.preview_refresh_state.finish_refresh();
            this.update_previews(window, cx);
        });
    }

    fn sql_has_changes(sql: &str) -> bool {
        let trimmed = sql.trim();
        let no_changes_localized = t!("SqlEditor.no_changes").to_string();
        !trimmed.is_empty()
            && !trimmed.starts_with("-- No changes")
            && !trimmed.starts_with(no_changes_localized.as_str())
    }

    fn contains_destructive_sql(sql: &str) -> bool {
        if !Self::sql_has_changes(sql) {
            return false;
        }

        let normalized_sql = sql
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("--"))
            .collect::<Vec<_>>()
            .join(" ")
            .to_uppercase();

        ["DROP COLUMN", "DROP INDEX", "DROP CONSTRAINT", "DROP TABLE"]
            .iter()
            .any(|keyword| normalized_sql.contains(keyword))
    }

    fn execute_request(&mut self, request: TableDesignerExecutionRequest, cx: &mut Context<Self>) {
        let global_state = cx.global::<GlobalDbState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = global_state
                .execute_script(
                    cx,
                    request.connection_id.clone(),
                    request.sql.clone(),
                    Some(request.database_name.clone()),
                    request.schema_name.clone(),
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
                                let msg = if request.is_new_table {
                                    t!("Table.create_success").to_string()
                                } else {
                                    t!("Table.modify_success").to_string()
                                };
                                window.push_notification(msg, cx);
                                let _ = this.update(cx, |designer, cx| {
                                    match &request.success_behavior {
                                        ExecuteSuccessBehavior::StayOpen { tab_id } => {
                                            cx.emit(TableDesignerEvent::Saved {
                                                connection_id: request.connection_id.clone(),
                                                database_name: request.database_name.clone(),
                                                schema_name: request.schema_name.clone(),
                                                table_name: request.table_name.clone(),
                                                is_new_table: request.is_new_table,
                                                tab_id: tab_id.clone(),
                                            });
                                            if request.is_new_table {
                                                designer.config.table_name =
                                                    Some(request.table_name.clone());
                                            }
                                            designer.load_table_structure(cx);
                                        }
                                        ExecuteSuccessBehavior::CloseTab {
                                            tab_container,
                                            tab_id,
                                            emitted_tab_id,
                                        } => {
                                            cx.emit(TableDesignerEvent::Saved {
                                                connection_id: request.connection_id.clone(),
                                                database_name: request.database_name.clone(),
                                                schema_name: request.schema_name.clone(),
                                                table_name: request.table_name.clone(),
                                                is_new_table: request.is_new_table,
                                                tab_id: emitted_tab_id.clone(),
                                            });
                                            tab_container.update(
                                                cx,
                                                |container: &mut TabContainer, cx| {
                                                    container.force_close_tab_by_id(tab_id, cx);
                                                },
                                            );
                                        }
                                    }
                                });
                            }
                        }
                        Err(e) => {
                            let msg = if request.is_new_table {
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

    fn maybe_confirm_and_execute(
        &mut self,
        request: TableDesignerExecutionRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !Self::contains_destructive_sql(&request.sql) {
            self.execute_request(request, cx);
            return;
        }

        self.active_tab = DesignerTab::SqlPreview;
        self.update_previews(window, cx);

        let designer_entity = cx.entity().clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let request_for_ok = request.clone();
            let designer_entity = designer_entity.clone();

            dialog
                .title(t!("Table.destructive_sql_confirm_title").to_string())
                .confirm()
                .overlay(false)
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Table.destructive_sql_confirm_execute").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(t!("Table.destructive_sql_confirm_message").to_string())
                        .child(t!("Table.destructive_sql_confirm_desc").to_string())
                        .child(t!("Common.irreversible").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let request = request_for_ok.clone();
                    designer_entity.update(cx, |designer, cx| {
                        designer.execute_request(request, cx);
                    });
                    true
                })
        });
    }

    pub fn has_unsaved_changes(&self, cx: &App) -> bool {
        let sql = self.sql_preview_input.read(cx).text().to_string();
        Self::sql_has_changes(&sql)
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

        let column_renames = self.collect_column_renames(cx);
        let sql = self.build_diff_preview_sql(&design, &column_renames, cx);

        if !Self::sql_has_changes(&sql) {
            tab_container.update(cx, |container: &mut TabContainer, cx| {
                container.force_close_tab_by_id(&tab_id, cx);
            });
            return;
        }

        let request = TableDesignerExecutionRequest {
            connection_id: self.config.connection_id.clone(),
            database_name: self.config.database_name.clone(),
            schema_name: self.config.schema_name.clone(),
            sql,
            table_name: design.table_name.clone(),
            is_new_table: self.original_design.is_none(),
            success_behavior: ExecuteSuccessBehavior::CloseTab {
                tab_container,
                tab_id,
                emitted_tab_id: self.config.tab_id.clone(),
            },
        };

        self.maybe_confirm_and_execute(request, window, cx);
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

                        let _ = this.update(cx, |designer, cx| {
                            let original_design = designer.build_original_design(
                                columns.unwrap_or_default(),
                                indexes.unwrap_or_default(),
                                cx,
                            );
                            designer.original_design = Some(original_design);
                            designer.update_previews(window, cx);
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
        cx: &App,
    ) -> TableDesign {
        let global_state = cx.global::<GlobalDbState>();
        let plugin = global_state
            .db_manager
            .get_plugin(&self.config.database_type)
            .ok();
        build_table_design_from_metadata(
            self.config.database_type,
            self.config.database_name.clone(),
            self.config.table_name.clone().unwrap_or_default(),
            &columns,
            &indexes,
            plugin.as_deref(),
        )
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

        let column_renames = self.collect_column_renames(cx);
        let sql = self.build_diff_preview_sql(&design, &column_renames, cx);

        if !Self::sql_has_changes(&sql) {
            window.push_notification(t!("Table.no_changes").to_string(), cx);
            return;
        }

        let request = TableDesignerExecutionRequest {
            connection_id: self.config.connection_id.clone(),
            database_name: self.config.database_name.clone(),
            schema_name: self.config.schema_name.clone(),
            sql,
            table_name: design.table_name.clone(),
            is_new_table: self.original_design.is_none(),
            success_behavior: ExecuteSuccessBehavior::StayOpen {
                tab_id: self.config.tab_id.clone(),
            },
        };

        self.maybe_confirm_and_execute(request, window, cx);
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
            DesignerTab::Ddl => 4,
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
                            4 => DesignerTab::Ddl,
                            _ => DesignerTab::Columns,
                        };
                        cx.notify();
                    }))
                    .child(Tab::new().label(t!("Table.columns").to_string()))
                    .child(Tab::new().label(t!("Table.indexes").to_string()))
                    .child(Tab::new().label(t!("Table.options").to_string()))
                    .child(Tab::new().label(t!("Table.sql_preview").to_string()))
                    .child(Tab::new().label(t!("Table.ddl").to_string())),
            )
            .into_any_element()
    }

    fn render_active_tab(&self, _window: &mut Window, cx: &Context<Self>) -> AnyElement {
        match self.active_tab {
            DesignerTab::Columns => self.columns_editor.clone().into_any_element(),
            DesignerTab::Indexes => self.indexes_editor.clone().into_any_element(),
            DesignerTab::Options => self.render_options(cx),
            DesignerTab::SqlPreview => self.render_sql_preview(cx),
            DesignerTab::Ddl => self.render_ddl_preview(cx),
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

    fn render_ddl_preview(&self, cx: &Context<Self>) -> AnyElement {
        let ddl_sql = self.ddl_preview_input.read(cx).text().to_string();

        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(
                h_flex()
                    .w_full()
                    .justify_end()
                    .child(Clipboard::new("table-designer-copy-ddl").value(ddl_sql)),
            )
            .child(
                Input::new(&self.ddl_preview_input)
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
    source_name: Option<String>,
    name_input: Entity<InputState>,
    type_select: Entity<SelectState<SearchableVec<String>>>,
    length_input: Entity<InputState>,
    scale_input: Entity<InputState>,
    nullable: bool,
    is_pk: bool,
    auto_increment: bool,
    is_unsigned: bool,
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

    fn update_collation_for_charset(
        database_type: DatabaseType,
        charset_select: &Entity<SelectState<Vec<CharsetSelectItem>>>,
        collation_select: &Entity<SelectState<Vec<CollationSelectItem>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selected = charset_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_default();
        // 空字符串表示 "(default)"，此时排序规则也重置为默认
        if selected.is_empty() {
            collation_select.update(cx, |state, cx| {
                let items = vec![CollationSelectItem {
                    info: CollationInfo {
                        name: "".into(),
                        charset: "".into(),
                        is_default: true,
                    },
                }];
                state.set_items(items, window, cx);
                state.set_selected_index(Some(IndexPath::new(0)), window, cx);
            });
            return;
        }
        let global_state = cx.global::<GlobalDbState>();
        let collations = if let Ok(plugin) = global_state.db_manager.get_plugin(&database_type) {
            plugin.get_collations(&selected)
        } else {
            vec![]
        };
        let items: Vec<CollationSelectItem> = collations
            .into_iter()
            .map(|info| CollationSelectItem { info })
            .collect();
        let default_idx = items.iter().position(|c| c.info.is_default).unwrap_or(0);
        collation_select.update(cx, |state, cx| {
            state.set_items(items, window, cx);
            state.set_selected_index(Some(IndexPath::new(default_idx)), window, cx);
        });
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
        let name_input = cx
            .new(|cx| InputState::new(window, cx).placeholder(t!("Table.column_name").to_string()));
        let type_items = SearchableVec::new(self.data_types.clone());
        let type_select = cx.new(|cx| {
            SelectState::new(type_items, Some(IndexPath::new(0)), window, cx).searchable(true)
        });
        let length_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("Table.length").to_string()));
        let scale_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("Table.decimal_places").to_string())
        });
        let default_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("Table.default_value").to_string())
        });
        let comment_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("Table.comment").to_string()));

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
        let type_sub = cx.subscribe_in(
            &type_select,
            window,
            |_this, _, _event: &SelectEvent<SearchableVec<String>>, _window, cx| {
                cx.emit(ColumnsEditorEvent::Changed);
            },
        );
        let charset_select_clone = charset_select.clone();
        let collation_select_clone = collation_select.clone();
        let charset_sub = cx.subscribe_in(
            &charset_select,
            window,
            move |this, _, _event: &SelectEvent<Vec<CharsetSelectItem>>, window, cx| {
                Self::update_collation_for_charset(
                    this.database_type,
                    &charset_select_clone,
                    &collation_select_clone,
                    window,
                    cx,
                );
                cx.emit(ColumnsEditorEvent::Changed);
            },
        );
        let collation_sub = cx.subscribe_in(
            &collation_select,
            window,
            |_this, _, _event: &SelectEvent<Vec<CollationSelectItem>>, _window, cx| {
                cx.emit(ColumnsEditorEvent::Changed);
            },
        );
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
            source_name: None,
            name_input,
            type_select,
            length_input,
            scale_input,
            nullable: true,
            is_pk: false,
            auto_increment: false,
            is_unsigned: false,
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
                let is_unsigned = row.is_unsigned && Self::supports_unsigned_type(&data_type);

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
                    is_unsigned,
                    default_value,
                    comment,
                    charset,
                    collation,
                }
            })
            .collect()
    }

    pub fn get_column_renames(&self, cx: &App) -> Vec<(String, String)> {
        self.columns
            .iter()
            .filter_map(|row| {
                let old_name = row.source_name.as_ref()?;
                let new_name = row.name_input.read(cx).text().to_string();
                if old_name.is_empty() || new_name.is_empty() || old_name == &new_name {
                    None
                } else {
                    Some((old_name.clone(), new_name))
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
            let parsed_type = plugin
                .as_ref()
                .map(|p| p.parse_column_type(&col.data_type))
                .unwrap_or_else(|| fallback_parse_column_type(&col.data_type));
            let base_type = parsed_type.base_type.clone();
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
            // 根据列已有的 charset 查找对应索引
            let charset_idx = col
                .charset
                .as_ref()
                .and_then(|cs| charset_items.iter().position(|item| item.info.name == *cs))
                .unwrap_or(0);
            let charset_select = cx.new(|cx| {
                SelectState::new(charset_items, Some(IndexPath::new(charset_idx)), window, cx)
            });

            // 根据列已有的 charset 加载对应的排序规则列表，并选中已有值
            let collation_select = cx.new(|cx| {
                let (items, selected_idx) = if let Some(ref charset_name) = col.charset {
                    let collations = if let Some(ref p) = plugin {
                        p.get_collations(charset_name)
                    } else {
                        vec![]
                    };
                    let items: Vec<CollationSelectItem> = collations
                        .into_iter()
                        .map(|info| CollationSelectItem { info })
                        .collect();
                    let idx = col
                        .collation
                        .as_ref()
                        .and_then(|coll| items.iter().position(|item| item.info.name == *coll))
                        .or_else(|| items.iter().position(|c| c.info.is_default))
                        .unwrap_or(0);
                    (items, idx)
                } else {
                    let items = vec![CollationSelectItem {
                        info: CollationInfo {
                            name: "".into(),
                            charset: "".into(),
                            is_default: true,
                        },
                    }];
                    (items, 0)
                };
                SelectState::new(items, Some(IndexPath::new(selected_idx)), window, cx)
            });

            let enum_values_input = cx.new(|cx| {
                let mut input = InputState::new(window, cx)
                    .placeholder(t!("Table.value_list_hint").to_string());
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
            let type_sub = cx.subscribe_in(
                &type_select,
                window,
                |_this, _, _event: &SelectEvent<SearchableVec<String>>, _window, cx| {
                    cx.emit(ColumnsEditorEvent::Changed);
                },
            );
            let charset_select_clone = charset_select.clone();
            let collation_select_clone = collation_select.clone();
            let charset_sub = cx.subscribe_in(
                &charset_select,
                window,
                move |this, _, _event: &SelectEvent<Vec<CharsetSelectItem>>, window, cx| {
                    Self::update_collation_for_charset(
                        this.database_type,
                        &charset_select_clone,
                        &collation_select_clone,
                        window,
                        cx,
                    );
                    cx.emit(ColumnsEditorEvent::Changed);
                },
            );
            let collation_sub = cx.subscribe_in(
                &collation_select,
                window,
                |_this, _, _event: &SelectEvent<Vec<CollationSelectItem>>, _window, cx| {
                    cx.emit(ColumnsEditorEvent::Changed);
                },
            );
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
                source_name: Some(col.name.clone()),
                name_input,
                type_select,
                length_input,
                scale_input,
                nullable: col.is_nullable,
                is_pk: col.is_primary_key,
                auto_increment: if matches!(self.database_type, DatabaseType::SQLite) {
                    col.is_primary_key && parsed_type.base_type.eq_ignore_ascii_case("INTEGER")
                } else {
                    parsed_type.is_auto_increment
                },
                is_unsigned: parsed_type.is_unsigned,
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

    fn supports_unsigned_type(data_type: &str) -> bool {
        matches!(
            data_type.to_uppercase().as_str(),
            "TINYINT"
                | "SMALLINT"
                | "MEDIUMINT"
                | "INT"
                | "INTEGER"
                | "BIGINT"
                | "DECIMAL"
                | "NUMERIC"
                | "FLOAT"
                | "DOUBLE"
                | "REAL"
        )
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
            .drag_over::<DragColumn>(move |el, _, _, _cx| {
                el.border_t_2().border_color(drag_border_color)
            })
            .on_drop(cx.listener(move |this, drag: &DragColumn, _window, cx| {
                this.move_column(drag.index, idx, cx);
            }))
            .child(
                div()
                    .id(("col-row-drag-handle", idx))
                    .w(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_grab()
                    .on_drag(DragColumn::new(idx, name), |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    })
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
        let name_input = cx
            .new(|cx| InputState::new(window, cx).placeholder(t!("Table.index_name").to_string()));
        let columns_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("Table.columns_comma_separated").to_string())
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
                let mut input = InputState::new(window, cx)
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

        let comment_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("Table.table_comment").to_string())
        });

        let engine_sub = cx.observe_in(&engine_select, window, |_this, _, _window, cx| {
            cx.emit(TableOptionsEvent::Changed);
        });
        let charset_select_clone = charset_select.clone();
        let collation_select_clone = collation_select.clone();
        let charset_sub = cx.observe_in(&charset_select, window, move |this, _, window, cx| {
            this.update_collations_for_charset(
                &charset_select_clone,
                &collation_select_clone,
                window,
                cx,
            );
            cx.emit(TableOptionsEvent::Changed);
        });
        let collation_sub = cx.observe_in(&collation_select, window, |_this, _, _window, cx| {
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

    fn update_collations_for_charset(
        &self,
        charset_select: &Entity<SelectState<Vec<CharsetSelectItem>>>,
        collation_select: &Entity<SelectState<Vec<CollationSelectItem>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selected_charset = charset_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "utf8mb4".to_string());

        let collations = Self::get_collations(&self._database_type, &selected_charset, cx);
        let collation_items: Vec<CollationSelectItem> = collations
            .into_iter()
            .map(|info| CollationSelectItem { info })
            .collect();
        let default_idx = collation_items
            .iter()
            .position(|c| c.info.is_default)
            .unwrap_or(0);

        collation_select.update(cx, |state, cx| {
            state.set_items(collation_items, window, cx);
            state.set_selected_index(Some(IndexPath::new(default_idx)), window, cx);
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use db::{
        clickhouse::ClickHousePlugin, mssql::MsSqlPlugin, mysql::MySqlPlugin, oracle::OraclePlugin,
        plugin::DatabasePlugin, postgresql::PostgresPlugin, sqlite::SqlitePlugin,
    };

    fn build_col(name: &str) -> ColumnDefinition {
        ColumnDefinition {
            name: name.to_string(),
            data_type: "INT".to_string(),
            is_nullable: false,
            ..Default::default()
        }
    }

    fn build_design(columns: Vec<ColumnDefinition>, index_columns: Vec<&str>) -> TableDesign {
        let indexes = if index_columns.is_empty() {
            vec![]
        } else {
            vec![IndexDefinition {
                name: "idx_test".to_string(),
                columns: index_columns
                    .into_iter()
                    .map(|name| name.to_string())
                    .collect(),
                is_unique: false,
                is_primary: false,
                index_type: None,
                comment: String::new(),
            }]
        };

        TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns,
            indexes,
            foreign_keys: vec![],
            options: TableOptions::default(),
        }
    }

    fn build_plugin(database_type: DatabaseType) -> Box<dyn DatabasePlugin> {
        match database_type {
            DatabaseType::MySQL => Box::new(MySqlPlugin::new()),
            DatabaseType::PostgreSQL => Box::new(PostgresPlugin::new()),
            DatabaseType::SQLite => Box::new(SqlitePlugin::new()),
            DatabaseType::DuckDB => Box::new(DuckDbPlugin::new()),
            DatabaseType::MSSQL => Box::new(MsSqlPlugin::new()),
            DatabaseType::Oracle => Box::new(OraclePlugin::new()),
            DatabaseType::ClickHouse => Box::new(ClickHousePlugin::new()),
        }
    }

    fn build_delete_and_rename_conflict_case() -> (TableDesign, TableDesign, Vec<(String, String)>)
    {
        let original = build_design(
            vec![build_col("a"), build_col("b"), build_col("c")],
            vec!["b"],
        );
        let current = build_design(vec![build_col("a"), build_col("c")], vec!["a"]);
        let renames = vec![("b".to_string(), "a".to_string())];
        (original, current, renames)
    }

    fn assert_contains_rename_sql(sql: &str, database_type: DatabaseType) {
        match database_type {
            DatabaseType::MySQL => {
                assert!(
                    sql.contains("CHANGE COLUMN"),
                    "MySQL 应使用 CHANGE COLUMN: {sql}"
                );
                assert!(sql.contains("`b`"), "MySQL 重命名 SQL 应包含源列 b: {sql}");
                assert!(
                    sql.contains("`a`"),
                    "MySQL 重命名 SQL 应包含目标列 a: {sql}"
                );
            }
            DatabaseType::PostgreSQL => {
                assert!(
                    sql.contains("RENAME COLUMN \"b\" TO \"a\""),
                    "PostgreSQL 应使用 RENAME COLUMN: {sql}"
                );
            }
            DatabaseType::SQLite | DatabaseType::DuckDB => {
                assert!(
                    sql.contains("RENAME COLUMN \"b\" TO \"a\""),
                    "SQLite/DuckDB 应使用 RENAME COLUMN: {sql}"
                );
            }
            DatabaseType::MSSQL => {
                assert!(
                    sql.contains("EXEC sp_rename '[users].[b]', 'a', 'COLUMN';"),
                    "MSSQL 应使用 sp_rename COLUMN: {sql}"
                );
            }
            DatabaseType::Oracle => {
                assert!(
                    sql.contains("RENAME COLUMN \"b\" TO \"a\""),
                    "Oracle 应使用 RENAME COLUMN: {sql}"
                );
            }
            DatabaseType::ClickHouse => {
                assert!(
                    sql.contains("RENAME COLUMN `b` TO `a`"),
                    "ClickHouse 应使用 RENAME COLUMN: {sql}"
                );
            }
        }
    }

    fn assert_not_drop_source_column(sql: &str, plugin: &dyn DatabasePlugin) {
        let drop_source = format!("DROP COLUMN {}", plugin.quote_identifier("b"));
        assert!(
            !sql.contains(&drop_source),
            "重命名来源列 b 不应被误删，出现 SQL: {drop_source} in {sql}"
        );
    }

    #[test]
    fn test_preview_refresh_schedule_state_coalesces_requests_in_same_cycle() {
        let mut state = PreviewRefreshScheduleState::default();

        assert!(state.request_refresh(), "第一次请求应当安排一次预览刷新");
        assert!(
            !state.request_refresh(),
            "同一事件周期内的第二次请求不应重复安排刷新"
        );
    }

    #[test]
    fn test_preview_refresh_schedule_state_allows_reschedule_after_finish() {
        let mut state = PreviewRefreshScheduleState::default();

        assert!(state.request_refresh(), "第一次请求应当成功安排刷新");
        state.finish_refresh();
        assert!(
            state.request_refresh(),
            "完成一次刷新后，后续请求应当可以再次安排刷新"
        );
    }

    #[test]
    fn test_contains_destructive_sql_detects_drop_column() {
        let sql = "ALTER TABLE users DROP COLUMN age;";
        assert!(TableDesigner::contains_destructive_sql(sql));
    }

    #[test]
    fn test_contains_destructive_sql_detects_drop_index() {
        let sql = "ALTER TABLE users DROP INDEX idx_users_name;";
        assert!(TableDesigner::contains_destructive_sql(sql));
    }

    #[test]
    fn test_contains_destructive_sql_detects_drop_constraint_and_table() {
        assert!(TableDesigner::contains_destructive_sql(
            "ALTER TABLE users DROP CONSTRAINT users_pk;"
        ));
        assert!(TableDesigner::contains_destructive_sql(
            "DROP TABLE users_backup;"
        ));
    }

    #[test]
    fn test_contains_destructive_sql_ignores_comment_only_drop_keyword() {
        let sql = "-- DROP COLUMN age\nALTER TABLE users ADD COLUMN age INT;";
        assert!(!TableDesigner::contains_destructive_sql(sql));
    }

    #[test]
    fn test_contains_destructive_sql_ignores_non_drop_changes() {
        assert!(!TableDesigner::contains_destructive_sql(
            "ALTER TABLE users ADD COLUMN age INT;"
        ));
        assert!(!TableDesigner::contains_destructive_sql(
            "ALTER TABLE users MODIFY COLUMN age BIGINT;"
        ));
    }

    #[test]
    fn test_contains_destructive_sql_ignores_no_changes_output() {
        assert!(!TableDesigner::contains_destructive_sql(
            "-- No changes detected"
        ));
    }

    #[test]
    fn test_normalize_column_renames_filters_invalid_items() {
        let original = build_design(
            vec![build_col("a"), build_col("b"), build_col("c")],
            vec!["a"],
        );
        let current = build_design(vec![build_col("a"), build_col("c")], vec!["a"]);

        let normalized = TableDesigner::normalize_column_renames(
            &original,
            &current,
            &[
                ("b".to_string(), "a".to_string()),
                ("x".to_string(), "z".to_string()),
                ("b".to_string(), "a2".to_string()),
                ("c".to_string(), "".to_string()),
            ],
        );

        assert_eq!(normalized, vec![("b".to_string(), "a".to_string())]);
    }

    #[test]
    fn test_map_design_for_diff_rewrites_column_and_index_names() {
        let current = build_design(vec![build_col("a"), build_col("c")], vec!["a"]);
        let mapped =
            db::plugin::map_design_for_diff(&current, &[("b".to_string(), "a".to_string())]);

        let column_names: Vec<&str> = mapped.columns.iter().map(|col| col.name.as_str()).collect();
        assert_eq!(column_names, vec!["b", "c"]);
        assert_eq!(mapped.indexes[0].columns, vec!["b".to_string()]);
    }

    #[test]
    fn test_merge_alter_sql_behaviour() {
        let sql = db::plugin::merge_alter_sql(
            "-- No changes detected".to_string(),
            vec!["ALTER TABLE \"users\" RENAME COLUMN \"b\" TO \"a\";".to_string()],
        );
        assert_eq!(sql, "ALTER TABLE \"users\" RENAME COLUMN \"b\" TO \"a\";");

        let no_change = db::plugin::merge_alter_sql(String::new(), Vec::new());
        assert_eq!(no_change, "-- No changes detected");
    }

    #[test]
    fn test_mysql_rename_sql_uses_change_column() {
        let plugin = MySqlPlugin::new();
        let col = build_col("a");
        let sql = plugin.build_column_rename_sql("users", "b", "a", Some(&col));
        assert!(sql.contains("CHANGE COLUMN"));
    }

    #[test]
    fn test_postgresql_rename_sql_uses_rename_column() {
        let plugin = PostgresPlugin::new();
        let sql = plugin.build_column_rename_sql("users", "b", "a", None);
        assert!(sql.contains("RENAME COLUMN \"b\" TO \"a\""));
    }

    #[test]
    fn test_sqlite_rename_sql_uses_rename_column() {
        let plugin = SqlitePlugin::new();
        let sql = plugin.build_column_rename_sql("users", "b", "a", None);
        assert!(sql.contains("RENAME COLUMN \"b\" TO \"a\""));
    }

    #[test]
    fn test_mssql_rename_sql_uses_sp_rename_column() {
        let plugin = MsSqlPlugin::new();
        let sql = plugin.build_column_rename_sql("users", "b", "a", None);
        assert_eq!(sql, "EXEC sp_rename '[users].[b]', 'a', 'COLUMN';");
    }

    #[test]
    fn test_oracle_rename_sql_uses_rename_column() {
        let plugin = OraclePlugin::new();
        let sql = plugin.build_column_rename_sql("users", "b", "a", None);
        assert!(sql.contains("RENAME COLUMN \"b\" TO \"a\""));
    }

    #[test]
    fn test_clickhouse_rename_sql_uses_rename_column() {
        let plugin = ClickHousePlugin::new();
        let sql = plugin.build_column_rename_sql("users", "b", "a", None);
        assert!(sql.contains("RENAME COLUMN `b` TO `a`"));
    }

    #[test]
    fn test_mysql_change_column_keeps_new_definition() {
        let plugin = MySqlPlugin::new();
        let mut renamed_col = build_col("a");
        renamed_col.data_type = "BIGINT".to_string();
        renamed_col.is_nullable = true;

        let sql = plugin.build_column_rename_sql("users", "b", "a", Some(&renamed_col));

        assert!(sql.contains("`a` BIGINT"));
        assert!(
            !sql.contains("NOT NULL"),
            "nullable=true 时不应强制生成 NOT NULL: {}",
            sql
        );
    }

    #[test]
    fn test_build_alter_table_sql_with_renames_contains_rename_for_all_databases() {
        let (original, current, renames) = build_delete_and_rename_conflict_case();

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql_with_renames(&original, &current, &renames);
            assert_contains_rename_sql(&sql, database_type);
        }
    }

    #[test]
    fn test_build_alter_table_sql_with_renames_not_drop_source_for_all_databases() {
        let (original, current, renames) = build_delete_and_rename_conflict_case();

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql_with_renames(&original, &current, &renames);
            assert_not_drop_source_column(&sql, plugin.as_ref());
        }
    }

    #[test]
    fn test_build_alter_table_sql_with_empty_renames_keeps_base_alter_sql() {
        let original = build_design(vec![build_col("a")], vec![]);
        let current = build_design(vec![build_col("a"), build_col("b")], vec![]);
        let plugin = PostgresPlugin::new();

        let base_sql = plugin.build_alter_table_sql(&original, &current);
        let sql = plugin.build_alter_table_sql_with_renames(&original, &current, &[]);

        assert_eq!(sql.trim(), base_sql.trim());
    }

    /// 测试简单重命名场景：仅修改列名，不修改其他属性
    #[test]
    fn test_simple_rename_generates_rename_not_drop_add() {
        let original = build_design(vec![build_col("a"), build_col("b"), build_col("c")], vec![]);
        // 用户将 b 重命名为 b2，其他不变
        let current = build_design(
            vec![build_col("a"), build_col("b2"), build_col("c")],
            vec![],
        );
        let renames = vec![("b".to_string(), "b2".to_string())];

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql_with_renames(&original, &current, &renames);
            // 不应包含 DROP COLUMN
            let drop_b = format!("DROP COLUMN {}", plugin.quote_identifier("b"));
            assert!(
                !sql.contains(&drop_b),
                "[{:?}] 简单重命名不应生成 DROP COLUMN: {sql}",
                database_type
            );
            // 不应包含 ADD COLUMN b2
            let add_b2 = format!("ADD COLUMN {}", plugin.quote_identifier("b2"));
            // MySQL ADD COLUMN 可能不包含引号后的完整格式，用更宽松的匹配
            let add_keyword = "ADD COLUMN";
            if !matches!(database_type, DatabaseType::SQLite | DatabaseType::DuckDB) {
                assert!(
                    !sql.contains(&add_b2)
                        && (!sql.contains(add_keyword) || sql.contains("RENAME")),
                    "[{:?}] 简单重命名不应生成 ADD COLUMN: {sql}",
                    database_type
                );
            }
            // 应包含 RENAME 相关语句
            let has_rename = sql.contains("RENAME COLUMN")
                || sql.contains("CHANGE COLUMN")
                || sql.contains("sp_rename");
            assert!(has_rename, "[{:?}] 应包含重命名SQL: {sql}", database_type);
            println!("[{:?}] SQL: {}", database_type, sql);
        }
    }

    /// 测试模拟真实场景：original_design 的 data_type 与 collect_design 的 data_type 大小写不同
    #[test]
    fn test_rename_with_type_case_mismatch_no_drop_add() {
        // 模拟 build_original_design 返回小写 data_type（从DB解析得到）
        let original = build_design(
            vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "integer".to_string(),
                    is_nullable: false,
                    is_primary_key: true,
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "name".to_string(),
                    data_type: "character varying".to_string(),
                    length: Some(50),
                    is_nullable: true,
                    ..Default::default()
                },
            ],
            vec![],
        );
        // 模拟 collect_design 返回大写 data_type（从下拉选择框获取）
        let current = build_design(
            vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    is_primary_key: true,
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "username".to_string(),
                    data_type: "VARCHAR".to_string(),
                    length: Some(50),
                    is_nullable: true,
                    ..Default::default()
                },
            ],
            vec![],
        );
        let renames = vec![("name".to_string(), "username".to_string())];

        let plugin = PostgresPlugin::new();
        let sql = plugin.build_alter_table_sql_with_renames(&original, &current, &renames);

        println!("Type case mismatch SQL: {}", sql);

        // 关键断言：不应包含 DROP COLUMN "name"
        assert!(
            !sql.contains("DROP COLUMN"),
            "类型大小写不同时不应生成 DROP COLUMN: {sql}"
        );
        // 不应包含 ADD COLUMN
        assert!(
            !sql.contains("ADD COLUMN"),
            "类型大小写不同时不应生成 ADD COLUMN: {sql}"
        );
        // 应包含 RENAME COLUMN
        assert!(
            sql.contains("RENAME COLUMN \"name\" TO \"username\""),
            "应包含正确的 RENAME SQL: {sql}"
        );
    }

    // ===== 以下为补充测试，覆盖各种表修改场景 =====

    /// 完全无变更时应返回 "-- No changes detected"
    #[test]
    fn test_no_changes_returns_no_changes_for_all_databases() {
        let design = build_design(vec![build_col("a"), build_col("b")], vec![]);

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql(&design, &design);
            assert_eq!(
                sql, "-- No changes detected",
                "[{:?}] 无变更时应返回 no changes",
                database_type
            );
        }
    }

    /// 仅新增列（无重命名）
    #[test]
    fn test_add_column_only_for_all_databases() {
        let original = build_design(vec![build_col("a")], vec![]);
        let current = build_design(vec![build_col("a"), build_col("b")], vec![]);

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql(&original, &current);
            let quoted_b = plugin.quote_identifier("b");
            assert!(
                sql.contains(&quoted_b),
                "[{:?}] 新增列应包含列名 b: {sql}",
                database_type
            );
            assert!(
                !sql.contains("DROP COLUMN"),
                "[{:?}] 仅新增列不应包含 DROP COLUMN: {sql}",
                database_type
            );
        }
    }

    /// 仅删除列（无重命名）
    #[test]
    fn test_drop_column_only_for_all_databases() {
        let original = build_design(vec![build_col("a"), build_col("b")], vec![]);
        let current = build_design(vec![build_col("a")], vec![]);

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql(&original, &current);
            // SQLite 使用 table recreation 方式，不包含 DROP COLUMN 关键词
            if !matches!(database_type, DatabaseType::SQLite | DatabaseType::DuckDB) {
                assert!(
                    sql.contains("DROP COLUMN"),
                    "[{:?}] 删除列应包含 DROP COLUMN: {sql}",
                    database_type
                );
            }
            assert!(
                !sql.starts_with("-- No changes"),
                "[{:?}] 删除列不应返回 no changes: {sql}",
                database_type
            );
        }
    }

    /// 仅修改列类型（无重命名）
    #[test]
    fn test_modify_column_type_for_all_databases() {
        let original = build_design(vec![build_col("a")], vec![]);
        let mut modified_col = build_col("a");
        modified_col.data_type = "BIGINT".to_string();
        let current = build_design(vec![modified_col], vec![]);

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql(&original, &current);
            assert!(
                sql.contains("BIGINT"),
                "[{:?}] 修改类型后应包含新类型 BIGINT: {sql}",
                database_type
            );
            assert!(
                !sql.starts_with("-- No changes"),
                "[{:?}] 修改类型不应返回 no changes: {sql}",
                database_type
            );
        }
    }

    /// 修改列 nullable 属性
    #[test]
    fn test_modify_column_nullable_for_all_databases() {
        let original = build_design(vec![build_col("a")], vec![]);
        let mut nullable_col = build_col("a");
        nullable_col.is_nullable = true;
        let current = build_design(vec![nullable_col], vec![]);

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql(&original, &current);
            assert!(
                !sql.starts_with("-- No changes"),
                "[{:?}] 修改 nullable 不应返回 no changes: {sql}",
                database_type
            );
        }
    }

    /// 重命名 + 同时新增另一列
    #[test]
    fn test_rename_and_add_column_simultaneously() {
        let original = build_design(vec![build_col("a"), build_col("b")], vec![]);
        let current = build_design(
            vec![build_col("a_new"), build_col("b"), build_col("c")],
            vec![],
        );
        let renames = vec![("a".to_string(), "a_new".to_string())];

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql_with_renames(&original, &current, &renames);
            // 应包含重命名
            let has_rename = sql.contains("RENAME COLUMN")
                || sql.contains("CHANGE COLUMN")
                || sql.contains("sp_rename");
            assert!(has_rename, "[{:?}] 应包含重命名 SQL: {sql}", database_type);
            // 应包含新增列 c
            let quoted_c = plugin.quote_identifier("c");
            assert!(
                sql.contains(&quoted_c),
                "[{:?}] 应包含新增列 c: {sql}",
                database_type
            );
            // 不应 DROP 被重命名的源列
            let drop_a = format!("DROP COLUMN {}", plugin.quote_identifier("a"));
            assert!(
                !sql.contains(&drop_a),
                "[{:?}] 不应 DROP 重命名源列 a: {sql}",
                database_type
            );
        }
    }

    /// 重命名 + 同时删除另一列
    #[test]
    fn test_rename_and_delete_another_column() {
        let original = build_design(vec![build_col("a"), build_col("b"), build_col("c")], vec![]);
        // 重命名 a→a2，删除 c
        let current = build_design(vec![build_col("a2"), build_col("b")], vec![]);
        let renames = vec![("a".to_string(), "a2".to_string())];

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql_with_renames(&original, &current, &renames);
            // 应包含重命名
            let has_rename = sql.contains("RENAME COLUMN")
                || sql.contains("CHANGE COLUMN")
                || sql.contains("sp_rename");
            assert!(has_rename, "[{:?}] 应包含重命名 SQL: {sql}", database_type);
            // 不应 DROP 重命名源列 a
            let drop_a = format!("DROP COLUMN {}", plugin.quote_identifier("a"));
            assert!(
                !sql.contains(&drop_a),
                "[{:?}] 不应 DROP 重命名源列 a: {sql}",
                database_type
            );
            // SQLite 使用 table recreation，不直接包含 DROP COLUMN
            if !matches!(database_type, DatabaseType::SQLite | DatabaseType::DuckDB) {
                // 应 DROP 被删除的列 c
                let drop_c = format!("DROP COLUMN {}", plugin.quote_identifier("c"));
                assert!(
                    sql.contains(&drop_c),
                    "[{:?}] 应 DROP 被删除的列 c: {sql}",
                    database_type
                );
            }
        }
    }

    /// 重命名 + 同时修改另一列的类型
    #[test]
    fn test_rename_and_modify_another_column() {
        let original = build_design(vec![build_col("a"), build_col("b")], vec![]);
        let mut modified_b = build_col("b");
        modified_b.data_type = "BIGINT".to_string();
        let current = build_design(vec![build_col("a_new"), modified_b], vec![]);
        let renames = vec![("a".to_string(), "a_new".to_string())];

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql_with_renames(&original, &current, &renames);
            // 应包含重命名
            let has_rename = sql.contains("RENAME COLUMN")
                || sql.contains("CHANGE COLUMN")
                || sql.contains("sp_rename");
            assert!(has_rename, "[{:?}] 应包含重命名 SQL: {sql}", database_type);
            // 应包含修改列 b 的类型
            assert!(
                sql.contains("BIGINT"),
                "[{:?}] 应包含修改后的类型 BIGINT: {sql}",
                database_type
            );
        }
    }

    /// 同时重命名多列
    #[test]
    fn test_multiple_renames_simultaneously() {
        let original = build_design(vec![build_col("a"), build_col("b"), build_col("c")], vec![]);
        let current = build_design(vec![build_col("x"), build_col("y"), build_col("c")], vec![]);
        let renames = vec![
            ("a".to_string(), "x".to_string()),
            ("b".to_string(), "y".to_string()),
        ];

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql_with_renames(&original, &current, &renames);
            // 不应 DROP 源列
            let drop_a = format!("DROP COLUMN {}", plugin.quote_identifier("a"));
            let drop_b = format!("DROP COLUMN {}", plugin.quote_identifier("b"));
            assert!(
                !sql.contains(&drop_a),
                "[{:?}] 不应 DROP 重命名源列 a: {sql}",
                database_type
            );
            assert!(
                !sql.contains(&drop_b),
                "[{:?}] 不应 DROP 重命名源列 b: {sql}",
                database_type
            );
            // 不应 ADD 目标列
            assert!(
                !sql.contains("ADD COLUMN") || sql.contains("RENAME") || sql.contains("CHANGE"),
                "[{:?}] 不应 ADD COLUMN: {sql}",
                database_type
            );
            // 应包含两条重命名语句
            let rename_count = sql.match_indices("RENAME COLUMN").count()
                + sql.match_indices("CHANGE COLUMN").count()
                + sql.match_indices("sp_rename").count();
            assert_eq!(
                rename_count, 2,
                "[{:?}] 应包含 2 条重命名语句，实际 {} 条: {sql}",
                database_type, rename_count
            );
        }
    }

    /// 重命名的列同时也是索引列
    #[test]
    fn test_rename_column_in_index() {
        let original = build_design(
            vec![build_col("a"), build_col("b"), build_col("c")],
            vec!["b"],
        );
        // 重命名索引列 b→b2，索引也应指向 b2
        let mut current = build_design(
            vec![build_col("a"), build_col("b2"), build_col("c")],
            vec!["b2"],
        );
        current.indexes[0].name = "idx_test".to_string();
        let renames = vec![("b".to_string(), "b2".to_string())];

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql_with_renames(&original, &current, &renames);
            // 应包含重命名语句
            let has_rename = sql.contains("RENAME COLUMN")
                || sql.contains("CHANGE COLUMN")
                || sql.contains("sp_rename");
            assert!(has_rename, "[{:?}] 应包含重命名 SQL: {sql}", database_type);
            // 不应误删再重建索引（因为 map_design_for_diff 会将 b2 映射回 b）
            assert!(
                !sql.contains("DROP INDEX"),
                "[{:?}] 重命名索引列不应删除索引: {sql}",
                database_type
            );
        }
    }

    /// 新增索引（无列重命名）
    #[test]
    fn test_add_index_for_all_databases() {
        let original = build_design(vec![build_col("a"), build_col("b")], vec![]);
        let current = build_design(vec![build_col("a"), build_col("b")], vec!["a"]);

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql(&original, &current);
            // 应包含 INDEX 相关关键字
            assert!(
                sql.to_uppercase().contains("INDEX"),
                "[{:?}] 新增索引应包含 INDEX 关键字: {sql}",
                database_type
            );
        }
    }

    /// 删除索引（无列重命名）
    #[test]
    fn test_drop_index_for_all_databases() {
        let original = build_design(vec![build_col("a"), build_col("b")], vec!["a"]);
        let current = build_design(vec![build_col("a"), build_col("b")], vec![]);

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql(&original, &current);
            assert!(
                sql.to_uppercase().contains("DROP") && sql.to_uppercase().contains("INDEX"),
                "[{:?}] 删除索引应包含 DROP INDEX: {sql}",
                database_type
            );
        }
    }

    /// map_design_for_diff 支持多列重命名
    #[test]
    fn test_map_design_for_diff_multiple_renames() {
        let current = build_design(
            vec![build_col("x"), build_col("y"), build_col("z")],
            vec!["x", "y"],
        );
        let renames = vec![
            ("a".to_string(), "x".to_string()),
            ("b".to_string(), "y".to_string()),
        ];
        let mapped = db::plugin::map_design_for_diff(&current, &renames);
        let names: Vec<&str> = mapped.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "z"]);
        // 索引列也应被回退
        assert_eq!(
            mapped.indexes[0].columns,
            vec!["a".to_string(), "b".to_string()]
        );
    }

    /// map_design_for_diff 当列名不匹配时不做任何修改
    #[test]
    fn test_map_design_for_diff_no_matching_column() {
        let current = build_design(vec![build_col("a"), build_col("b")], vec![]);
        let renames = vec![("x".to_string(), "y".to_string())];
        let mapped = db::plugin::map_design_for_diff(&current, &renames);
        let names: Vec<&str> = mapped.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    /// map_design_for_diff 传空重命名列表时返回原样
    #[test]
    fn test_map_design_for_diff_empty_renames() {
        let current = build_design(vec![build_col("a"), build_col("b")], vec!["a"]);
        let mapped = db::plugin::map_design_for_diff(&current, &[]);
        let names: Vec<&str> = mapped.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
        assert_eq!(mapped.indexes[0].columns, vec!["a".to_string()]);
    }

    /// merge_alter_sql 同时有 base SQL 和 rename 语句时两者都保留
    #[test]
    fn test_merge_alter_sql_combines_base_and_renames() {
        let base = "ALTER TABLE \"users\" ADD COLUMN \"c\" INT NOT NULL;".to_string();
        let renames = vec![
            "ALTER TABLE \"users\" RENAME COLUMN \"a\" TO \"x\";".to_string(),
            "ALTER TABLE \"users\" RENAME COLUMN \"b\" TO \"y\";".to_string(),
        ];
        let result = db::plugin::merge_alter_sql(base.clone(), renames);
        assert!(result.contains(&base), "应包含 base SQL");
        assert!(
            result.contains("RENAME COLUMN \"a\" TO \"x\""),
            "应包含第一条 rename"
        );
        assert!(
            result.contains("RENAME COLUMN \"b\" TO \"y\""),
            "应包含第二条 rename"
        );
        // 各语句用换行分隔
        assert_eq!(result.lines().count(), 3, "应有 3 行 SQL 语句");
    }

    /// merge_alter_sql base 为空字符串时只保留 rename 语句
    #[test]
    fn test_merge_alter_sql_empty_base_with_renames() {
        let renames = vec!["ALTER TABLE t RENAME COLUMN a TO b;".to_string()];
        let result = db::plugin::merge_alter_sql(String::new(), renames);
        assert_eq!(result, "ALTER TABLE t RENAME COLUMN a TO b;");
    }

    /// merge_alter_sql base 为纯空白时跳过 base
    #[test]
    fn test_merge_alter_sql_whitespace_base() {
        let renames = vec!["RENAME SQL;".to_string()];
        let result = db::plugin::merge_alter_sql("   \n  ".to_string(), renames);
        assert_eq!(result, "RENAME SQL;");
    }

    /// normalize_column_renames 过滤自身重命名（old == new）
    #[test]
    fn test_normalize_column_renames_filters_self_renames() {
        let original = build_design(vec![build_col("a"), build_col("b")], vec![]);
        let current = build_design(vec![build_col("a"), build_col("b")], vec![]);
        let normalized = TableDesigner::normalize_column_renames(
            &original,
            &current,
            &[("a".to_string(), "a".to_string())],
        );
        assert!(normalized.is_empty(), "old == new 的重命名应被过滤");
    }

    /// normalize_column_renames 去重：同一旧列名只保留第一次出现
    #[test]
    fn test_normalize_column_renames_dedup_old_name() {
        let original = build_design(vec![build_col("a"), build_col("b"), build_col("c")], vec![]);
        let current = build_design(vec![build_col("x"), build_col("y"), build_col("c")], vec![]);
        let normalized = TableDesigner::normalize_column_renames(
            &original,
            &current,
            &[
                ("a".to_string(), "x".to_string()),
                ("a".to_string(), "y".to_string()), // 重复旧列名
            ],
        );
        assert_eq!(
            normalized,
            vec![("a".to_string(), "x".to_string())],
            "同一旧列名只保留第一次"
        );
    }

    /// normalize_column_renames 去重：同一新列名只保留第一次出现
    #[test]
    fn test_normalize_column_renames_dedup_new_name() {
        let original = build_design(vec![build_col("a"), build_col("b"), build_col("c")], vec![]);
        let current = build_design(vec![build_col("x"), build_col("b"), build_col("c")], vec![]);
        let normalized = TableDesigner::normalize_column_renames(
            &original,
            &current,
            &[
                ("a".to_string(), "x".to_string()),
                ("b".to_string(), "x".to_string()), // 重复新列名
            ],
        );
        assert_eq!(
            normalized,
            vec![("a".to_string(), "x".to_string())],
            "同一新列名只保留第一次"
        );
    }

    /// MySQL build_column_rename_sql 无列定义时回退为 RENAME COLUMN
    #[test]
    fn test_mysql_rename_sql_fallback_without_column_def() {
        let plugin = MySqlPlugin::new();
        let sql = plugin.build_column_rename_sql("users", "old_col", "new_col", None);
        assert!(
            sql.contains("RENAME COLUMN"),
            "MySQL 无列定义时应回退为 RENAME COLUMN: {sql}"
        );
        assert!(
            !sql.contains("CHANGE COLUMN"),
            "MySQL 无列定义时不应使用 CHANGE COLUMN: {sql}"
        );
    }

    /// MySQL CHANGE COLUMN 保留 default 值和 comment
    #[test]
    fn test_mysql_change_column_preserves_default_and_comment() {
        let plugin = MySqlPlugin::new();
        let col = ColumnDefinition {
            name: "new_name".to_string(),
            data_type: "VARCHAR".to_string(),
            length: Some(100),
            is_nullable: true,
            default_value: Some("'hello'".to_string()),
            comment: "用户名字段".to_string(),
            ..Default::default()
        };
        let sql = plugin.build_column_rename_sql("users", "old_name", "new_name", Some(&col));
        assert!(sql.contains("CHANGE COLUMN"), "应使用 CHANGE COLUMN: {sql}");
        assert!(sql.contains("DEFAULT 'hello'"), "应保留 DEFAULT 值: {sql}");
        assert!(
            sql.contains("COMMENT '用户名字段'"),
            "应保留 COMMENT: {sql}"
        );
    }

    /// MySQL CHANGE COLUMN 保留 auto_increment
    #[test]
    fn test_mysql_change_column_preserves_auto_increment() {
        let plugin = MySqlPlugin::new();
        let col = ColumnDefinition {
            name: "id".to_string(),
            data_type: "INT".to_string(),
            is_nullable: false,
            is_auto_increment: true,
            ..Default::default()
        };
        let sql = plugin.build_column_rename_sql("users", "old_id", "id", Some(&col));
        assert!(
            sql.contains("AUTO_INCREMENT"),
            "应保留 AUTO_INCREMENT: {sql}"
        );
    }

    /// MSSQL sp_rename 正确处理包含单引号的列名
    #[test]
    fn test_mssql_rename_escapes_single_quotes() {
        let plugin = MsSqlPlugin::new();
        let sql = plugin.build_column_rename_sql("users", "col'a", "col'b", None);
        // 单引号应被转义
        assert!(sql.contains("col''a"), "旧列名中的单引号应被转义: {sql}");
        assert!(sql.contains("col''b"), "新列名中的单引号应被转义: {sql}");
    }

    /// build_alter_table_sql_with_renames 传入空设计（无列、无索引）
    #[test]
    fn test_alter_table_with_renames_empty_designs() {
        let empty = build_design(vec![], vec![]);
        let plugin = PostgresPlugin::new();
        let sql = plugin.build_alter_table_sql_with_renames(&empty, &empty, &[]);
        assert_eq!(sql, "-- No changes detected");
    }

    /// 重命名 + 修改同一列的类型（重命名列同时改了 data_type）
    #[test]
    fn test_rename_and_modify_same_column() {
        let original = build_design(vec![build_col("a"), build_col("b")], vec![]);
        let mut renamed_and_modified = build_col("a_new");
        renamed_and_modified.data_type = "BIGINT".to_string();
        let current = build_design(vec![renamed_and_modified, build_col("b")], vec![]);
        let renames = vec![("a".to_string(), "a_new".to_string())];

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql_with_renames(&original, &current, &renames);
            // 应包含重命名
            let has_rename = sql.contains("RENAME COLUMN")
                || sql.contains("CHANGE COLUMN")
                || sql.contains("sp_rename");
            assert!(has_rename, "[{:?}] 应包含重命名 SQL: {sql}", database_type);
            // 对于 MySQL 的 CHANGE COLUMN，类型变更和重命名合为一条
            // 对于其他数据库，类型变更和重命名分别生成
            if database_type == DatabaseType::MySQL {
                // MySQL CHANGE COLUMN 自带完整列定义，包含新类型
                assert!(
                    sql.contains("BIGINT"),
                    "[MySQL] CHANGE COLUMN 应包含新类型 BIGINT: {sql}"
                );
            } else {
                // 其他数据库应同时包含 RENAME 和 ALTER COLUMN（类型变更）
                assert!(
                    sql.contains("BIGINT"),
                    "[{:?}] 应包含类型变更 BIGINT: {sql}",
                    database_type
                );
            }
        }
    }

    /// 所有列都被删除后应生成合法 SQL
    #[test]
    fn test_delete_all_columns() {
        let original = build_design(vec![build_col("a"), build_col("b")], vec![]);
        let current = build_design(vec![], vec![]);

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql(&original, &current);
            // 应生成非空 SQL（而非 no changes）
            assert!(
                !sql.starts_with("-- No changes"),
                "[{:?}] 删除所有列不应返回 no changes: {sql}",
                database_type
            );
        }
    }

    /// 从零列新增多列
    #[test]
    fn test_add_multiple_columns_from_empty() {
        let original = build_design(vec![], vec![]);
        let current = build_design(vec![build_col("a"), build_col("b"), build_col("c")], vec![]);

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql(&original, &current);
            let quoted_a = plugin.quote_identifier("a");
            let quoted_b = plugin.quote_identifier("b");
            let quoted_c = plugin.quote_identifier("c");
            assert!(
                sql.contains(&quoted_a) && sql.contains(&quoted_b) && sql.contains(&quoted_c),
                "[{:?}] 应包含所有新增列 a, b, c: {sql}",
                database_type
            );
        }
    }

    /// 同时新增和删除索引
    #[test]
    fn test_add_and_drop_index_simultaneously() {
        let mut original = build_design(
            vec![build_col("a"), build_col("b"), build_col("c")],
            vec!["a"],
        );
        original.indexes[0].name = "idx_old".to_string();

        let mut current = build_design(
            vec![build_col("a"), build_col("b"), build_col("c")],
            vec!["b"],
        );
        current.indexes[0].name = "idx_new".to_string();

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql(&original, &current);
            let upper = sql.to_uppercase();
            assert!(
                upper.contains("DROP") && upper.contains("INDEX"),
                "[{:?}] 应包含 DROP INDEX: {sql}",
                database_type
            );
            // 应包含创建新索引（INDEX 关键字出现在创建和删除两处）
            let idx_count = upper.match_indices("INDEX").count();
            assert!(
                idx_count >= 2,
                "[{:?}] INDEX 关键字应至少出现 2 次（增+删）: {sql}",
                database_type
            );
        }
    }

    /// 添加带完整属性的列（default、comment、nullable）
    #[test]
    fn test_add_column_with_full_attributes() {
        let original = build_design(vec![build_col("id")], vec![]);
        let new_col = ColumnDefinition {
            name: "email".to_string(),
            data_type: "VARCHAR".to_string(),
            length: Some(255),
            is_nullable: true,
            default_value: Some("''".to_string()),
            comment: "邮箱地址".to_string(),
            ..Default::default()
        };
        let current = build_design(vec![build_col("id"), new_col], vec![]);

        // MySQL 支持 COMMENT 和 DEFAULT
        let plugin = MySqlPlugin::new();
        let sql = plugin.build_alter_table_sql(&original, &current);
        assert!(sql.contains("VARCHAR(255)"), "应包含类型和长度: {sql}");
        assert!(sql.contains("DEFAULT ''"), "应包含 DEFAULT: {sql}");
        assert!(sql.contains("COMMENT '邮箱地址'"), "应包含 COMMENT: {sql}");
    }

    /// 修改列 default 值
    #[test]
    fn test_modify_column_default_value() {
        let mut col_orig = build_col("a");
        col_orig.default_value = Some("0".to_string());
        let original = build_design(vec![col_orig], vec![]);

        let mut col_new = build_col("a");
        col_new.default_value = Some("1".to_string());
        let current = build_design(vec![col_new], vec![]);

        for database_type in DatabaseType::all().iter().copied() {
            let plugin = build_plugin(database_type);
            let sql = plugin.build_alter_table_sql(&original, &current);
            assert!(
                !sql.starts_with("-- No changes"),
                "[{:?}] 修改 DEFAULT 值不应返回 no changes: {sql}",
                database_type
            );
        }
    }

    #[test]
    fn test_column_info_to_definition_preserves_text_metadata() {
        let column = ColumnInfo {
            name: "session_id".to_string(),
            data_type: "varchar(255)".to_string(),
            is_nullable: false,
            is_primary_key: false,
            default_value: None,
            comment: Some("会话ID".to_string()),
            charset: Some("utf8mb4".to_string()),
            collation: Some("utf8mb4_general_ci".to_string()),
        };
        let parsed = MySqlPlugin::new().parse_column_type(&column.data_type);

        let definition = column_info_to_definition(DatabaseType::MySQL, &column, parsed);

        assert_eq!(definition.data_type, "varchar");
        assert_eq!(definition.length, Some(255));
        assert_eq!(definition.comment, "会话ID");
        assert_eq!(definition.charset.as_deref(), Some("utf8mb4"));
        assert_eq!(definition.collation.as_deref(), Some("utf8mb4_general_ci"));
    }

    #[test]
    fn test_column_info_to_definition_preserves_unsigned_and_enum_values() {
        let numeric = ColumnInfo {
            name: "amount".to_string(),
            data_type: "int(11) unsigned".to_string(),
            is_nullable: false,
            is_primary_key: false,
            default_value: Some("0".to_string()),
            comment: None,
            charset: None,
            collation: None,
        };
        let enum_col = ColumnInfo {
            name: "status".to_string(),
            data_type: "enum('todo','done')".to_string(),
            is_nullable: false,
            is_primary_key: false,
            default_value: Some("'todo'".to_string()),
            comment: None,
            charset: Some("utf8mb4".to_string()),
            collation: Some("utf8mb4_bin".to_string()),
        };

        let numeric_definition = column_info_to_definition(
            DatabaseType::MySQL,
            &numeric,
            MySqlPlugin::new().parse_column_type(&numeric.data_type),
        );
        let enum_definition = column_info_to_definition(
            DatabaseType::MySQL,
            &enum_col,
            MySqlPlugin::new().parse_column_type(&enum_col.data_type),
        );

        assert!(numeric_definition.is_unsigned);
        assert_eq!(numeric_definition.length, Some(11));
        assert_eq!(enum_definition.data_type, "enum('todo','done')");
        assert_eq!(enum_definition.collation.as_deref(), Some("utf8mb4_bin"));
    }
}
