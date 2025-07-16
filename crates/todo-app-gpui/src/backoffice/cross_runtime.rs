use crate::backoffice::llm::{types::*, LlmChatRequest, LlmRegistry};
use crate::backoffice::mcp::server::McpServerSnapshot;
use crate::backoffice::mcp::{
    GetAllSnapshots, GetServerSnapshot, McpCallToolRequest, McpCallToolResult, McpRegistry,
};
use crate::xbus::{self, Subscription};
use actix::Arbiter;
use futures::StreamExt;
use std::any::{Any, TypeId};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
/// 跨运行时通信桥接器
///
/// ## 核心职责
///
/// 这个结构体是连接 GPUI 界面线程和 Actix Actor 系统之间的关键桥梁。
/// 解决了两个不同运行时环境之间的异步通信问题：
///
/// - **GPUI 主线程**: 负责界面渲染和用户交互
/// - **Actix Actor 系统**: 负责后台业务逻辑处理
///
/// ## 设计原理
///
/// ```text
/// GPUI Thread                Bridge                 Actix Runtime
/// ┌─────────────┐           ┌─────────────┐        ┌──────────────┐
/// │ UI 事件     │ ──请求──→ │ 消息队列    │ ──→    │ Registry     │
/// │ 用户交互    │           │ 调度器      │        │ Actor        │
/// │ 状态更新    │ ←─响应──  │ 响应通道    │ ←──    │ 业务逻辑     │
/// └─────────────┘           └─────────────┘        └──────────────┘
/// ```
///
/// ## 功能范围
///
/// **包含的操作（需要跨运行时）:**
/// - MCP 工具调用和服务器管理
/// - LLM 聊天和推理请求
/// - 动态状态查询和更新
///
/// **不包含的操作（无需跨运行时）:**
/// - 配置文件读取（GUI 直接通过 `LlmProviderManager` 读取）
/// - 静态资源访问
/// - 本地状态管理
///
/// ## 性能特性
///
/// - **非阻塞**: 所有操作都是异步的，不会阻塞 GPUI 渲染
/// - **并发安全**: 支持多个请求同时处理
/// - **错误隔离**: 单个请求失败不会影响其他请求
/// - **资源高效**: 使用消息传递避免数据拷贝
pub struct CrossRuntimeBridge {
    /// 消息调度器
    ///
    /// **职责**: 接收来自 GPUI 的各种类型消息并快速分发
    ///
    /// **实现原理**:
    /// - 使用无界通道避免 GPUI 线程阻塞
    /// - 每个消息封装为 `Box<dyn MessageHandler>`，实现类型擦除
    /// - 调度器在独立的 Arbiter 中运行，确保高可用性
    ///
    /// **设计权衡**:
    /// - 选择无界通道：GPUI 消息频率较低，避免背压问题
    /// - 动态分发：支持不同类型消息的统一处理
    /// - 生命周期管理：调度器与桥接器实例绑定
    dispatcher: mpsc::UnboundedSender<Box<dyn MessageHandler + Send>>,
}

impl CrossRuntimeBridge {
    /// 获取全局桥接器实例
    ///
    /// **设计模式**: 单例模式，确保全局唯一的通信桥梁
    /// **线程安全**: `OnceLock` 提供线程安全的懒加载
    /// **生命周期**: 与应用程序生命周期一致
    pub fn global() -> &'static Self {
        CROSS_RUNTIME_BRIDGE.get_or_init(|| CrossRuntimeBridge::new())
    }
}

impl CrossRuntimeBridge {
    /// 创建新的桥接器实例
    ///
    /// **初始化流程**:
    /// 1. 创建无界消息通道
    /// 2. 启动消息调度器在独立 Arbiter 中
    /// 3. 返回桥接器实例供 GPUI 使用
    ///
    /// **生命周期管理**:
    /// - 调度器与 receiver 绑定，自动处理资源清理
    /// - 当所有 dispatcher 克隆都销毁时，调度器自然退出
    fn new() -> Self {
        let (dispatcher, mut receiver) =
            mpsc::unbounded_channel::<Box<dyn MessageHandler + Send>>();

        // 启动消息调度器
        //
        // **职责**: 持续接收消息并分发给对应的处理器
        // **并发模型**: 每个消息在独立的任务中处理，支持高并发
        // **错误隔离**: 单个消息处理失败不会影响调度器和其他消息
        Arbiter::new().spawn(async move {
            while let Some(handler) = receiver.recv().await {
                // 每个 handler 在独立的上下文中执行
                // 这确保了并发处理和错误隔离
                handler.handle();
            }
        });

        Self { dispatcher }
    }

    pub fn post<E: Any + 'static + Send + Sync>(&self, event: E) {
        xbus::post(event);
    }

    pub fn subscribe<E: Any + 'static + Send + Sync, F: Fn(&E) + Send + Sync + 'static>(
        &self,
        f: F,
    ) -> Subscription {
        xbus::subscribe(f)
    }

    pub fn subscribe_any<F: Fn(TypeId, &dyn Any) + Send + Sync + 'static>(
        &self,
        f: F,
    ) -> Subscription {
        xbus::subscribe_any(f)
    }
    /// 异步获取指定服务器实例
    ///
    /// **参数**: `server_id` - 支持任何可转换为字符串的类型
    /// **返回**: `Option<McpServerInstance>` - None 表示服务器不存在或获取失败
    ///
    /// **使用场景**:
    /// - UI 显示服务器详情
    /// - 验证服务器是否可用
    /// - 获取服务器提供的工具列表
    pub async fn get_server_snapshot<S: ToString>(
        &self,
        server_id: S,
    ) -> Option<McpServerSnapshot> {
        // 创建一次性响应通道
        let (response_tx, response_rx) = oneshot::channel();

        // 构造消息处理器
        let handler = Box::new(GetServerSnapshotHandler {
            server_id: server_id.to_string(),
            response: response_tx,
        });

        // 发送到调度器
        if self.dispatcher.send(handler).is_err() {
            // 调度器已关闭，返回失败
            return None;
        }

        // 等待响应，处理接收错误
        response_rx.await.unwrap_or(None)
    }

    /// 异步调用 MCP 服务器工具
    ///
    /// **参数**:
    /// - `server_id`: 目标服务器标识
    /// - `tool_name`: 工具名称
    /// - `arguments`: JSON 格式的参数字符串
    ///
    /// **返回**: `Result<McpCallToolResult, String>`
    /// - 成功时包含工具执行结果
    /// - 失败时包含错误描述
    ///
    /// **错误类型**:
    /// - 服务器不存在或未启动
    /// - 工具不存在或参数无效
    /// - 网络通信错误
    /// - 工具执行异常
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
    ///
    /// **返回**: `Vec<McpServerSnapshot>` - 当前所有活跃的服务器实例
    ///
    /// **使用场景**:
    /// - 服务器管理界面
    /// - 工具选择器
    /// - 系统状态监控
    ///
    /// **性能考虑**:
    /// - 返回的是快照数据，不保证实时性
    /// - 大量服务器时可能有性能影响，考虑分页或缓存
    pub async fn get_all_server_snapshot(&self) -> Vec<McpServerSnapshot> {
        let (response_tx, response_rx) = oneshot::channel();

        let handler = Box::new(GetAllSnapshotsHandler {
            response: response_tx,
        });

        if self.dispatcher.send(handler).is_err() {
            return Vec::new();
        }

        response_rx.await.unwrap_or_default()
    }

    /// 异步 LLM 聊天
    ///
    /// **功能**: 与指定的 LLM 模型进行对话
    ///
    /// **参数**:
    /// - `provider_id`: LLM 提供商（如 "openai", "anthropic"）
    /// - `model_id`: 具体模型（如 "gpt-4", "claude-3"）
    /// - `source`: 请求来源标识（用于日志追踪）
    /// - `prompt`: 用户输入
    /// - `chat_history`: 对话历史（TODO: 应移至 Agent 层管理）
    ///
    /// **返回**: `Result<LlmChatResult, String>`
    /// - 成功时包含 LLM 回复、token 使用量等信息
    /// - 失败时包含错误描述
    ///
    /// **注意事项**:
    /// - 聊天历史目前由调用方维护，未来计划移至 Agent 层
    /// - 不同提供商的 API 限制和特性可能不同
    pub async fn llm_chat(
        &self,
        provider_id: String,
        model_id: String,
        source: String,
        prompt: String,
        history: Vec<ChatMessage>, //TODO:放到Agent里维护，下游不需要维护
    ) -> Result<(), String> {
        let (response_tx, response_rx) = oneshot::channel();

        let handler = Box::new(LlmChatHandler {
            request: LlmChatRequest {
                provider_id,
                model_id,
                source,
                prompt,
                history, // 使用新的历史参数
            },
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

/// 消息处理器特征
///
/// **设计目的**: 实现类型擦除，允许不同类型的消息在同一个队列中处理
///
/// **核心思想**:
/// - 每种业务操作对应一个具体的 Handler 实现
/// - 通过 trait object 实现统一的消息接口
/// - `Box<Self>` 转移所有权，避免生命周期问题
trait MessageHandler {
    /// 处理消息的核心方法
    ///
    /// **执行环境**: 在独立的 Arbiter 中执行，不阻塞调度器
    /// **错误处理**: 每个 handler 负责自己的错误处理和响应
    /// **资源管理**: 通过 `Box<Self>` 自动管理内存
    fn handle(self: Box<Self>);
}

/// 获取服务器实例消息处理器
///
/// **业务功能**: 从 MCP Registry 获取指定 ID 的服务器实例
/// **异步模式**: 使用 oneshot 通道返回结果到 GPUI 线程
/// **错误处理**: 网络错误或 Actor 错误会返回 None
struct GetServerSnapshotHandler {
    /// 目标服务器的唯一标识符
    server_id: String,
    /// 响应通道，用于将结果发送回 GPUI 线程
    ///
    /// **类型**: `Option<McpServerInstance>` - None 表示服务器不存在或获取失败
    /// **生命周期**: 消息处理完成后自动释放
    response: oneshot::Sender<Option<McpServerSnapshot>>,
}

impl MessageHandler for GetServerSnapshotHandler {
    fn handle(self: Box<Self>) {
        // 在新的 Arbiter 中执行，避免阻塞消息调度器
        Arbiter::new().spawn(async move {
            // 获取全局 MCP Registry 的地址
            let registry = McpRegistry::global();

            // 发送获取实例的消息到 Registry Actor
            let result = registry
                .send(GetServerSnapshot {
                    server_id: self.server_id,
                })
                .await;

            // 处理 Actor 通信结果
            let instance = match result {
                Ok(instance) => instance,
                Err(e) => {
                    // 记录错误但不中断处理流程
                    tracing::warn!("Failed to get server instance: {}", e);
                    None
                }
            };

            // 将结果发送回 GPUI 线程，忽略发送失败（接收端可能已关闭）
            let _ = self.response.send(instance);
        });
    }
}

/// 工具调用消息处理器
///
/// **业务功能**: 调用指定 MCP 服务器的特定工具
/// **参数验证**: 在 Actor 层面进行参数验证和格式检查
/// **结果处理**: 统一包装成功和错误结果
struct CallToolHandler {
    /// 目标服务器 ID
    server_id: String,
    /// 要调用的工具名称
    tool_name: String,
    /// 工具参数（JSON 字符串格式）
    arguments: String,
    /// 响应通道
    ///
    /// **成功**: `Ok(McpCallToolResult)` - 包含工具执行结果
    /// **失败**: `Err(String)` - 包含错误描述信息
    response: oneshot::Sender<Result<McpCallToolResult, String>>,
}

impl MessageHandler for CallToolHandler {
    fn handle(self: Box<Self>) {
        Arbiter::new().spawn(async move {
            let registry = McpRegistry::global();

            // 构造工具调用请求
            let result = registry
                .send(McpCallToolRequest {
                    id: self.server_id,
                    name: self.tool_name,
                    arguments: self.arguments,
                })
                .await;

            // 统一错误处理：将 Actor 错误转换为字符串
            let final_result = match result {
                Ok(tool_result) => Ok(tool_result),
                Err(e) => Err(e.to_string()),
            };

            let _ = self.response.send(final_result);
        });
    }
}

/// 获取所有实例消息处理器
///
/// **业务功能**: 获取当前所有活跃的 MCP 服务器实例列表
/// **使用场景**: UI 展示服务器状态、工具选择器等
/// **数据一致性**: 返回的是某个时间点的快照，可能与实时状态有微小差异
struct GetAllSnapshotsHandler {
    /// 响应通道，返回实例列表
    ///
    /// **空列表**: 当没有活跃实例或获取失败时返回空 Vec
    /// **顺序**: 不保证特定的排序，由 Registry 内部实现决定
    response: oneshot::Sender<Vec<McpServerSnapshot>>,
}

impl MessageHandler for GetAllSnapshotsHandler {
    fn handle(self: Box<Self>) {
        Arbiter::new().spawn(async move {
            let registry = McpRegistry::global();
            let result = registry.send(GetAllSnapshots).await;
            // 错误时返回空列表，确保 UI 不会因为网络问题而崩溃
            let instances = result.unwrap_or_default();
            let _ = self.response.send(instances);
        });
    }
}

/// LLM 聊天消息处理器
///
/// **业务功能**: 与指定的 LLM 提供商进行对话
/// **上下文管理**: 支持传递聊天历史以保持对话连续性
/// **提供商抽象**: 通过 provider_id 和 model_id 抽象不同的 LLM 服务
struct LlmChatHandler {
    request: LlmChatRequest,
    /// 响应通道
    ///
    /// **成功**: `Ok(LlmChatResult)` - 包含 LLM 的回复和元数据
    /// **失败**: `Err(String)` - 包含错误信息（API 错误、网络错误等）
    response: oneshot::Sender<Result<(), String>>,
}

impl MessageHandler for LlmChatHandler {
    fn handle(self: Box<Self>) {
        Arbiter::new().spawn(async move {
            let source = self.request.source.clone();

            let result = tokio::time::timeout(
                Duration::from_secs(15),
                LlmRegistry::chat_stream(self.request),
            )
            .await;

            tracing::trace!("开始处理 LLM 聊天请求");
            // 错误统一转换为字符串，简化 GPUI 端的错误处理
            match result {
                Ok(Ok(mut stream)) => {
                    self.response.send(Ok(())).ok();
                    tracing::trace!("开始接收 LLM 聊天流消息");
                    loop {
                        match tokio::time::timeout(Duration::from_secs(15), stream.next()).await {
                            Ok(Some(Ok(message))) => {
                                tracing::trace!("接收到 LLM 聊天流消息: {:?}", message);
                                let stream_message = StreamMessage::stream(source.clone(), message);
                                // 这里可以将消息发送到 UI 或其他处理器
                                xbus::post(stream_message);
                            }
                            Ok(Some(Err(err))) => {
                                xbus::post(StreamMessage::stream(
                                    source.clone(),
                                    MessageContent::TextChunk(format!("模型错误: {:?}", err)),
                                ));
                            }
                            Ok(None) => {
                                // 流结束，发送完成消息
                                break;
                            }
                            Err(_) => {
                                // 超时或错误，发送错误消息
                                xbus::post(StreamMessage::stream(
                                    source.clone(),
                                    MessageContent::TextChunk("模型超时".to_string()),
                                ));
                                break;
                            }
                        }
                    }
                    tracing::trace!("LLM 聊天流消息接收完毕");
                    xbus::post(StreamMessage::done(source));
                }

                Ok(Err(err)) => {
                    self.response.send(Err(err.to_string())).ok();
                    xbus::post(StreamMessage::stream(
                        source.clone(),
                        MessageContent::TextChunk(format!("模型错误: {:?}", err)),
                    ));
                    xbus::post(StreamMessage::done(source));
                }
                Err(_) => {
                    self.response.send(Err("模型超时".to_string())).ok();
                    xbus::post(StreamMessage::stream(
                        source.clone(),
                        MessageContent::TextChunk("模型超时".to_string()),
                    ));
                    xbus::post(StreamMessage::done(source));
                }
            }
        });
    }
}

/// 全局桥接器实例
///
/// **设计模式**: 单例模式，确保应用程序中只有一个桥接器实例
/// **线程安全**: `OnceLock` 提供线程安全的懒加载初始化
/// **生命周期**: 与应用程序生命周期一致，在程序退出时自动清理
///
/// **使用方式**:
/// ```rust
/// // 初始化（通常在应用启动时）
/// CrossRuntimeBridge::init_runtime();
///
/// // 在需要时获取实例
/// let bridge = CrossRuntimeBridge::global();
/// let result = bridge.call_tool("server1", "tool1", "{}").await;
/// ```
static CROSS_RUNTIME_BRIDGE: std::sync::OnceLock<CrossRuntimeBridge> = std::sync::OnceLock::new();

#[derive(Debug, Clone)]
pub enum StreamMessage {
    Stream(String, MessageContent),
    Done(String),
}

impl StreamMessage {
    pub fn stream(source: String, message: MessageContent) -> Self {
        StreamMessage::Stream(source, message)
    }

    pub fn done(source: String) -> Self {
        StreamMessage::Done(source)
    }
}
