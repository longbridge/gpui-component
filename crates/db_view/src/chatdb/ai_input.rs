//! AI Input - 支持 SQL/Agent 双模式的智能输入组件

use db::GlobalDbState;
use db::plugin::SqlCompletionInfo;
use gpui::prelude::FluentBuilder;
use gpui::{div, px, App, AppContext, AsyncApp, Context, Corner, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement, Render, Styled, Subscription, Window};
use gpui_component::{
    button::Button,
    h_flex,
    input::InputEvent,
    popover::Popover,
    v_flex, ActiveTheme, IconName, Sizable, Size,
};
use gpui_component::button::ButtonVariants;
use rust_i18n::t;
use std::rc::Rc;

// 从核心库导入可复用组件
use one_core::ai_chat::components::{
    ProviderItem, ProviderSelectEvent, ProviderSelectState, SendButton, SendButtonState,
    ModelSettings, ModelSettingsPanel, ModelSettingsEvent, ModelSettingsLabels,
};

use crate::chatdb::db_connection_selector::{ConnectionItem, DbConnectionSelector, DbConnectionSelectorEvent};
use crate::sql_editor::{DefaultSqlCompletionProvider, SqlEditor, SqlSchema, TableMentionCompletionProvider};

// ============================================================================
// 数据类型定义
// ============================================================================

/// 输入模式
#[derive(Clone, Debug, Default, PartialEq)]
pub enum InputMode {
    #[default]
    Agent,
    Sql,
}

impl InputMode {
    pub fn label(&self) -> &'static str {
        match self {
            InputMode::Agent => "Agent",
            InputMode::Sql => "SQL",
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            InputMode::Agent => IconName::Bot,
            InputMode::Sql => IconName::Database,
        }
    }
}

// ============================================================================
// 事件定义
// ============================================================================

/// AI 输入框事件
#[derive(Clone, Debug)]
pub enum AIInputEvent {
    Submit { content: String },
    ProviderChanged { provider_id: String },
    ModelChanged { model: String },
    ExecuteSql {
        sql: String,
        connection_id: String,
        database: Option<String>,
        schema: Option<String>,
    },
    ModeChanged { mode: InputMode },
    SettingsChanged { settings: ModelSettings },
    /// 取消当前操作
    Cancel,
}

// ============================================================================
// AIInput 组件
// ============================================================================

/// AI 输入框组件
pub struct AIInput {
    focus_handle: FocusHandle,

    // 模式状态
    mode: InputMode,

    // SQL/Agent 共用编辑器
    sql_editor: Entity<SqlEditor>,

    // 选择器
    provider_select_state: ProviderSelectState,
    db_selector: Entity<DbConnectionSelector>,

    // 模型设置
    settings_panel: Entity<ModelSettingsPanel>,
    model_settings: ModelSettings,
    send_button_state: SendButtonState,

    // 订阅
    _subscriptions: Vec<Subscription>,

    // 状态
    selected_provider: Option<String>,
    selected_model: Option<String>,
    selected_connection: Option<ConnectionItem>,
    selected_database: Option<String>,
    selected_schema: Option<String>,
    supports_schema: bool,
    uses_schema_as_database: bool,
    is_loading: bool,
    sql_schema: SqlSchema,
    db_completion_info: Option<SqlCompletionInfo>,
}

impl AIInput {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // SQL/Agent 共用编辑器
        let sql_editor = cx.new(|cx| SqlEditor::new(window, cx));

        // Provider 选择器（回调直接接收 &mut Self，避免重复借用）
        let provider_select_state = ProviderSelectState::new(window, cx, |event, this, window, cx| {
            match event {
                ProviderSelectEvent::ProviderChanged { provider_id, .. } => {
                    this.selected_provider = Some(provider_id.clone());
                    this.selected_model = this.provider_select_state
                        .update_models_for_provider(&provider_id, window, cx);
                    cx.emit(AIInputEvent::ProviderChanged {
                        provider_id: provider_id.clone(),
                    });
                    if let Some(model) = this.selected_model.clone() {
                        cx.emit(AIInputEvent::ModelChanged { model });
                    }
                }
                ProviderSelectEvent::ModelChanged { model } => {
                    this.selected_model = Some(model.clone());
                    cx.emit(AIInputEvent::ModelChanged { model });
                }
            }
        });

        // 数据源选择器
        let db_selector = cx.new(|cx| DbConnectionSelector::new(window, cx));

        // 模型设置（使用国际化标签）
        let model_settings = ModelSettings::default();
        let labels = ModelSettingsLabels {
            title: t!("ModelSettings.title").to_string(),
            temperature_label: t!("ModelSettings.temperature_label").to_string(),
            temperature_desc: t!("ModelSettings.temperature_desc").to_string(),
            history_label: t!("ModelSettings.history_label").to_string(),
            history_desc: t!("ModelSettings.history_desc").to_string(),
            max_tokens_label: t!("ModelSettings.max_tokens_label").to_string(),
            max_tokens_desc: t!("ModelSettings.max_tokens_desc").to_string(),
            footer_notice: t!("ModelSettings.footer_notice").to_string(),
        };
        let settings_panel = cx.new(|cx| {
            ModelSettingsPanel::with_labels(model_settings.clone(), labels, window, cx)
        });

        let send_button_state = SendButtonState::new()
            .with_send_label(t!("AIInput.send").to_string())
            .with_cancel_label(t!("AIInput.cancel").to_string());

        let mut subscriptions = Vec::new();

        // 订阅数据源选择事件
        subscriptions.push(cx.subscribe_in(
            &db_selector,
            window,
            |this, _selector, event, _window, cx| {
                let DbConnectionSelectorEvent::SelectionChanged {
                    connection,
                    database,
                    schema,
                    supports_schema,
                    uses_schema_as_database,
                } = event;
                this.selected_connection = connection.clone();
                this.selected_database = database.clone();
                this.selected_schema = schema.clone();
                this.supports_schema = *supports_schema;
                this.uses_schema_as_database = *uses_schema_as_database;
                this.update_sql_editor_schema(cx);
            },
        ));

        // 订阅 SQL 编辑器事件
        subscriptions.push(cx.subscribe_in(
            &sql_editor.read(cx).input(),
            window,
            |this, _state, event, window, cx| {
                if let InputEvent::PressEnter { secondary } = event {
                    if !secondary {
                        this.submit(window, cx);
                    }
                }
            },
        ));

        // 订阅设置面板事件
        subscriptions.push(cx.subscribe_in(
            &settings_panel,
            window,
            |this, _panel, event, _window, cx| {
                let ModelSettingsEvent::Changed(settings) = event;
                this.model_settings = settings.clone();
                cx.emit(AIInputEvent::SettingsChanged {
                    settings: settings.clone(),
                });
            },
        ));

        let mut instance = Self {
            focus_handle,
            mode: InputMode::Agent,
            sql_editor,
            provider_select_state,
            db_selector,
            settings_panel,
            model_settings,
            send_button_state,
            _subscriptions: subscriptions,
            selected_provider: None,
            selected_model: None,
            selected_connection: None,
            selected_database: None,
            selected_schema: None,
            supports_schema: false,
            uses_schema_as_database: false,
            is_loading: false,
            sql_schema: SqlSchema::default(),
            db_completion_info: None,
        };

        instance.apply_editor_mode(window, cx);

        instance
    }

    // ========================================================================
    // 公开方法
    // ========================================================================

    pub fn get_connection_info(&self) -> Option<(String, Option<String>, Option<String>)> {
        let conn = self.selected_connection.as_ref()?;
        if self.uses_schema_as_database {
            let schema = self.selected_schema.clone()?;
            return Some((conn.id.clone(), None, Some(schema)));
        }
        let database = self.selected_database.clone()?;
        Some((conn.id.clone(), Some(database), self.selected_schema.clone()))
    }

    /// 获取模型设置
    pub fn get_model_settings(&self) -> &ModelSettings {
        &self.model_settings
    }

    pub fn update_providers(
        &mut self,
        providers: Vec<ProviderItem>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 使用 set_providers_with_builtin 自动添加内置 provider，再同步状态
        self.provider_select_state.set_providers_with_builtin(providers, window, cx);
        self.selected_provider = self.provider_select_state.selected_provider().cloned();
        self.selected_model = self.provider_select_state.selected_model().cloned();

        // 通知外部当前选中的 provider 和 model
        if let Some(provider_id) = self.selected_provider.clone() {
            cx.emit(AIInputEvent::ProviderChanged { provider_id });
        }
        if let Some(model) = self.selected_model.clone() {
            cx.emit(AIInputEvent::ModelChanged { model });
        }
        cx.notify();
    }

    pub fn set_loading(&mut self, loading: bool, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_loading == loading {
            return;
        }
        self.is_loading = loading;
        self.send_button_state.set_loading(loading);
        cx.notify();
    }

    pub fn set_mode(&mut self, mode: InputMode, window: &mut Window, cx: &mut Context<Self>) {
        if self.mode == mode {
            return;
        }
        self.mode = mode.clone();
        self.apply_editor_mode(window, cx);
        cx.emit(AIInputEvent::ModeChanged { mode });
        cx.notify();
    }

    fn apply_editor_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.mode {
            InputMode::Agent => {
                self.sql_editor.update(cx, |editor, cx| {
                    editor.set_placeholder(
                        t!("AIInput.placeholder_agent").to_string(),
                        window,
                        cx
                    );
                });
                self.send_button_state.send_label = t!("AIInput.send").to_string();
            }
            InputMode::Sql => {
                self.sql_editor.update(cx, |editor, cx| {
                    editor.set_placeholder(
                        t!("AIInput.placeholder_sql").to_string(),
                        window,
                        cx
                    );
                });
                self.send_button_state.send_label = t!("AIInput.execute").to_string();
            }
        }
        self.apply_completion_provider(window, cx);
    }

    fn apply_completion_provider(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let schema = self.sql_schema.clone();
        match self.mode {
            InputMode::Agent => {
                let provider = Rc::new(TableMentionCompletionProvider::new(schema));
                self.sql_editor.update(cx, |editor, cx| {
                    editor.set_completion_provider(provider, window, cx);
                });
            }
            InputMode::Sql => {
                let mut provider = DefaultSqlCompletionProvider::new(schema);
                if let Some(info) = self.db_completion_info.clone() {
                    provider = provider.with_db_completion_info(info);
                }
                let provider = Rc::new(provider);
                self.sql_editor.update(cx, |editor, cx| {
                    editor.set_completion_provider(provider, window, cx);
                });
            }
        }
    }

    // ========================================================================
    // 架构同步
    // ========================================================================

    fn update_sql_editor_schema(&mut self, cx: &mut Context<Self>) {
        let Some(conn) = &self.selected_connection else {
            return;
        };

        let (database, schema) = if self.uses_schema_as_database {
            let Some(schema) = self.selected_schema.clone() else {
                return;
            };
            (String::new(), Some(schema))
        } else {
            let Some(database) = self.selected_database.clone() else {
                return;
            };
            let schema = if self.supports_schema {
                self.selected_schema.clone()
            } else {
                None
            };
            (database, schema)
        };

        let global_state = cx.global::<GlobalDbState>().clone();
        let connection_id = conn.id.clone();
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let tables = match global_state
                .list_tables(cx, connection_id.clone(), database.clone(), schema.clone())
                .await
            {
                Ok(tables) => tables,
                Err(_) => return,
            };

            let db_completion_info = global_state
                .get_completion_info(cx, connection_id.clone())
                .ok();

            let mut sql_schema = SqlSchema::default();
            let table_items: Vec<(String, String)> = tables
                .iter()
                .map(|t| (t.name.clone(), t.comment.clone().unwrap_or_default()))
                .collect();
            sql_schema = sql_schema.with_tables(table_items);

            for table in &tables {
                if let Ok(columns) = global_state
                    .list_columns(
                        cx,
                        connection_id.clone(),
                        database.clone(),
                        schema.clone(),
                        table.name.clone(),
                    )
                    .await
                {
                    let column_items: Vec<(String, String, String)> = columns
                        .iter()
                        .map(|c| {
                            (
                                c.name.clone(),
                                c.data_type.clone(),
                                c.comment.clone().unwrap_or_default(),
                            )
                        })
                        .collect();
                    sql_schema = sql_schema.with_table_columns_typed(&table.name, column_items);
                }
            }

            if let Some(entity) = this.upgrade() {
                let _ = cx.update(|cx| {
                    if let Some(window_id) = cx.active_window() {
                        let schema_snapshot = sql_schema.clone();
                        let completion_snapshot = db_completion_info.clone();
                        let _ = cx.update_window(window_id, |_, window, cx| {
                            entity.update(cx, |input, cx| {
                                input.sql_schema = schema_snapshot.clone();
                                input.db_completion_info = completion_snapshot.clone();
                                input.apply_completion_provider(window, cx);
                            });
                        });
                    }
                });
            }
        })
        .detach();
    }

    // ========================================================================
    // 模式切换和提交
    // ========================================================================

    fn toggle_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let new_mode = match self.mode {
            InputMode::Agent => InputMode::Sql,
            InputMode::Sql => InputMode::Agent,
        };
        self.set_mode(new_mode, window, cx);
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.mode {
            InputMode::Agent => {
                let content = self.sql_editor.read(cx).get_text(cx);
                if content.trim().is_empty() {
                    return;
                }
                cx.emit(AIInputEvent::Submit { content });
                self.sql_editor.update(cx, |editor, cx| {
                    editor.set_value(String::new(), window, cx);
                });
            }
            InputMode::Sql => {
                let sql = self.sql_editor.read(cx).get_text(cx);
                if sql.trim().is_empty() {
                    return;
                }
                let Some((connection_id, database, schema)) = self.get_connection_info() else {
                    return;
                };
                cx.emit(AIInputEvent::ExecuteSql {
                    sql,
                    connection_id,
                    database,
                    schema,
                });
                self.sql_editor.update(cx, |editor, cx| {
                    editor.set_value(String::new(), window, cx);
                });
            }
        }
    }

    // ========================================================================
    // 渲染
    // ========================================================================

    fn render_header(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_center()
            .px_3()
            .pt_3()
            .gap_2()
            .child(self.db_selector.clone())
            .child(div().flex_1())
    }

    fn render_input_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .px_3()
            .pt_2()
            .pb_2()
            .min_h(px(80.0))
            .child(
                div()
                    .w_full()
                    .h(px(120.0))
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(cx.theme().border)
                    .overflow_hidden()
                    .child(self.sql_editor.clone()),
            )
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_panel = self.settings_panel.clone();
        let send_button_state = self.send_button_state.clone();
        let entity = cx.entity().downgrade();
        let submit_entity = entity.clone();
        let cancel_entity = entity.clone();

        h_flex()
            .w_full()
            .items_center()
            .px_3()
            .pb_3()
            .gap_2()
            // 左侧：模式切换
            .child(
                Button::new("mode-switch")
                    .icon(self.mode.icon().color())
                    .ghost()
                    .with_size(Size::Small)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.toggle_mode(window, cx);
                    })),
            )
            // Provider 选择器（仅 Agent 模式）
            .when(self.mode == InputMode::Agent, |this| {
                this.child(self.provider_select_state.render())
            })
            // 模型设置按钮（仅 Agent 模式）
            .when(self.mode == InputMode::Agent, |this| {
                this.child(
                    Popover::new("model-settings-popover")
                        .anchor(Corner::BottomLeft)
                        .trigger(
                            Button::new("model-settings-btn")
                                .icon(IconName::Settings)
                                .ghost()
                                .with_size(Size::Small),
                        )
                        .content(move |_state, _window, _cx| {
                            settings_panel.clone()
                        }),
                )
            })
            // 弹性空间
            .child(div().flex_1())
            // 发送按钮（或终止按钮）
            .child(
                SendButton::render(
                    &send_button_state,
                    move |window, app| {
                        if let Some(entity) = submit_entity.upgrade() {
                            let _ = entity.update(app, |this, cx| {
                                this.submit(window, cx);
                            });
                        }
                    },
                    move |window, app| {
                        if let Some(entity) = cancel_entity.upgrade() {
                            let _ = entity.update(app, |this, cx| {
                                cx.emit(AIInputEvent::Cancel);
                                this.set_loading(false, window, cx);
                            });
                        }
                    },
                ),
            )
    }
}

impl EventEmitter<AIInputEvent> for AIInput {}

impl Focusable for AIInput {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AIInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .bg(cx.theme().background)
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .shadow_sm()
            .child(self.render_header(cx))
            .child(self.render_input_area(cx))
            .child(self.render_footer(cx))
    }
}
