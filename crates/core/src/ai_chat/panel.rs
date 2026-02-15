//! AI Chat Panel - 数据库 AI 助手对话面板

use std::sync::Arc;
use futures::StreamExt;
use gpui::{div, prelude::FluentBuilder, px, AnyElement, App, AppContext, AsyncApp, Context, Corner, Entity, EventEmitter, FocusHandle, Focusable, Hsla, InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Subscription, Window};
use gpui_component::{
    button::{Button, ButtonVariants},
    clipboard::Clipboard,
    dialog::DialogButtonProps,
    h_flex,
    input::{Input, InputEvent, InputState},
    list::{List, ListState},
    popover::Popover,
    text::TextView,
    v_flex,
    ActiveTheme,
    Icon, IconName, Sizable, Size,
    WindowExt as _,
};
use tracing::{debug, error, info, warn};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use crate::llm::ProviderConfig;
use crate::llm::{
    chat_history::{ChatMessage, ChatSession, MessageRepository, SessionRepository},
    manager::GlobalProviderState,
    storage::ProviderRepository,
    ChatRequest, Message, MessageBlock, Role,
};
use crate::storage::{traits::Repository, GlobalStorageState};
use crate::gpui_tokio::Tokio;

// 使用共享类型
use super::types::{ChatMessageUI, ChatRole, MessageVariant};
// 使用共享组件
use super::components::{
    ProviderSelectState, ProviderSelectEvent,
    SessionData, SessionListConfig, SessionListDelegate, SessionListHost,
    ModelSettings, ModelSettingsPanel, ModelSettingsEvent,
};
// 使用共享服务
use super::services::{SessionService, extract_session_name};

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
        Self::Exact(vec!["sql", "mysql", "postgresql", "postgres", "sqlite", "mssql", "oracle", "plsql"])
    }

    /// 创建 Shell/Bash 语言匹配器
    pub fn shell() -> Self {
        Self::Exact(vec!["bash", "sh", "shell", "zsh", "fish", "powershell", "ps1", "cmd", "batch"])
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
            LanguageMatcher::Exact(langs) => {
                lang.map_or(false, |l| {
                    let l_lower = l.to_lowercase();
                    langs.iter().any(|&expected| expected.eq_ignore_ascii_case(&l_lower))
                })
            }
            LanguageMatcher::Prefix(prefix) => {
                lang.map_or(false, |l| l.to_lowercase().starts_with(&prefix.to_lowercase()))
            }
            LanguageMatcher::Custom(f) => {
                lang.map_or(false, |l| f(l))
            }
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
pub type CodeBlockActionCallback = Arc<dyn Fn(String, Option<String>, &mut Window, &mut App) + Send + Sync>;

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
        Self { actions: Vec::new() }
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
    messages: Vec<ChatMessageUI>,

    ai_input_state: Entity<InputState>,
    provider_select_state: ProviderSelectState,
    provider_configs: Vec<ProviderConfig>,

    _subscriptions: Vec<Subscription>,
    session_id: Option<i64>,
    provider_id: Option<String>,
    selected_model: Option<String>,
    connection_name: Option<String>,
    database: Option<String>,
    is_loading: bool,
    scroll_handle: ScrollHandle,
    history_sessions: Vec<ChatSession>,
    auto_scroll_enabled: bool,
    history_popover_open: bool,
    session_list: Option<Entity<ListState<SessionListDelegate<AiChatPanel>>>>,
    /// 可选的自定义颜色（用于终端等需要自定义主题的场景）
    custom_colors: Option<AiChatColors>,
    /// 代码块操作注册表
    code_block_actions: CodeBlockActionRegistry,
    /// 取消令牌，用于终止正在进行的请求
    cancel_token: Option<CancellationToken>,
    /// 模型设置
    model_settings: ModelSettings,
    /// 模型设置面板
    settings_panel: Entity<ModelSettingsPanel>,
    /// 会话服务
    session_service: SessionService,
    /// 是否为新会话（等待第一条消息更新名称）
    is_new_session: bool,
}


impl AiChatPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Agent 模式输入框
        let agent_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("向 AI 提问... (Enter 发送)")
                .auto_grow(2, 6)
                .default_value("")
        });

        // Provider/Model 选择器（回调直接接收 &mut Self，避免重复借用）
        let provider_select_state = ProviderSelectState::new(window, cx, |event, this, window, cx| {
            match event {
                ProviderSelectEvent::ProviderChanged { provider_id, .. } => {
                    this.provider_id = Some(provider_id.clone());
                    this.selected_model = this.provider_select_state
                        .update_models_for_provider(&provider_id, window, cx);
                    cx.notify();
                }
                ProviderSelectEvent::ModelChanged { model } => {
                    this.selected_model = Some(model.clone());
                    cx.notify();
                }
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
        let settings_panel = cx.new(|cx| {
            ModelSettingsPanel::new(model_settings.clone(), window, cx)
        });

        // 订阅模型设置事件
        subscriptions.push(cx.subscribe_in(
            &settings_panel,
            window,
            |this, _panel, event: &ModelSettingsEvent, _window, cx| {
                match event { ModelSettingsEvent::Changed(settings) => {
                    this.model_settings = settings.clone();
                    cx.notify();
                    }
                }
            },
        ));

        // 创建会话服务
        let global_state = cx.global::<GlobalStorageState>();
        let session_service = SessionService::new(global_state.storage.clone());

        let mut panel = Self {
            focus_handle,
            messages: Vec::new(),
            ai_input_state: agent_input_state,
            provider_select_state,
            provider_configs: Vec::new(),
            _subscriptions: subscriptions,
            session_id: None,
            provider_id: None,
            selected_model: None,
            connection_name: None,
            database: None,
            is_loading: false,
            scroll_handle: ScrollHandle::new(),
            history_sessions: Vec::new(),
            auto_scroll_enabled: true,
            history_popover_open: false,
            session_list: None,
            custom_colors: None,
            code_block_actions: CodeBlockActionRegistry::new(),
            cancel_token: None,
            model_settings,
            settings_panel,
            session_service,
            is_new_session: false,
        };

        // 加载 providers
        panel.load_providers(cx);
        panel
    }

    fn load_providers(&mut self, cx: &mut Context<Self>) {
        let global_state = cx.global::<GlobalStorageState>();
        let storage_manager = global_state.storage.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let providers = {
                let repo = match storage_manager.get::<ProviderRepository>() {
                    Some(r) => r,
                    None => return,
                };
                match repo.list() {
                    Ok(all) => all.into_iter().filter(|p| p.enabled).collect::<Vec<_>>(),
                    Err(_) => Vec::new(),
                }
            };

            let _ = cx.update(|cx| {
                if let Some(window_id) = cx.active_window() {
                    let _ = cx.update_window(window_id, |_, window, cx| {
                        if let Some(entity) = this.upgrade() {
                            entity.update(cx, |panel, cx| {
                                panel.provider_configs = {
                                    let mut configs = providers.clone();
                                    // 添加内置的 OnetCli provider（始终在最前面）
                                    configs.insert(0, ProviderConfig::builtin_onet_cli());
                                    configs
                                };
                                // 使用组件统一设置 providers 和 models
                                panel.provider_select_state.set_provider_configs(&panel.provider_configs, window, cx);
                                panel.provider_id = panel.provider_select_state.selected_provider().cloned();
                                panel.selected_model = panel.provider_select_state.selected_model().cloned();
                                cx.notify();
                            });
                        }
                    });
                }
            });
        })
        .detach();
    }

    pub fn set_connection_info(&mut self, connection_name: Option<String>, database: Option<String>) {
        self.connection_name = connection_name;
        self.database = database;
    }

    /// 设置自定义颜色（用于终端等需要自定义主题的场景）
    pub fn set_colors(&mut self, colors: AiChatColors, cx: &mut Context<Self>) {
        self.custom_colors = Some(colors);
        cx.notify();
    }

    /// 获取背景色（自定义或默认主题）
    fn background(&self, cx: &App) -> Hsla {
        self.custom_colors.as_ref().map(|c| c.background).unwrap_or_else(|| cx.theme().background)
    }

    /// 获取前景色（自定义或默认主题）
    fn foreground(&self, cx: &App) -> Hsla {
        self.custom_colors.as_ref().map(|c| c.foreground).unwrap_or_else(|| cx.theme().foreground)
    }

    /// 获取次要背景色（自定义或默认主题）
    fn muted(&self, cx: &App) -> Hsla {
        self.custom_colors.as_ref().map(|c| c.muted).unwrap_or_else(|| cx.theme().muted)
    }

    /// 获取边框色（自定义或默认主题）
    fn border(&self, cx: &App) -> Hsla {
        self.custom_colors.as_ref().map(|c| c.border).unwrap_or_else(|| cx.theme().border)
    }


    pub fn set_provider_id(&mut self, provider_id: String, cx: &mut Context<Self>) {
        self.provider_id = Some(provider_id.clone());
        // 使用 ProviderSelectState 的静态方法从 config 计算模型
        if let Some(config) = self
            .provider_configs
            .iter()
            .find(|provider| provider.id.to_string() == provider_id)
        {
            let models = ProviderSelectState::build_model_list_from_config(config);
            self.selected_model = ProviderSelectState::resolve_default_model_from_config(config, &models);
        }
        cx.notify();
    }

    /// 注册代码块操作
    ///
    /// 注册后，当 AI 回复的代码块语言匹配时，会显示对应的操作按钮。
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 注册 SQL 操作
    /// panel.register_code_block_action(
    ///     CodeBlockAction::new("send-to-editor")
    ///         .icon(IconName::Edit)
    ///         .label("发送到编辑器")
    ///         .matcher(LanguageMatcher::sql())
    ///         .on_click(|code, _lang, _window, _cx| {
    ///             println!("SQL: {}", code);
    ///         })
    ///         .build()
    ///         .unwrap(),
    ///     cx,
    /// );
    /// ```
    pub fn register_code_block_action(&mut self, action: CodeBlockAction, cx: &mut Context<Self>) {
        self.code_block_actions.register(action);
        cx.notify();
    }

    /// 批量注册代码块操作
    pub fn register_code_block_actions(&mut self, actions: Vec<CodeBlockAction>, cx: &mut Context<Self>) {
        for action in actions {
            self.code_block_actions.register(action);
        }
        cx.notify();
    }

    /// 获取代码块操作注册表的引用（用于外部查询）
    pub fn code_block_actions(&self) -> &CodeBlockActionRegistry {
        &self.code_block_actions
    }

    /// 从外部发送消息到AI聊天
    pub fn send_external_message(&mut self, message: String, cx: &mut Context<Self>) {
        if !message.trim().is_empty() {
            self.send_message(message, cx);
        }
    }

    // 创建新会话 - 同步返回，异步保存
    pub fn start_new_session(&mut self, cx: &mut Context<Self>) {
        self.session_id = None;
        self.is_new_session = false;
        self.messages.clear();
        cx.notify();
    }

    /// 确保会话存在，如果不存在则创建新会话
    fn ensure_session_id(&mut self, provider_id: &str, cx: &mut Context<Self>) -> Option<i64> {
        if let Some(id) = self.session_id {
            return Some(id);
        }

        match self.session_service.ensure_session(None, provider_id, "新会话") {
            Ok(id) => {
                self.session_id = Some(id);
                self.is_new_session = true; // 标记为新会话，等待第一条消息更新名称
                self.load_history_sessions(cx);
                Some(id)
            }
            Err(e) => {
                warn!("创建会话失败: {}", e);
                None
            }
        }
    }

    /// 持久化用户消息，并在新会话时更新标题
    fn persist_user_message(&mut self, session_id: i64, content: String, cx: &mut Context<Self>) {
        let _ = self.session_service.add_user_message(session_id, content.clone());

        // 如果是新会话的第一条消息，使用消息内容的前几个字作为会话名称
        if self.is_new_session {
            self.is_new_session = false;
            let session_name = extract_session_name(&content);
            if self.session_service.update_session_name(session_id, session_name).is_ok() {
                self.load_history_sessions(cx);
            }
        }
    }

    /// 取消当前操作
    pub fn cancel_current_operation(&mut self, cx: &mut Context<Self>) {
        // 取消正在进行的请求
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }

        // 重置状态
        self.is_loading = false;

        // 更新最后一条流式消息为取消状态
        if let Some(msg) = self.messages.iter_mut().rev().find(|m| m.is_streaming) {
            msg.is_streaming = false;
            if msg.content.is_empty() {
                msg.content = "操作已取消".to_string();
            } else {
                msg.content.push_str("\n\n*操作已取消*");
            }
        }

        cx.notify();
    }

    /// 是否可以取消
    pub fn can_cancel(&self) -> bool {
        self.is_loading && self.cancel_token.is_some()
    }

    fn update_session_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sessions_data: Vec<SessionData> = self.history_sessions
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
                ).searchable(true)
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
                                this.history_sessions = sessions;
                                this.update_session_list(window, cx);
                                cx.notify();
                            });
                        });
                    }
                });
            }
        }).detach();
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
                // 先删除消息，再删除会话
                message_repo.delete_by_session(session_id).is_ok()
                    && session_repo.delete(session_id).is_ok()
            };

            if delete_ok {
                if let Some(entity) = this.upgrade() {
                    let _ = cx.update(|cx| {
                        entity.update(cx, |this, cx| {
                            // 如果删除的是当前会话，清空界面
                            if this.session_id == Some(session_id) {
                                this.session_id = None;
                                this.messages.clear();
                            }
                            // 从历史列表中移除
                            this.history_sessions.retain(|s| s.id != session_id);
                            cx.notify();
                        });
                    });
                }
            }
        }).detach();
    }

    fn start_rename_session(&mut self, session_id: i64, current_name: String, window: &mut Window, cx: &mut Context<Self>) {
        // 创建输入框状态
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(&current_name)
                .placeholder("会话名称")
        });

        let panel_entity = cx.entity();
        let input_for_dialog = input_state.clone();

        // 打开重命名对话框
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let input_for_ok = input_for_dialog.clone();
            let panel_for_ok = panel_entity.clone();

            dialog
                .title("重命名会话")
                .w(px(360.0))
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("保存")
                        .cancel_text("取消")
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
                                .child("请输入新的会话名称：")
                        )
                        .child(
                            Input::new(&input_for_dialog)
                                .w_full()
                        )
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
                            // 重新加载历史会话列表
                            this.load_history_sessions(cx);
                        });
                    });
                }
            }
        }).detach();
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
                        this.session_id = Some(session_id);
                        this.messages = messages.iter()
                            .map(|msg| {
                                let role = match msg.role.as_str() {
                                    "user" => ChatRole::User,
                                    "assistant" => ChatRole::Assistant,
                                    "system" => ChatRole::System,
                                    _ => ChatRole::User,
                                };
                                ChatMessageUI::from_history(msg.id.to_string(), role, msg.content.clone())
                            })
                            .collect();
                        this.history_popover_open = false;
                        cx.notify();
                    });
                });
            }
        }).detach();
    }

    fn send_message(&mut self, content: String, cx: &mut Context<Self>) {
        if content.trim().is_empty() || self.is_loading {
            return;
        }

        let Some(provider_id_str) = self.provider_id.clone() else {
            self.messages.push(ChatMessageUI::assistant("请先选择 AI 提供商".to_string()));
            cx.notify();
            return;
        };

        let provider_id: i64 = match provider_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                self.messages.push(ChatMessageUI::assistant("无效的提供商 ID".to_string()));
                cx.notify();
                return;
            }
        };

        // 确保会话存在并持久化用户消息
        if let Some(session_id) = self.ensure_session_id(&provider_id_str, cx) {
            self.persist_user_message(session_id, content.clone(), cx);
        }

        let global_provider_state = cx.global::<GlobalProviderState>().clone();
        let global_state = cx.global::<GlobalStorageState>();
        let storage_manager = global_state.storage.clone();
        let connection_name = self.connection_name.clone();
        let session_id = self.session_id;
        let selected_model = self.selected_model.clone();
        let history_count = self.model_settings.history_count;
        let max_tokens = self.model_settings.max_tokens;
        let temperature = self.model_settings.temperature;

        // 添加用户消息到 UI
        self.messages.push(ChatMessageUI::user(content.clone()));

        // 创建助手消息占位符
        let assistant_msg_id = Uuid::new_v4().to_string();
        self.messages.push(
            ChatMessageUI::streaming_assistant()
                .with_id(assistant_msg_id.clone())
        );

        self.auto_scroll_enabled = true;
        self.is_loading = true;

        // 创建取消令牌
        let cancel_token = CancellationToken::new();
        self.cancel_token = Some(cancel_token.clone());

        self.auto_scroll_to_bottom();
        cx.notify();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            // 检查是否已取消
            if cancel_token.is_cancelled() {
                return;
            }

            // 获取或创建会话
            let session_db_id = match session_id {
                Some(id) => id,
                None => {
                    let session_repo = match storage_manager.get::<SessionRepository>() {
                        Some(r) => r,
                        None => {
                            if let Some(entity) = this.upgrade() {
                                let _ = cx.update(|cx| {
                                    entity.update(cx, |this, cx| {
                                        if let Some(msg) = this.messages.iter_mut().find(|m| m.id == assistant_msg_id) {
                                            msg.is_streaming = false;
                                            msg.content = "SessionRepository not found".to_string();
                                        }
                                        this.is_loading = false;
                                        cx.notify();
                                    });
                                });
                            }
                            return;
                        }
                    };
                    let session_name = format!("Chat with {}", connection_name.as_deref().unwrap_or("Database"));
                    let mut session = ChatSession::new(session_name, provider_id.to_string());
                    match session_repo.insert(&mut session) {
                        Ok(id) => {
                            // 更新 UI 中的 session_id
                            if let Some(entity) = this.upgrade() {
                                let _ = cx.update(|cx| {
                                    entity.update(cx, |this, cx| {
                                        this.session_id = Some(id);
                                        cx.notify();
                                    });
                                });
                            }
                            id
                        }
                        Err(_) => {
                            if let Some(entity) = this.upgrade() {
                                let _ = cx.update(|cx| {
                                    entity.update(cx, |this, cx| {
                                        if let Some(msg) = this.messages.iter_mut().find(|m| m.id == assistant_msg_id) {
                                            msg.is_streaming = false;
                                            msg.content = "Failed to create session.".to_string();
                                        }
                                        this.is_loading = false;
                                        cx.notify();
                                    });
                                });
                            }
                            return;
                        }
                    }
                }
            };

            // 保存用户消息
            {
                let content_clone = content.clone();
                if let Some(message_repo) = storage_manager.get::<MessageRepository>() {
                    let mut message = ChatMessage::new(session_db_id, "user".to_string(), content_clone);
                    if let Err(e) = message_repo.insert(&mut message) {
                        error!("Failed to save user message: {}", e);
                    }
                }
            }

            // 获取聊天历史（不包含当前消息）
            let mut history: Vec<Message> = {
                if let Some(message_repo) = storage_manager.get::<MessageRepository>() {
                    match message_repo.list_by_session(session_db_id) {
                        Ok(messages) => {
                            messages.iter().map(|msg| {
                                let role = match msg.role.as_str() {
                                    "user" => Role::User,
                                    "assistant" => Role::Assistant,
                                    "system" => Role::System,
                                    _ => Role::User,
                                };
                                Message::text(role, &msg.content)
                            }).collect()
                        }
                        Err(e) => {
                            error!("Failed to load chat history: {}", e);
                            vec![]
                        }
                    }
                } else {
                    vec![]
                }
            };

            // 确保当前用户消息在历史中
            let content_clone_for_check = content.clone();
            let should_add = history.is_empty() || {
                history.last().map(|m| {
                    m.content.iter()
                        .filter_map(|block| {
                            if let MessageBlock::Text { text } = block {
                                Some(text.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<String>()
                }) != Some(content_clone_for_check.clone())
            };
            if should_add {
                history.push(Message::text(Role::User, content.clone()));
            }

            let model_name = {
                // 对于内置 provider，直接使用内置配置
                let config = if provider_id == crate::llm::BUILTIN_ONET_CLI_ID {
                    ProviderConfig::builtin_onet_cli()
                } else {
                    let storage_manager_for_model = storage_manager.clone();
                    let repo = match storage_manager_for_model.get::<ProviderRepository>() {
                        Some(r) => r,
                        None => {
                            if let Some(entity) = this.upgrade() {
                                let _ = cx.update(|cx| {
                                    entity.update(cx, |this, cx| {
                                        if let Some(msg) = this.messages.iter_mut().find(|m| m.id == assistant_msg_id) {
                                            msg.is_streaming = false;
                                            msg.content = "ProviderRepository not found".to_string();
                                        }
                                        this.is_loading = false;
                                        cx.notify();
                                    });
                                });
                            }
                            return;
                        }
                    };
                    match repo.get(provider_id) {
                        Ok(Some(c)) => c,
                        Ok(None) => {
                            if let Some(entity) = this.upgrade() {
                                let _ = cx.update(|cx| {
                                    entity.update(cx, |this, cx| {
                                        if let Some(msg) = this.messages.iter_mut().find(|m| m.id == assistant_msg_id) {
                                            msg.is_streaming = false;
                                            msg.content = format!("Provider not found: {}", provider_id);
                                        }
                                        this.is_loading = false;
                                        cx.notify();
                                    });
                                });
                            }
                            return;
                        }
                        Err(e) => {
                            if let Some(entity) = this.upgrade() {
                                let _ = cx.update(|cx| {
                                    entity.update(cx, |this, cx| {
                                        if let Some(msg) = this.messages.iter_mut().find(|m| m.id == assistant_msg_id) {
                                            msg.is_streaming = false;
                                            msg.content = format!("Failed to get provider: {}", e);
                                        }
                                        this.is_loading = false;
                                        cx.notify();
                                    });
                                });
                            }
                            return;
                        }
                    }
                };
                selected_model
                    .clone()
                    .unwrap_or_else(|| config.model.clone())
            };

            let request = ChatRequest {
                model: model_name.clone(),
                messages: history.iter().rev().take(history_count).rev().cloned().collect(),
                max_tokens: Some(max_tokens as u32),
                temperature: Some(temperature),
                stream: Some(true),
                ..Default::default()
            };

            // 打印请求信息用于调试
            debug!("Request details:");
            debug!("  Model: {}", model_name);
            debug!("  Max tokens: 2000");
            debug!("  Temperature: 0.7");
            debug!("  Messages count: {}", history.len());
            for (i, msg) in history.iter().enumerate() {
                let content_preview = msg.content.iter()
                    .filter_map(|block| {
                        if let MessageBlock::Text { text } = block {
                            Some(text.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<String>();
                let preview = if content_preview.chars().count() > 100 {
                    format!("{}...", content_preview.chars().take(100).collect::<String>())
                } else {
                    content_preview
                };
                debug!("  Message #{}: {:?} - {:?}", i + 1, msg.role, preview);
            }

            // 开始流式聊天
            info!("Starting chat stream for provider_id: {}", provider_id);
            let storage_manager_for_stream = storage_manager.clone();
            let stream_result = Tokio::spawn(cx, async move {
                // 对于内置 provider，直接使用内置配置
                let config = if provider_id == crate::llm::BUILTIN_ONET_CLI_ID {
                    ProviderConfig::builtin_onet_cli()
                } else {
                    let repo = storage_manager_for_stream.get::<ProviderRepository>()
                        .ok_or_else(|| anyhow::anyhow!("ProviderRepository not found"))?;
                    repo.get(provider_id)?
                        .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_id))?
                };
                let provider = global_provider_state.manager().get_provider(&config).await?;
                provider.chat_stream(&request).await
            }).await;

            debug!("Waiting for stream initialization...");
            let mut stream = match stream_result {
                Ok(task) => match task {
                    Ok(s) => {
                        info!("Stream initialized successfully");
                        s
                    },
                    Err(e) => {
                        let error_msg = format!("Failed to start chat: {}", e);
                        error!("Stream error: {}", error_msg);
                        if let Some(entity) = this.upgrade() {
                            cx.update(|cx| {
                                entity.update(cx, |this, cx| {
                                    if let Some(msg) = this.messages.iter_mut().find(|m| m.id == assistant_msg_id) {
                                        msg.is_streaming = false;
                                        msg.content = error_msg;
                                    }
                                    this.is_loading = false;
                                    cx.notify();
                                })
                            })
                        }
                        return;
                    }
                }
                Err(e) => {
                    error!("Tokio spawn error: {:?}", e);
                    return;
                }
            };

            // 处理流式响应
            info!("Starting to process stream responses...");
            let mut full_content = String::new();
            let mut chunk_count = 0;
            while let Some(result) = stream.next().await {
                // 检查是否已取消
                if cancel_token.is_cancelled() {
                    info!("Stream cancelled by user");
                    break;
                }

                chunk_count += 1;
                match result {
                    Ok(response) => {
                        debug!("Chunk #{}: response.choices.len() = {}", chunk_count, response.choices.len());
                        for (i, choice) in response.choices.iter().enumerate() {
                            debug!("  Choice #{}: finish_reason = {:?}", i, choice.finish_reason);
                        }

                        if let Some(content) = response.get_content() {
                            let content_preview: String = if content.chars().count() > 50 {
                                content.chars().take(50).collect()
                            } else {
                                content.to_string()
                            };
                            debug!("Chunk #{}: received {} chars, content: {:?}",
                                chunk_count, content.len(), content_preview);
                            full_content.push_str(&content);
                            debug!("Total content length now: {}", full_content.len());

                            if let Some(entity) = this.upgrade() {
                                let content_clone = full_content.clone();
                                let msg_id = assistant_msg_id.clone();
                                debug!("Attempting to update UI...");
                                cx.update(|cx| {
                                    entity.update(cx, |this, cx| {
                                        if let Some(msg) = this.messages.iter_mut().find(|m| m.id == msg_id) {
                                            msg.content = content_clone;
                                            debug!("Message content updated in UI");
                                        } else {
                                            warn!("Message with id {} not found!", msg_id);
                                        }
                                        this.auto_scroll_to_bottom();
                                        cx.notify();
                                    })
                                });
                            } else {
                                warn!("Entity dropped during streaming, stopping");
                                return;
                            }
                        } else {
                            debug!("Chunk #{}: no content in response", chunk_count);
                        }

                        // 检查是否完成：finish_reason 存在且不是 "null" 字符串
                        let is_done = response.choices.iter().any(|c| {
                            if let Some(reason) = &c.finish_reason {
                                reason != "null"
                            } else {
                                false
                            }
                        });
                        if is_done {
                            info!("Stream finished (finish_reason detected)");
                            break;
                        }
                    }
                    Err(err) => {
                        error!("Stream error occurred: {}", err);
                        if let Some(entity) = this.upgrade() {
                            let error_msg = format!("Stream error: {}", err);
                            let msg_id = assistant_msg_id.clone();
                            cx.update(|cx| {
                                entity.update(cx, |this, cx| {
                                    if let Some(msg) = this.messages.iter_mut().find(|m| m.id == msg_id) {
                                        msg.is_streaming = false;
                                        msg.content = error_msg;
                                    }
                                    this.is_loading = false;
                                    this.auto_scroll_to_bottom();
                                    cx.notify();
                                })
                            });
                        }
                        return;
                    }
                }
            }

            info!("Stream loop ended. Total chunks: {}, final content length: {}",
                chunk_count, full_content.len());

            // 流结束，保存助手消息
            debug!("Finalizing message...");
            if let Some(entity) = this.upgrade() {
                let final_content = full_content.clone();
                let msg_id = assistant_msg_id.clone();
                let storage_manager_final = storage_manager.clone();
                cx.update(|cx| {
                    entity.update(cx, |this, cx| {
                        if let Some(msg) = this.messages.iter_mut().find(|m| m.id == msg_id) {
                            msg.is_streaming = false;
                            msg.content = final_content.clone();
                            debug!("Message marked as not streaming, final length: {}", final_content.len());
                        }

                        // 保存助手消息到数据库
                        let final_content_inner = final_content.clone();
                        cx.spawn(async move |_this, _cx: &mut AsyncApp| {
                            debug!("Saving assistant message to database...");
                            let message_repo = storage_manager_final.get::<MessageRepository>();
                            if let Some (repo) = message_repo {
                                let mut assistant_message = ChatMessage::new(session_db_id, "assistant".to_string(), final_content_inner);
                                let result = repo.insert(&mut assistant_message);
                                match result {
                                    Ok(id) => {
                                        info!("Assistant message saved successfully, id={}",id);
                                    }
                                    Err(e) => {
                                        error!("Failed to schedule assistant message save: {}", e);
                                    }
                                }
                            }
                        }).detach();

                        this.is_loading = false;
                        this.cancel_token = None;
                        this.auto_scroll_to_bottom();
                        cx.notify();
                        debug!("Finalization complete");
                    })
                });
            } else {
                warn!("Entity dropped before finalization");
            }
        }).detach();
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
                    .child("AI 助手")
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
                            }))
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
                                    .small()
                            )
                            .when_some(session_list, |popover, list| {
                                popover.child(
                                    List::new(&list)
                                        .w(px(280.0))
                                        .max_h(px(350.0))
                                        .border_1()
                                        .border_color(border)
                                        .rounded(cx.theme().radius)
                                )
                            })
                    )
                    .child(
                        Button::new("close-panel")
                            .icon(IconName::Close)
                            .ghost()
                            .small()
                            .on_click(cx.listener(|_this, _event, _window, cx| {
                                cx.emit(AiChatPanelEvent::Close);
                            }))
                    )
            )
    }

    fn render_messages(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("chat-messages-list")
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .p_4()
            .pb_8()
            .child(
                v_flex()
                    .w_full()
                    .gap_4()
                    .children(
                        self.messages
                            .iter()
                            .map(|msg| self.render_message(msg, window, cx)),
                    ),
            )
    }

    fn auto_scroll_to_bottom(&self) {
        if self.auto_scroll_enabled {
            self.scroll_handle.scroll_to_bottom();
        }
    }

    fn render_message(&self, msg: &ChatMessageUI, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        match msg.role {
            ChatRole::User => {
                div()
                    .w_full()
                    .px_3()
                    .py_2()
                    .bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
                    .rounded_lg()
                    .child(TextView::markdown(SharedString::from(format!("user-msg-{}", msg.id)), msg.content.clone()))
                    .into_any_element()
            }
            ChatRole::Assistant => {
                match &msg.variant {
                    MessageVariant::Status { title, is_done } => {
                        self.render_status_message(msg.id.clone(), title, *is_done, msg.is_expanded, cx)
                    }
                    MessageVariant::Text => {
                        self.render_assistant_message(msg, window, cx)
                    }
                    MessageVariant::SqlResult => {
                        // SqlResult 在通用面板中不支持，返回占位符
                        div()
                            .w_full()
                            .py_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("SQL 结果")
                            )
                            .into_any_element()
                    }
                }
            }
            ChatRole::System => {
                h_flex()
                    .w_full()
                    .justify_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(msg.content.clone())
                    )
                    .into_any_element()
            }
        }
    }

    fn render_status_message(&self, id: String, title: &str, is_done: bool, _is_expanded: bool, cx: &mut Context<Self>) -> AnyElement {
        let icon = if is_done { IconName::Check } else { IconName::Loader };

        div()
            .id(SharedString::from(id))
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .py_1()
            .child(
                Icon::new(icon)
                    .with_size(Size::Small)
                    .text_color(if is_done { cx.theme().success } else { cx.theme().muted_foreground })
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(title.to_string())
            )
            .into_any_element()
    }

    fn render_assistant_message(&self, msg: &ChatMessageUI, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if msg.is_streaming && msg.content.is_empty() {
            return div()
                .w_full()
                .py_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Thinking...")
                )
                .into_any_element();
        }

        let view_id = SharedString::from(format!("ai-msg-{}", msg.id));
        let registry = self.code_block_actions.clone();

        div()
            .w_full()
            .child(
                TextView::markdown(view_id, msg.content.clone())
                    .code_block_actions({
                        move |code_block, _window, _cx| {
                            let code = code_block.code();
                            let lang = code_block.lang();
                            let lang_str = lang.as_ref().map(|s| s.as_ref());
                            let matched_actions = registry.get_actions_for_lang(lang_str);

                            let mut row = h_flex()
                                .gap_1()
                                .child(Clipboard::new("copy").value(code.clone()));

                            for (idx, action) in matched_actions.iter().enumerate() {
                                let btn_id = SharedString::from(format!("{}-{}", action.id, idx));
                                let callback = action.callback.clone();
                                let icon = action.icon.clone();
                                let label = action.label.clone();
                                let code = code.to_string();
                                let lang = lang.as_ref().map(|s| s.to_string());
                                let mut btn = Button::new(btn_id)
                                    .icon(icon)
                                    .ghost()
                                    .xsmall()
                                    .on_click({
                                        let code = code.clone();
                                        let lang = lang.clone();
                                        move |_, window, cx| {
                                            callback(code.clone(), lang.clone(), window, cx);
                                        }
                                    });

                                if let Some(lbl) = label {
                                    btn = btn.label(lbl);
                                }

                                row = row.child(btn);
                            }

                            row
                        }
                    })
                    .p_3()
                    .selectable(true)
            )
            .into_any_element()
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
                                    .child(
                                        self.provider_select_state.render_provider_select(),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        self.provider_select_state.render_model_select(),
                                    ),
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
                                    .content(move |_state, _window, _cx| {
                                        settings_panel.clone()
                                    })
                            }),
                    )
                    .child(
                        if self.can_cancel() {
                            // 加载中显示终止按钮
                            Button::new("cancel")
                                .with_size(Size::Small)
                                .danger()
                                .icon(IconName::CircleX)
                                .label("终止")
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.cancel_current_operation(cx);
                                }))
                        } else {
                            // 正常状态显示发送按钮
                            Button::new("send")
                                .with_size(Size::Small)
                                .primary()
                                .icon(IconName::ArrowRight)
                                .label("发送")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.submit(window, cx);
                                }))
                        },
                    ),
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

    fn on_session_edit(&mut self, session_id: i64, name: String, window: &mut Window, cx: &mut Context<Self>) {
        self.start_rename_session(session_id, name, window, cx);
    }

    fn on_session_delete(&mut self, session_id: i64, cx: &mut Context<Self>) {
        self.delete_session(session_id, cx);
    }

    fn is_current_session(&self, session_id: i64) -> bool {
        self.session_id == Some(session_id)
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
        let bg_color = self.background(cx);
        let fg_color = self.foreground(cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .bg(bg_color)
                    .text_color(fg_color)
                    .child(self.render_header(window, cx))
                    .child(self.render_messages(window, cx))
                    .child(self.render_input(window, cx))
            )
    }
}
