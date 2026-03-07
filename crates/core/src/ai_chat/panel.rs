//! AI Chat Panel - 数据库 AI 助手对话面板

use crate::cloud_sync::GlobalCloudUser;
use crate::gpui_tokio::Tokio;
use crate::llm::chat_history::ChatMessage;
use crate::llm::{
    Message, Role,
    chat_history::{MessageRepository, SessionRepository},
    manager::GlobalProviderState,
    storage::ProviderRepository,
};
use crate::storage::{GlobalStorageState, traits::Repository};
use gpui::{
    App, AppContext, AsyncApp, Context, Corner, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable, Size, WindowExt as _,
    button::{Button, ButtonVariants},
    dialog::DialogButtonProps,
    h_flex,
    input::{Input, InputEvent, InputState},
    list::{List, ListState},
    popover::Popover,
    v_flex,
};
use rust_i18n::t;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
// 使用引擎和渲染器
use super::engine::ChatEngine;
use super::rendering::ChatMessageRenderer;
use super::stream::{ChatStreamProcessor, StreamEvent};
// 使用共享组件
use super::components::{
    ModelSettings, ModelSettingsEvent, ModelSettingsPanel, ProviderItem, ProviderSelectEvent,
    ProviderSelectState, SessionData, SessionListConfig, SessionListDelegate, SessionListHost,
};

/// AI 聊天面板的自定义颜色配置
///
/// 用于在终端等需要自定义主题的场景中覆盖默认颜色
#[derive(Clone, Debug)]
pub struct AiChatColors {
    /// 主背景色
    pub background: Hsla,
    /// 主前景色（文字）
    pub foreground: Hsla,
    /// 次要背景色（卡片、列表项）
    pub muted: Hsla,
    /// 次要前景色（占位符、次要文字）
    pub muted_foreground: Hsla,
    /// 边框色
    pub border: Hsla,
    /// 强调背景色
    pub accent: Hsla,
    /// 强调前景色
    pub accent_foreground: Hsla,
}

// ============================================================================
// 代码块操作扩展机制
// ============================================================================

/// 语言匹配器 - 用于匹配代码块的语言类型
#[derive(Clone)]
pub enum LanguageMatcher {
    /// 精确匹配（不区分大小写）
    Exact(Vec<&'static str>),
    /// 前缀匹配
    Prefix(&'static str),
    /// 自定义匹配函数
    Custom(Arc<dyn Fn(&str) -> bool + Send + Sync>),
    /// 匹配所有语言（包括未指定语言的代码块）
    Any,
}

impl LanguageMatcher {
    /// 创建精确匹配器（单个语言）
    pub fn exact(lang: &'static str) -> Self {
        Self::Exact(vec![lang])
    }

    /// 创建精确匹配器（多个语言）
    pub fn exact_many(langs: Vec<&'static str>) -> Self {
        Self::Exact(langs)
    }

    /// 创建 SQL 语言匹配器
    pub fn sql() -> Self {
        Self::Exact(vec![
            "sql",
            "mysql",
            "postgresql",
            "postgres",
            "sqlite",
            "mssql",
            "oracle",
            "plsql",
        ])
    }

    /// 创建 Shell/Bash 语言匹配器
    pub fn shell() -> Self {
        Self::Exact(vec![
            "bash",
            "sh",
            "shell",
            "zsh",
            "fish",
            "powershell",
            "ps1",
            "cmd",
            "batch",
        ])
    }

    /// 创建 Python 语言匹配器
    pub fn python() -> Self {
        Self::Exact(vec!["python", "py", "python3"])
    }

    /// 创建 Rust 语言匹配器
    pub fn rust() -> Self {
        Self::Exact(vec!["rust", "rs"])
    }

    /// 创建 JavaScript/TypeScript 语言匹配器
    pub fn javascript() -> Self {
        Self::Exact(vec!["javascript", "js", "typescript", "ts", "jsx", "tsx"])
    }

    /// 检查是否匹配给定的语言
    pub fn matches(&self, lang: Option<&str>) -> bool {
        match self {
            LanguageMatcher::Exact(langs) => lang.map_or(false, |l| {
                let l_lower = l.to_lowercase();
                langs
                    .iter()
                    .any(|&expected| expected.eq_ignore_ascii_case(&l_lower))
            }),
            LanguageMatcher::Prefix(prefix) => lang.map_or(false, |l| {
                l.to_lowercase().starts_with(&prefix.to_lowercase())
            }),
            LanguageMatcher::Custom(f) => lang.map_or(false, |l| f(l)),
            LanguageMatcher::Any => true,
        }
    }
}

/// 代码块操作回调函数类型
///
/// 参数：
/// - `code`: 代码块内容
/// - `lang`: 代码块语言（可能为空）
/// - `window`: 窗口引用
/// - `cx`: 应用上下文
pub type CodeBlockActionCallback =
    Arc<dyn Fn(String, Option<String>, &mut Window, &mut App) + Send + Sync>;

/// 代码块操作定义
///
/// 用于定义一个可以在代码块上执行的操作，例如：
/// - SQL 代码发送到编辑器
/// - Shell 命令复制到终端
/// - Python 代码直接运行
#[derive(Clone)]
pub struct CodeBlockAction {
    /// 唯一标识符
    pub id: SharedString,
    /// 显示图标
    pub icon: IconName,
    /// 按钮标签（可选，如果为 None 则只显示图标）
    pub label: Option<SharedString>,
    /// 语言匹配器
    pub matcher: LanguageMatcher,
    /// 操作回调
    pub callback: CodeBlockActionCallback,
}

impl CodeBlockAction {
    /// 创建新的代码块操作
    pub fn new(id: impl Into<SharedString>) -> CodeBlockActionBuilder {
        CodeBlockActionBuilder {
            id: id.into(),
            icon: IconName::SquareTerminal,
            label: None,
            matcher: LanguageMatcher::Any,
            callback: None,
        }
    }
}

/// 代码块操作构建器
pub struct CodeBlockActionBuilder {
    id: SharedString,
    icon: IconName,
    label: Option<SharedString>,
    matcher: LanguageMatcher,
    callback: Option<CodeBlockActionCallback>,
}

impl CodeBlockActionBuilder {
    /// 设置图标
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = icon;
        self
    }

    /// 设置标签
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// 设置语言匹配器
    pub fn matcher(mut self, matcher: LanguageMatcher) -> Self {
        self.matcher = matcher;
        self
    }

    /// 设置回调函数
    pub fn on_click<F>(mut self, f: F) -> Self
    where
        F: Fn(String, Option<String>, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.callback = Some(Arc::new(f));
        self
    }

    /// 构建代码块操作
    pub fn build(self) -> Option<CodeBlockAction> {
        self.callback.map(|callback| CodeBlockAction {
            id: self.id,
            icon: self.icon,
            label: self.label,
            matcher: self.matcher,
            callback,
        })
    }
}

/// 代码块操作注册表
///
/// 用于管理和查询已注册的代码块操作
#[derive(Clone, Default)]
pub struct CodeBlockActionRegistry {
    actions: Vec<CodeBlockAction>,
}

impl CodeBlockActionRegistry {
    /// 创建空的注册表
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    /// 注册一个代码块操作
    pub fn register(&mut self, action: CodeBlockAction) {
        self.actions.push(action);
    }

    /// 获取匹配指定语言的所有操作
    pub fn get_actions_for_lang(&self, lang: Option<&str>) -> Vec<&CodeBlockAction> {
        self.actions
            .iter()
            .filter(|action| action.matcher.matches(lang))
            .collect()
    }

    /// 检查是否有注册的操作
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// 获取所有操作数量
    pub fn len(&self) -> usize {
        self.actions.len()
    }
}

/// AI 聊天面板事件
#[derive(Clone, Debug)]
pub enum AiChatPanelEvent {
    Close,
    ExecuteSql {
        sql: String,
        connection_id: String,
        database: Option<String>,
        schema: Option<String>,
    },
}

/// AI 聊天面板
pub struct AiChatPanel {
    focus_handle: FocusHandle,

    /// 共享业务逻辑引擎
    engine: ChatEngine,

    ai_input_state: Entity<InputState>,
    provider_select_state: ProviderSelectState,

    _subscriptions: Vec<Subscription>,
    connection_name: Option<String>,
    database: Option<String>,
    history_popover_open: bool,
    session_list: Option<Entity<ListState<SessionListDelegate<AiChatPanel>>>>,
    /// 可选的自定义颜色（用于终端等需要自定义主题的场景）
    custom_colors: Option<AiChatColors>,
    /// 模型设置面板
    settings_panel: Entity<ModelSettingsPanel>,
    is_logged_in: bool,
    /// 场景专属系统提示词，仅在发送消息时前置注入
    system_instruction: Option<String>,
}

impl AiChatPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // 创建引擎
        let global_state = cx.global::<GlobalStorageState>();
        let engine = ChatEngine::new(global_state.storage.clone());

        // Agent 模式输入框
        let agent_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("AiChat.input_placeholder").to_string())
                .auto_grow(2, 6)
                .default_value("")
        });

        // Provider/Model 选择器（回调直接接收 &mut Self，避免重复借用）
        let provider_select_state =
            ProviderSelectState::new(window, cx, |event, this, window, cx| match event {
                ProviderSelectEvent::ProviderChanged { provider_id, .. } => {
                    this.engine.provider_id = Some(provider_id.clone());
                    this.engine.selected_model = this
                        .provider_select_state
                        .update_models_for_provider(&provider_id, window, cx);
                    cx.notify();
                }
                ProviderSelectEvent::ModelChanged { model } => {
                    this.engine.selected_model = Some(model.clone());
                    cx.notify();
                }
            });

        let mut subscriptions = Vec::new();

        // 订阅 Agent 输入事件
        subscriptions.push(cx.subscribe_in(
            &agent_input_state,
            window,
            |this, _state, event, window, cx| {
                if let InputEvent::PressEnter { secondary } = event {
                    if !secondary {
                        this.submit(window, cx);
                    }
                }
            },
        ));

        // 创建模型设置面板
        let model_settings = ModelSettings::default();
        let settings_panel =
            cx.new(|cx| ModelSettingsPanel::new(model_settings.clone(), window, cx));

        // 订阅模型设置事件
        subscriptions.push(cx.subscribe_in(
            &settings_panel,
            window,
            |this, _panel, event: &ModelSettingsEvent, _window, cx| match event {
                ModelSettingsEvent::Changed(settings) => {
                    this.engine.model_settings = settings.clone();
                    cx.notify();
                }
            },
        ));

        let mut panel = Self {
            focus_handle,
            engine,
            ai_input_state: agent_input_state,
            provider_select_state,
            _subscriptions: subscriptions,
            connection_name: None,
            database: None,
            history_popover_open: false,
            session_list: None,
            custom_colors: None,
            settings_panel,
            is_logged_in: GlobalCloudUser::is_logged_in(cx),
            system_instruction: None,
        };

        // 加载 providers
        panel.load_providers(cx);
        panel
    }

    fn load_providers(&mut self, cx: &mut Context<Self>) {
        let global_state = cx.global::<GlobalStorageState>();
        let storage_manager = global_state.storage.clone();
        let is_logged_in = GlobalCloudUser::is_logged_in(cx);
        self.is_logged_in = is_logged_in;

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let providers = {
                let repo = match storage_manager.get::<ProviderRepository>() {
                    Some(r) => r,
                    None => return,
                };
                let mut list = match repo.list() {
                    Ok(all) => all.into_iter().filter(|p| p.enabled).collect::<Vec<_>>(),
                    Err(_) => Vec::new(),
                };
                if is_logged_in {
                    if let Ok(onet) = repo.ensure_onetcli_provider() {
                        if !list.iter().any(|p| p.id == onet.id) {
                            list.insert(0, onet);
                        }
                    }
                } else {
                    list.retain(|p| !p.is_builtin());
                }
                list
            };

            let _ = cx.update(|cx| {
                if let Some(window_id) = cx.active_window() {
                    let _ = cx.update_window(window_id, |_, window, cx| {
                        if let Some(entity) = this.upgrade() {
                            entity.update(cx, |panel, cx| {
                                panel.engine.provider_configs = providers.clone();
                                let items: Vec<_> =
                                    providers.iter().map(ProviderItem::from_config).collect();
                                panel.provider_select_state.set_providers(items, window, cx);
                                panel.engine.provider_id =
                                    panel.provider_select_state.selected_provider().cloned();
                                panel.engine.selected_model =
                                    panel.provider_select_state.selected_model().cloned();
                                cx.notify();
                            });
                        }
                    });
                }
            });
        })
        .detach();
    }

    pub fn set_connection_info(
        &mut self,
        connection_name: Option<String>,
        database: Option<String>,
    ) {
        self.connection_name = connection_name;
        self.database = database;
    }

    /// 设置场景专属系统提示词
    pub fn set_system_instruction(&mut self, instruction: Option<String>, cx: &mut Context<Self>) {
        self.system_instruction = instruction.and_then(|instruction| {
            let trimmed = instruction.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });
        cx.notify();
    }

    /// 设置自定义颜色（用于终端等需要自定义主题的场景）
    pub fn set_colors(&mut self, colors: AiChatColors, cx: &mut Context<Self>) {
        self.custom_colors = Some(colors);
        cx.notify();
    }

    /// 获取背景色（自定义或默认主题）
    fn background(&self, cx: &App) -> Hsla {
        self.custom_colors
            .as_ref()
            .map(|c| c.background)
            .unwrap_or_else(|| cx.theme().background)
    }

    /// 获取前景色（自定义或默认主题）
    fn foreground(&self, cx: &App) -> Hsla {
        self.custom_colors
            .as_ref()
            .map(|c| c.foreground)
            .unwrap_or_else(|| cx.theme().foreground)
    }

    /// 获取次要背景色（自定义或默认主题）
    fn muted(&self, cx: &App) -> Hsla {
        self.custom_colors
            .as_ref()
            .map(|c| c.muted)
            .unwrap_or_else(|| cx.theme().muted)
    }

    /// 获取边框色（自定义或默认主题）
    fn border(&self, cx: &App) -> Hsla {
        self.custom_colors
            .as_ref()
            .map(|c| c.border)
            .unwrap_or_else(|| cx.theme().border)
    }

    pub fn set_provider_id(&mut self, provider_id: String, cx: &mut Context<Self>) {
        self.engine.provider_id = Some(provider_id.clone());
        if let Some(config) = self
            .engine
            .provider_configs
            .iter()
            .find(|provider| provider.id.to_string() == provider_id)
        {
            let models = ProviderSelectState::build_model_list_from_config(config);
            self.engine.selected_model =
                ProviderSelectState::resolve_default_model_from_config(config, &models);
        }
        cx.notify();
    }

    /// 注册代码块操作
    pub fn register_code_block_action(&mut self, action: CodeBlockAction, cx: &mut Context<Self>) {
        self.engine.code_block_actions.register(action);
        cx.notify();
    }

    /// 批量注册代码块操作
    pub fn register_code_block_actions(
        &mut self,
        actions: Vec<CodeBlockAction>,
        cx: &mut Context<Self>,
    ) {
        for action in actions {
            self.engine.code_block_actions.register(action);
        }
        cx.notify();
    }

    /// 获取代码块操作注册表的引用（用于外部查询）
    pub fn code_block_actions(&self) -> &CodeBlockActionRegistry {
        &self.engine.code_block_actions
    }

    /// 从外部发送消息到AI聊天
    pub fn send_external_message(&mut self, message: String, cx: &mut Context<Self>) {
        if !message.trim().is_empty() {
            self.send_message(message, cx);
        }
    }

    // 创建新会话 - 同步返回，异步保存
    pub fn start_new_session(&mut self, cx: &mut Context<Self>) {
        self.engine.start_new_session();
        cx.notify();
    }

    /// 确保会话存在，如果不存在则创建新会话
    fn ensure_session_id(&mut self, provider_id: &str, cx: &mut Context<Self>) -> Option<i64> {
        let result = self
            .engine
            .ensure_session_id(provider_id, t!("AiChat.new_session_name").as_ref());
        if result.is_some() && self.engine.is_new_session {
            // 新创建的会话，需要刷新历史列表（ensure_session_id 已经设置了 is_new_session）
            self.load_history_sessions(cx);
        }
        result
    }

    /// 持久化用户消息，并在新会话时更新标题
    fn persist_user_message(&mut self, session_id: i64, content: &str, cx: &mut Context<Self>) {
        let was_new = self.engine.is_new_session;
        self.engine.persist_user_message(session_id, content);
        if was_new {
            self.load_history_sessions(cx);
        }
    }

    /// 取消当前操作
    pub fn cancel_current_operation(&mut self, cx: &mut Context<Self>) {
        self.engine.cancel_current_operation();
        cx.notify();
    }

    /// 是否可以取消
    pub fn can_cancel(&self) -> bool {
        self.engine.can_cancel()
    }

    fn update_session_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sessions_data: Vec<SessionData> = self
            .engine
            .history_sessions
            .iter()
            .map(|s| SessionData::new(s.id, s.name.clone(), s.updated_at))
            .collect();
        let panel = cx.entity();

        if let Some(session_list) = &self.session_list {
            session_list.update(cx, |state, _| {
                let delegate = state.delegate_mut();
                delegate.update_sessions(sessions_data);
            });
        } else {
            self.session_list = Some(cx.new(|cx| {
                ListState::new(
                    SessionListDelegate::new(panel, sessions_data, SessionListConfig::default()),
                    window,
                    cx,
                )
                .searchable(true)
            }));
        }
    }

    fn load_history_sessions(&mut self, cx: &mut Context<Self>) {
        let global_state = cx.global::<GlobalStorageState>();
        let storage_manager = global_state.storage.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let sessions = {
                let session_repo = match storage_manager.get::<SessionRepository>() {
                    Some(r) => r,
                    None => return,
                };
                match session_repo.list() {
                    Ok(s) => s,
                    Err(_) => return,
                }
            };

            if let Some(entity) = this.upgrade() {
                let _ = cx.update(|cx| {
                    if let Some(window_id) = cx.active_window() {
                        let _ = cx.update_window(window_id, |_, window, cx| {
                            entity.update(cx, |this, cx| {
                                this.engine.history_sessions = sessions;
                                this.update_session_list(window, cx);
                                cx.notify();
                            });
                        });
                    }
                });
            }
        })
        .detach();
    }

    fn delete_session(&mut self, session_id: i64, cx: &mut Context<Self>) {
        let global_state = cx.global::<GlobalStorageState>();
        let storage_manager = global_state.storage.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let delete_ok = {
                let session_repo = match storage_manager.get::<SessionRepository>() {
                    Some(r) => r,
                    None => return,
                };
                let message_repo = match storage_manager.get::<MessageRepository>() {
                    Some(r) => r,
                    None => return,
                };
                message_repo.delete_by_session(session_id).is_ok()
                    && session_repo.delete(session_id).is_ok()
            };

            if delete_ok {
                if let Some(entity) = this.upgrade() {
                    let _ = cx.update(|cx| {
                        entity.update(cx, |this, cx| {
                            if this.engine.session_id == Some(session_id) {
                                this.engine.session_id = None;
                                this.engine.messages.clear();
                            }
                            this.engine.history_sessions.retain(|s| s.id != session_id);
                            cx.notify();
                        });
                    });
                }
            }
        })
        .detach();
    }

    fn start_rename_session(
        &mut self,
        session_id: i64,
        current_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(1)
                .default_value(&current_name)
                .placeholder(t!("AiChat.session_name_placeholder").to_string())
        });

        let panel_entity = cx.entity();
        let input_for_dialog = input_state.clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let input_for_ok = input_for_dialog.clone();
            let panel_for_ok = panel_entity.clone();

            dialog
                .title(t!("AiChat.rename_session_title").to_string())
                .w(px(360.0))
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.save").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, _window, cx| {
                    let new_name = input_for_ok.read(cx).value().to_string();
                    if !new_name.trim().is_empty() {
                        panel_for_ok.update(cx, |this, cx| {
                            this.rename_session(session_id, new_name, cx);
                        });
                    }
                    true
                })
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .child(t!("AiChat.rename_session_prompt").to_string()),
                        )
                        .child(Input::new(&input_for_dialog).w_full()),
                )
        });
    }

    fn rename_session(&mut self, session_id: i64, new_name: String, cx: &mut Context<Self>) {
        let global_state = cx.global::<GlobalStorageState>();
        let storage_manager = global_state.storage.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let renamed = {
                let session_repo = match storage_manager.get::<SessionRepository>() {
                    Some(r) => r,
                    None => return,
                };
                if let Ok(Some(mut session)) = session_repo.get(session_id) {
                    session.name = new_name;
                    session_repo.update(&session).is_ok()
                } else {
                    false
                }
            };

            if renamed {
                if let Some(entity) = this.upgrade() {
                    let _ = cx.update(|cx| {
                        entity.update(cx, |this, cx| {
                            this.load_history_sessions(cx);
                        });
                    });
                }
            }
        })
        .detach();
    }

    fn load_session(&mut self, session_id: i64, cx: &mut Context<Self>) {
        let global_state = cx.global::<GlobalStorageState>();
        let storage_manager = global_state.storage.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let messages = {
                let message_repo = match storage_manager.get::<MessageRepository>() {
                    Some(r) => r,
                    None => return,
                };
                match message_repo.list_by_session(session_id) {
                    Ok(m) => m,
                    Err(_) => return,
                }
            };

            if let Some(entity) = this.upgrade() {
                let _ = cx.update(|cx| {
                    entity.update(cx, |this, cx| {
                        this.engine.session_id = Some(session_id);
                        this.engine.messages = ChatEngine::messages_from_history(&messages);
                        this.history_popover_open = false;
                        cx.notify();
                    });
                });
            }
        })
        .detach();
    }

    fn send_message(&mut self, content: String, cx: &mut Context<Self>) {
        if content.trim().is_empty() || self.engine.is_loading {
            return;
        }

        let Some(provider_id_str) = self.engine.provider_id.clone() else {
            self.engine
                .push_assistant(t!("AiChat.select_provider_first").to_string());
            cx.notify();
            return;
        };

        let provider_id: i64 = match provider_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                self.engine
                    .push_assistant(t!("AiChat.invalid_provider_id").to_string());
                cx.notify();
                return;
            }
        };

        // 确保会话存在并持久化用户消息
        if let Some(session_id) = self.ensure_session_id(&provider_id_str, cx) {
            self.persist_user_message(session_id, &content, cx);
        }

        let global_provider_state = cx.global::<GlobalProviderState>().clone();
        let global_state = cx.global::<GlobalStorageState>();
        let storage_manager = global_state.storage.clone();
        let session_id = self.engine.session_id;
        let history_count = self.engine.model_settings.history_count;
        let max_tokens = self.engine.model_settings.max_tokens;
        let temperature = self.engine.model_settings.temperature;
        let system_instruction = self.system_instruction.clone();

        // 获取用户选择的模型
        let selected_model = self.engine.selected_model.clone().unwrap_or_else(|| {
            self.engine
                .provider_configs
                .iter()
                .find(|c| c.id == provider_id)
                .map(|c| c.model.clone())
                .unwrap_or_default()
        });

        // 添加用户消息到 UI 并创建助手消息占位符
        self.engine.push_user_message(content.clone());
        let assistant_msg_id = self.engine.push_streaming_assistant();

        self.engine.auto_scroll_enabled = true;
        self.engine.is_loading = true;

        // 创建取消令牌
        let cancel_token = CancellationToken::new();
        self.engine.cancel_token = Some(cancel_token.clone());

        self.engine.scroll_to_bottom();
        cx.notify();

        // 获取 Tokio runtime handle
        let tokio_handle = Tokio::handle(cx);

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            // 进入 Tokio runtime 上下文
            let _guard = tokio_handle.enter();

            if cancel_token.is_cancelled() {
                return;
            }

            // 构建聊天历史消息
            let messages: Vec<Message> = {
                let mut messages = if let Some(sid) = session_id {
                    if let Some(message_repo) = storage_manager.get::<MessageRepository>() {
                        match message_repo.list_by_session(sid) {
                            Ok(messages) => {
                                let mut msgs: Vec<Message> = messages
                                    .iter()
                                    .map(|msg| {
                                        let role = match msg.role.as_str() {
                                            "user" => Role::User,
                                            "assistant" => Role::Assistant,
                                            "system" => Role::System,
                                            _ => Role::User,
                                        };
                                        Message::text(role, &msg.content)
                                    })
                                    .collect();
                                // 限制历史条数
                                if msgs.len() > history_count {
                                    msgs = msgs.split_off(msgs.len() - history_count);
                                }
                                msgs
                            }
                            Err(_) => vec![Message::text(Role::User, &content)],
                        }
                    } else {
                        vec![Message::text(Role::User, &content)]
                    }
                } else {
                    vec![Message::text(Role::User, &content)]
                };

                if let Some(instruction) = system_instruction.as_deref() {
                    messages.insert(0, Message::text(Role::System, instruction));
                }

                messages
            };

            // 直接使用 ChatStreamProcessor 进行流式对话
            let mut rx = match ChatStreamProcessor::start(
                provider_id,
                Some(selected_model),
                messages,
                max_tokens as u32,
                temperature,
                cancel_token,
                global_provider_state,
                storage_manager.clone(),
            )
            .await
            {
                Ok(rx) => rx,
                Err(e) => {
                    if let Some(entity) = this.upgrade() {
                        let msg_id = assistant_msg_id.clone();
                        let error_msg = e.to_string();
                        let _ = cx.update(|cx| {
                            entity.update(cx, |this, cx| {
                                this.engine.set_message_error(&msg_id, error_msg);
                                this.engine.is_loading = false;
                                this.engine.cancel_token = None;
                                cx.notify();
                            });
                        });
                    }
                    return;
                }
            };

            // 处理流式事件
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::ContentDelta { full_content, .. } => {
                        if let Some(entity) = this.upgrade() {
                            let msg_id = assistant_msg_id.clone();
                            cx.update(|cx| {
                                entity.update(cx, |this, cx| {
                                    this.engine.update_streaming_content(&msg_id, full_content);
                                    this.engine.scroll_to_bottom();
                                    cx.notify();
                                })
                            });
                        } else {
                            return;
                        }
                    }
                    StreamEvent::Completed { full_content } => {
                        if let Some(entity) = this.upgrade() {
                            let msg_id = assistant_msg_id.clone();
                            let storage_for_save = storage_manager.clone();
                            cx.update(|cx| {
                                entity.update(cx, |this, cx| {
                                    this.engine
                                        .finalize_streaming(&msg_id, full_content.clone());
                                    this.engine.is_loading = false;
                                    this.engine.cancel_token = None;
                                    this.engine.scroll_to_bottom();

                                    // 持久化助手消息
                                    if let Some(sid) = session_id {
                                        let content_to_save = full_content;
                                        let storage = storage_for_save;
                                        cx.spawn(async move |_this, _cx: &mut AsyncApp| {
                                            if let Some(repo) = storage.get::<MessageRepository>() {
                                                let mut msg = ChatMessage::new(
                                                    sid,
                                                    "assistant".to_string(),
                                                    content_to_save,
                                                );
                                                if let Err(e) = repo.insert(&mut msg) {
                                                    warn!(
                                                        "Failed to save assistant message: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        })
                                        .detach();
                                    }

                                    cx.notify();
                                })
                            });
                        }
                        break;
                    }
                    StreamEvent::Error { message } => {
                        if let Some(entity) = this.upgrade() {
                            let msg_id = assistant_msg_id.clone();
                            cx.update(|cx| {
                                entity.update(cx, |this, cx| {
                                    this.engine.set_message_error(&msg_id, message);
                                    this.engine.is_loading = false;
                                    this.engine.cancel_token = None;
                                    this.engine.scroll_to_bottom();
                                    cx.notify();
                                })
                            });
                        }
                        break;
                    }
                    StreamEvent::Cancelled => {
                        info!("Stream cancelled by user");
                        if let Some(entity) = this.upgrade() {
                            let _ = cx.update(|cx| {
                                entity.update(cx, |this, cx| {
                                    this.engine.is_loading = false;
                                    this.engine.cancel_token = None;
                                    cx.notify();
                                });
                            });
                        }
                        break;
                    }
                }
            }
        })
        .detach();
    }

    fn render_header(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border = self.border(cx);
        let muted = self.muted(cx);
        let fg = self.foreground(cx);
        let session_list = self.session_list.clone();

        h_flex()
            .flex_shrink_0()
            .w_full()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(border)
            .bg(muted)
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(fg)
                    .child(t!("AiChat.title").to_string()),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("new-session")
                            .icon(IconName::Plus)
                            .ghost()
                            .small()
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.start_new_session(cx);
                            })),
                    )
                    .child(
                        Popover::new("history-popover")
                            .anchor(Corner::TopRight)
                            .p_0()
                            .open(self.history_popover_open)
                            .on_open_change(cx.listener(|this, open, window, cx| {
                                this.history_popover_open = *open;
                                if *open {
                                    this.update_session_list(window, cx);
                                    this.load_history_sessions(cx);
                                }
                                cx.notify();
                            }))
                            .when_some(session_list.as_ref(), |popover, list| {
                                popover.track_focus(&list.focus_handle(cx))
                            })
                            .trigger(
                                Button::new("history")
                                    .icon(IconName::BookOpen)
                                    .ghost()
                                    .small(),
                            )
                            .when_some(session_list, |popover, list| {
                                popover.child(
                                    List::new(&list)
                                        .w(px(280.0))
                                        .max_h(px(350.0))
                                        .border_1()
                                        .border_color(border)
                                        .rounded(cx.theme().radius),
                                )
                            }),
                    )
                    .child(
                        Button::new("close-panel")
                            .icon(IconName::Close)
                            .ghost()
                            .small()
                            .on_click(cx.listener(|_this, _event, _window, cx| {
                                cx.emit(AiChatPanelEvent::Close);
                            })),
                    ),
            )
    }

    fn render_messages(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let code_block_actions = self.engine.code_block_actions.clone();

        div()
            .id("chat-messages-list")
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll()
            .track_scroll(&self.engine.scroll_handle)
            .p_4()
            .pb_8()
            .child(
                v_flex().w_full().gap_4().children(
                    self.engine.messages.iter().map(|msg| {
                        ChatMessageRenderer::render_message(msg, &code_block_actions, cx)
                    }),
                ),
            )
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let content = self.ai_input_state.read(cx).value().to_string();
        if content.trim().is_empty() {
            return;
        }
        self.send_message(content, cx);
        self.ai_input_state.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
    }

    fn render_input(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border = self.border(cx);
        let bg = self.background(cx);
        let muted = self.muted(cx);

        v_flex()
            .flex_shrink_0()
            .w_full()
            .px_3()
            .py_2()
            .gap_2()
            .border_t_1()
            .border_color(border)
            .bg(bg)
            // 输入框
            .child(
                Input::new(&self.ai_input_state)
                    .w_full()
                    .with_size(Size::Large)
                    .bordered(false)
                    .appearance(false)
                    .bg(muted)
                    .rounded(cx.theme().radius),
            )
            // 底部工具栏
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .flex_1()
                            .gap_2()
                            .min_w_0()
                            .overflow_hidden()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(self.provider_select_state.render_provider_select()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(self.provider_select_state.render_model_select()),
                            )
                            // 模型设置按钮
                            .child({
                                let settings_panel = self.settings_panel.clone();
                                Popover::new("model-settings-popover")
                                    .anchor(Corner::BottomLeft)
                                    .trigger(
                                        Button::new("model-settings-btn")
                                            .icon(IconName::Settings)
                                            .ghost()
                                            .with_size(Size::Small),
                                    )
                                    .content(move |_state, _window, _cx| settings_panel.clone())
                            }),
                    )
                    .child(if self.can_cancel() {
                        // 加载中显示终止按钮
                        Button::new("cancel")
                            .with_size(Size::Small)
                            .danger()
                            .icon(IconName::CircleX)
                            .label(t!("AiChat.cancel").to_string())
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.cancel_current_operation(cx);
                            }))
                    } else {
                        // 正常状态显示发送按钮
                        Button::new("send")
                            .with_size(Size::Small)
                            .primary()
                            .icon(IconName::ArrowRight)
                            .label(t!("AiChat.send").to_string())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.submit(window, cx);
                            }))
                    }),
            )
    }
}

impl EventEmitter<AiChatPanelEvent> for AiChatPanel {}

impl Focusable for AiChatPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl SessionListHost for AiChatPanel {
    fn on_session_select(&mut self, session_id: i64, cx: &mut Context<Self>) {
        self.load_session(session_id, cx);
    }

    fn on_session_edit(
        &mut self,
        session_id: i64,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_rename_session(session_id, name, window, cx);
    }

    fn on_session_delete(&mut self, session_id: i64, cx: &mut Context<Self>) {
        self.delete_session(session_id, cx);
    }

    fn is_current_session(&self, session_id: i64) -> bool {
        self.engine.session_id == Some(session_id)
    }

    fn on_session_list_confirm(&mut self, cx: &mut Context<Self>) {
        self.history_popover_open = false;
        cx.notify();
    }

    fn on_session_list_cancel(&mut self, cx: &mut Context<Self>) {
        self.history_popover_open = false;
        cx.notify();
    }
}

impl Render for AiChatPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_logged_in = GlobalCloudUser::is_logged_in(cx);
        if is_logged_in != self.is_logged_in {
            self.is_logged_in = is_logged_in;
            self.load_providers(cx);
        }

        let bg_color = self.background(cx);
        let fg_color = self.foreground(cx);

        div().size_full().child(
            v_flex()
                .size_full()
                .bg(bg_color)
                .text_color(fg_color)
                .child(self.render_header(window, cx))
                .child(self.render_messages(window, cx))
                .child(self.render_input(window, cx)),
        )
    }
}
