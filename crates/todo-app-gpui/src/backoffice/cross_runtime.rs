use crate::backoffice::agentic::llm::{
    LlmChatRequest, LlmChatResult, LlmChatWithToolsRequest, LlmRegistry,
};
use crate::backoffice::mcp::server::McpServerInstance;
use crate::backoffice::mcp::{
    GetAllInstances, GetServerInstance, McpCallToolRequest, McpCallToolResult, McpRegistry,
};
use crate::config::todo_item::SelectedTool;
use crate::ui::views::todo_thread::ChatMessage;
use actix::Arbiter;
use tokio::sync::{mpsc, oneshot};

/// 跨运行时通信桥接器
///
/// 这个结构体是连接 GPUI 界面线程和 Actix Actor 系统之间的关键桥梁。
/// 主要处理需要与 Actor 系统交互的操作，如 MCP 工具调用和 LLM 聊天。
///
/// 对于配置文件读取操作（如获取提供商列表、模型列表），GUI 直接通过
/// LlmProviderManager 读取，无需跨运行时通信。
pub struct CrossRuntimeBridge {
    dispatcher: mpsc::UnboundedSender<Box<dyn MessageHandler + Send>>,
}

impl CrossRuntimeBridge {
    pub fn init_runtime() {
        CROSS_RUNTIME_BRIDGE.get_or_init(|| CrossRuntimeBridge::new());
    }

    /// 获取全局桥接器实例
    pub fn global() -> &'static Self {
        CROSS_RUNTIME_BRIDGE.get_or_init(|| CrossRuntimeBridge::new())
    }
}
/// 消息处理器特征
trait MessageHandler {
    fn handle(self: Box<Self>);
}

// ===== MCP 相关处理器（保持原有实现）=====

/// 获取服务器实例消息处理器
struct GetInstanceHandler {
    server_id: String,
    response: oneshot::Sender<Option<McpServerInstance>>,
}

impl MessageHandler for GetInstanceHandler {
    fn handle(self: Box<Self>) {
        Arbiter::new().spawn(async move {
            let registry = McpRegistry::global();
            let result = registry
                .send(GetServerInstance {
                    server_id: self.server_id,
                })
                .await;

            let instance = match result {
                Ok(instance) => instance,
                Err(e) => {
                    eprintln!("Failed to get server instance: {}", e);
                    None
                }
            };

            let _ = self.response.send(instance);
        });
    }
}

/// 工具调用消息处理器
struct CallToolHandler {
    server_id: String,
    tool_name: String,
    arguments: String,
    response: oneshot::Sender<Result<McpCallToolResult, String>>,
}

impl MessageHandler for CallToolHandler {
    fn handle(self: Box<Self>) {
        Arbiter::new().spawn(async move {
            let registry = McpRegistry::global();
            let result = registry
                .send(McpCallToolRequest {
                    id: self.server_id,
                    name: self.tool_name,
                    arguments: self.arguments,
                })
                .await;

            let final_result = match result {
                Ok(tool_result) => Ok(tool_result),
                Err(e) => Err(e.to_string()),
            };

            let _ = self.response.send(final_result);
        });
    }
}

/// 获取所有实例消息处理器
struct GetAllInstancesHandler {
    response: oneshot::Sender<Vec<McpServerInstance>>,
}

impl MessageHandler for GetAllInstancesHandler {
    fn handle(self: Box<Self>) {
        Arbiter::new().spawn(async move {
            let registry = McpRegistry::global();
            let result = registry.send(GetAllInstances).await;
            let instances = result.unwrap_or_default();
            let _ = self.response.send(instances);
        });
    }
}
/// LLM 聊天消息处理器
struct LlmChatHandler {
    provider_id: String,
    model_id: String,
    source: String,
    prompt: String,
    chat_history: Vec<ChatMessage>,
    response: oneshot::Sender<Result<LlmChatResult, String>>,
}

impl MessageHandler for LlmChatHandler {
    fn handle(self: Box<Self>) {
        Arbiter::new().spawn(async move {
            let registry = LlmRegistry::global();
            let result = registry
                .send(LlmChatRequest {
                    provider_id: self.provider_id,
                    model_id: self.model_id,
                    source: self.source,
                    prompt: self.prompt,
                    chat_history: self.chat_history,
                })
                .await;

            let final_result = match result {
                Ok(chat_result) => Ok(chat_result),
                Err(e) => Err(e.to_string()),
            };

            let _ = self.response.send(final_result);
        });
    }
}

/// LLM 带工具聊天消息处理器
struct LlmChatWithToolsHandler {
    provider_id: String,
    model_id: String,
    source: String,
    prompt: String,
    tools: Vec<SelectedTool>,
    chat_history: Vec<ChatMessage>,
    response: oneshot::Sender<Result<LlmChatResult, String>>,
}

impl MessageHandler for LlmChatWithToolsHandler {
    fn handle(self: Box<Self>) {
        Arbiter::new().spawn(async move {
            let registry = LlmRegistry::global();
            let result = registry
                .send(LlmChatWithToolsRequest {
                    provider_id: self.provider_id,
                    model_id: self.model_id,
                    source: self.source,
                    prompt: self.prompt,
                    tools: self.tools,
                    chat_history: self.chat_history,
                })
                .await;

            let final_result = match result {
                Ok(chat_result) => Ok(chat_result),
                Err(e) => Err(e.to_string()),
            };

            let _ = self.response.send(final_result);
        });
    }
}

impl CrossRuntimeBridge {
    /// 创建新的桥接器实例
    pub fn new() -> Self {
        let (dispatcher, mut receiver) =
            mpsc::unbounded_channel::<Box<dyn MessageHandler + Send>>();

        // 启动消息调度器
        Arbiter::new().spawn(async move {
            while let Some(handler) = receiver.recv().await {
                handler.handle();
            }
        });

        Self { dispatcher }
    }

    /// 异步获取指定服务器实例
    pub async fn get_instance<S: ToString>(&self, server_id: S) -> Option<McpServerInstance> {
        let (response_tx, response_rx) = oneshot::channel();

        let handler = Box::new(GetInstanceHandler {
            server_id: server_id.to_string(),
            response: response_tx,
        });

        if self.dispatcher.send(handler).is_err() {
            return None;
        }

        response_rx.await.unwrap_or(None)
    }

    /// 异步调用 MCP 服务器工具
    pub async fn call_tool(
        &self,
        server_id: String,
        tool_name: String,
        arguments: String,
    ) -> Result<McpCallToolResult, String> {
        let (response_tx, response_rx) = oneshot::channel();

        let handler = Box::new(CallToolHandler {
            server_id,
            tool_name,
            arguments,
            response: response_tx,
        });

        if self.dispatcher.send(handler).is_err() {
            return Err("Failed to send message to dispatcher".to_string());
        }

        response_rx
            .await
            .map_err(|_| "Failed to receive response".to_string())?
    }

    /// 异步获取所有服务器实例
    pub async fn get_all_instances(&self) -> Vec<McpServerInstance> {
        let (response_tx, response_rx) = oneshot::channel();

        let handler = Box::new(GetAllInstancesHandler {
            response: response_tx,
        });

        if self.dispatcher.send(handler).is_err() {
            return Vec::new();
        }

        response_rx.await.unwrap_or_default()
    }

    // ===== LLM 方法（仅保留聊天相关）=====

    /// 异步 LLM 聊天
    pub async fn llm_chat(
        &self,
        provider_id: String,
        model_id: String,
        source: String,
        prompt: String,
        chat_history: Vec<ChatMessage>, //TODO:放到Agent里维护，下游不需要维护
    ) -> Result<LlmChatResult, String> {
        let (response_tx, response_rx) = oneshot::channel();

        let handler = Box::new(LlmChatHandler {
            provider_id,
            model_id,
            source,
            prompt,
            chat_history,
            response: response_tx,
        });

        if self.dispatcher.send(handler).is_err() {
            return Err("Failed to send message to dispatcher".to_string());
        }

        response_rx
            .await
            .map_err(|_| "Failed to receive response".to_string())?
    }

    /// 异步 LLM 带工具聊天
    pub async fn llm_chat_with_tools(
        &self,
        provider_id: String,
        model_id: String,
        source: String,
        prompt: String,
        tools: Vec<SelectedTool>,
        chat_history: Vec<ChatMessage>,
    ) -> Result<LlmChatResult, String> {
        let (response_tx, response_rx) = oneshot::channel();

        let handler = Box::new(LlmChatWithToolsHandler {
            provider_id,
            model_id,
            source,
            prompt,
            tools,
            chat_history,
            response: response_tx,
        });

        if self.dispatcher.send(handler).is_err() {
            return Err("Failed to send message to dispatcher".to_string());
        }

        response_rx
            .await
            .map_err(|_| "Failed to receive response".to_string())?
    }
}

static CROSS_RUNTIME_BRIDGE: std::sync::OnceLock<CrossRuntimeBridge> = std::sync::OnceLock::new();
