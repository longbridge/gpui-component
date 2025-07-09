mod action;
mod view;
use crate::app::AppExt;
use crate::app::FoEvent;
use crate::backoffice::cross_runtime::StreamMessage;
use crate::backoffice::llm::types::ToolCall;
use crate::backoffice::llm::types::{ChatMessage, MessageRole};
use crate::config::todo_item::*;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{input::InputState, scroll::ScrollbarState, *};
use std::time::Duration;

// 从 rmcp 导入 MCP 类型
use rmcp::model::Tool as McpTool;

actions!(todo_thread, [Tab, TabPrev, CloseWindow, SendMessage]);

const CONTEXT: &str = "TodoThread";

impl MessageRole {
    fn display_name(&self) -> &'static str {
        match self {
            MessageRole::User => "你",
            MessageRole::Assistant => "AI助手",
            MessageRole::System => "系统",
        }
    }

    fn color(&self) -> gpui::Rgba {
        match self {
            MessageRole::User => gpui::rgb(0x3B82F6),
            MessageRole::Assistant => gpui::rgb(0x10B981),
            MessageRole::System => gpui::rgb(0x6B7280),
        }
    }
}

pub struct TodoThreadChat {
    focus_handle: FocusHandle,
    // 聊天功能
    chat_messages: Vec<ChatMessage>,
    chat_input: Entity<InputState>,
    is_loading: bool,
    scroll_handle: ScrollHandle,
    scroll_size: gpui::Size<Pixels>,
    scroll_state: ScrollbarState,

    // 手风琴展开状态
    expanded_providers: Vec<usize>,
    expanded_tool_providers: Vec<usize>,

    // 新增：缓存从 McpRegistry 获取的工具数据
    cached_server_tools: std::collections::HashMap<String, Vec<McpTool>>,

    _subscriptions: Vec<Subscription>,
    todoitem: Todo,
}

const WIDTH: Pixels = px(700.0);
const HEIGHT: Pixels = px(650.0);
const SIZE: gpui::Size<Pixels> = size(WIDTH, HEIGHT);

const PLACEHOLDER: &'static str = if cfg!(target_os = "macos") {
    "输入消息与AI助手对话...，按Cmd+Enter发送，按ESC清除输入框"
} else {
    "输入消息与AI助手对话...，按Ctrl+Enter发送，按ESC清除输入框"
};

impl TodoThreadChat {
    pub fn open(todo: Todo, cx: &mut App) -> WindowHandle<Root> {
        cx.activate(true);
        let window_bounds = Bounds::centered(None, SIZE, cx);
        let options = WindowOptions {
            app_id: Some("x-todo-app".to_string()),
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: Some(SIZE),
            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };

        cx.create_normal_window(
            format!("xTo-Do {}", todo.title),
            options,
            move |window, cx| cx.new(|cx| Self::new(todo, window, cx)),
        )
    }

    fn new(todoitem: Todo, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let todo_id = todoitem.id.clone();
        window.on_window_should_close(cx, move |_window, app| {
            app.dispatch_event(FoEvent::TodoChatWindowClosed(todo_id.clone()));
            true
        });
        // 聊天输入框 - 多行支持
        let chat_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(PLACEHOLDER)
                .clean_on_escape()
                .multi_line()
                .auto_grow(1, 6)
        });

        let _subscriptions = vec![cx.subscribe_in(&chat_input, window, Self::on_chat_input_event)];
        let chat_messages = vec![
        // 1. 系统消息 - 任务描述
        ChatMessage::system_text_with_source(
            format!("任务: {}\n描述: {}", todoitem.title, todoitem.description),
            "task_system"
        ).with_metadata("task_id", todoitem.id.clone()),
        
        // 2. 系统提示消息
        ChatMessage::system_text_with_source(
            "你是一个专业的任务管理助手。请帮助用户高效地完成任务，提供实用的建议和解决方案。你可以使用可用的工具来协助完成任务。",
            "ai_assistant"
        ),
        
        // 3. 欢迎消息（助手）
        ChatMessage::assistant_text_with_source(
            format!("👋 您好！我是您的AI助手，很高兴为您服务！\n\n我看到您当前的任务是：**{}**\n\n我可以帮助您：\n• 分析任务需求\n• 提供解决方案\n• 协助完成具体步骤\n• 使用各种工具来支持您的工作\n\n请告诉我您需要什么帮助？", todoitem.title),
            "ai_assistant"
        ).with_model("gpt-4", "GPT-4")
         .with_metadata("message_type", "welcome"),
        
        // 4. 示例用户消息
        ChatMessage::user_text_with_source(
            "能帮我分析一下这个任务吗？",
            "user_demo"
        ).with_metadata("session_id", "demo_session")
         .with_timestamp(chrono::Utc::now() - chrono::Duration::minutes(5)),
        
        // 5. 助手回复（带工具使用说明）
        ChatMessage::assistant_text_with_source(
            format!("当然可以！让我来分析您的任务：**{}**\n\n基于任务描述，我建议以下步骤：\n\n1. **任务分解**：将复杂任务分解为可管理的小步骤\n2. **资源评估**：确定需要的工具和资源\n3. **时间规划**：制定合理的时间表\n4. **执行监控**：跟踪进度并及时调整\n\n我可以使用以下工具来协助您：\n• 📝 文档处理工具\n• 🔍 信息搜索工具\n• 📊 数据分析工具\n• 📅 时间管理工具\n\n您希望我从哪个方面开始帮助您？", todoitem.title),
            "ai_assistant"
        ).with_model("gpt-4", "GPT-4")
         .with_metadata("has_tool_suggestions", "true")
         .with_timestamp(chrono::Utc::now() - chrono::Duration::minutes(3)),
        
        // 6. 用户询问工具使用
        ChatMessage::user_text_with_source(
            "你能使用搜索工具帮我查找相关资料吗？",
            "user_demo"
        ).with_metadata("tool_request", "search")
         .with_timestamp(chrono::Utc::now() - chrono::Duration::minutes(2)),
        
        // 7. 助手确认工具调用
        ChatMessage::assistant_text_with_source(
            "好的，我将使用搜索工具为您查找相关资料。让我搜索一下...",
            "ai_assistant"
        ).with_model("gpt-4", "GPT-4")
         .with_metadata("tool_preparation", "search")
         .with_timestamp(chrono::Utc::now() - chrono::Duration::minutes(1)),
        
        // 8. 工具调用消息（示例）
        ChatMessage::tool_call_with_source(
            ToolCall {
                name: "web_search".to_string(),
                args: format!(r#"{{"query": "{} 最佳实践", "max_results": 5}}"#, todoitem.title),
            },
            "search_tool"
        ).with_metadata("tool_type", "search")
         .with_timestamp(chrono::Utc::now() - chrono::Duration::seconds(30)),
        
        // 9. 工具返回结果（模拟）
        ChatMessage::assistant_text_with_source(
            "🔍 **搜索结果**：\n\n基于搜索到的资料，我为您整理了以下要点：\n\n• **关键策略**：采用敏捷方法，分步骤执行\n• **常见陷阱**：避免一次性承担过多任务\n• **成功经验**：定期回顾和调整计划\n• **推荐工具**：使用项目管理软件跟踪进度\n\n根据这些信息，我建议您从制定详细的行动计划开始。需要我帮您创建一个具体的执行计划吗？",
            "ai_assistant"
        ).with_model("gpt-4", "GPT-4")
         .with_metadata("tool_result", "search")
         .with_metadata("has_suggestions", "true")
         .with_timestamp(chrono::Utc::now() - chrono::Duration::seconds(10)),
        
        // 10. 当前状态消息
        ChatMessage::system_text_with_source(
            "💡 提示：您可以继续与AI助手对话，获取更多帮助和建议。",
            "system_tip"
        ).with_metadata("tip_type", "interaction_guide"),
    ];


        let instance = Self {
            focus_handle: cx.focus_handle(),
            chat_messages,
            chat_input,
            is_loading: false,
            scroll_handle: ScrollHandle::new(),
            expanded_providers: Vec::new(),
            expanded_tool_providers: Vec::new(),
            cached_server_tools: std::collections::HashMap::new(), // 新增初始化
            _subscriptions,
            scroll_state: ScrollbarState::default(),
            scroll_size: gpui::Size::default(),
            todoitem,
        };
        instance.start_external_message_handler(cx);
        instance.scroll_handle.scroll_to_bottom();
        instance
    }

    /// 启动外部消息处理器
    fn start_external_message_handler(&self, cx: &mut Context<Self>) {
        let todo_id = self.todoitem.id.clone();

        cx.spawn(async move |this, app: &mut AsyncApp| {
            Self::handle_external_messages(this, app, todo_id).await;
        })
        .detach();
    }

    /// 处理外部消息的异步任务
    async fn handle_external_messages(this: WeakEntity<Self>, app: &mut AsyncApp, todo_id: String) {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // 订阅外部消息
        let _sub = app.subscribe_event(move |StreamMessage { source, message }: &StreamMessage| {
            if &todo_id == source {
                tracing::trace!("接收到消息: {} {:?}", source, message);
                tx.try_send(message.clone()).unwrap_or_else(|e| {
                    tracing::error!("Failed to send message to channel: {}", e);
                });
            }
        });

        // 消息处理循环
        'message_loop: loop {
            Timer::after(Duration::from_millis(50)).await;

            let mut buffer = String::new();
            let mut message_count = 0;

            // 批量收集消息
            loop {
                match rx.try_recv() {
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        break;
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        tracing::info!("外部消息通道已断开连接");
                        break 'message_loop;
                    }
                    Ok(msg) => {
                        if msg.is_text_only() {
                            buffer.push_str(&msg.get_text());
                            message_count += 1;
                        }
                    }
                }
            }

            // 如果没有新消息，继续等待
            if buffer.is_empty() {
                continue;
            }

            // 更新UI
            let update_result = this.update(app, |this, cx| {
                Self::process_received_message(this, buffer, cx);
            });

            if update_result.is_err() {
                tracing::warn!("更新UI失败，可能组件已销毁");
                break 'message_loop;
            }

            tracing::trace!("处理了 {} 条消息", message_count);
        }

        tracing::info!("外部消息处理器已停止");
    }

    /// 处理接收到的消息
    fn process_received_message(&mut self, buffer: String, cx: &mut Context<Self>) {
        if let Some(last_message) = self.chat_messages.last_mut() {
            last_message.add_text_chunk(&buffer);
        } else {
            // 如果没有消息，创建一个新的助手消息
            let new_message =
                ChatMessage::assistant_text_with_source(buffer, self.todoitem.id.clone());
            self.chat_messages.push(new_message);
        }

        self.is_loading = false;
        self.scroll_handle.scroll_to_bottom();
        cx.notify();
    }
}
