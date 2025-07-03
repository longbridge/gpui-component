use crate::backoffice::mcp::server::McpServerInstance;
use crate::backoffice::mcp::{
    GetAllInstances, GetServerInstance, McpCallToolRequest, McpCallToolResult, McpRegistry,
};
use actix::Arbiter;
use tokio::sync::{mpsc, oneshot};

/// 跨运行时通信桥接器
/// 
/// 这个结构体是连接 GPUI 界面线程和 Actix Actor 系统之间的关键桥梁。
/// 它解决了以下问题：
/// 
/// ## 设计目标
/// 
/// 1. **运行时隔离**: GPUI 运行在主线程，Actix Actor 运行在独立的运行时中
/// 2. **异步通信**: 提供异步接口，避免阻塞 GPUI 的渲染循环
/// 3. **并发处理**: 支持同时处理多个请求，提高系统响应性
/// 4. **错误隔离**: 单个消息处理失败不会影响其他消息的处理
/// 
/// ## 架构设计
/// 
/// ```text
/// GPUI Thread              Bridge              Actix Runtime
/// ┌─────────────┐         ┌─────────┐         ┌──────────────┐
/// │ UI 组件      │ ──────→ │ 消息队列  │ ──────→ │ MCP Registry │
/// │ 事件处理     │         │ 调度器   │         │ Actor        │
/// │ 状态更新     │ ←────── │ 响应通道  │ ←────── │ 工具调用     │
/// └─────────────┘         └─────────┘         └──────────────┘
/// ```
/// 
/// ## 核心特性
/// 
/// - **消息队列**: 使用无界通道接收来自 GPUI 的请求
/// - **独立处理**: 每个消息在独立的 Arbiter 中处理，避免相互阻塞
/// - **类型安全**: 通过泛型和 trait 确保消息处理的类型安全
/// - **资源管理**: 自动处理 Actor 生命周期和资源清理
/// 
/// ## 使用场景
/// 
/// 1. **服务器管理**: 获取、创建、删除 MCP 服务器实例
/// 2. **工具调用**: 调用 MCP 服务器提供的工具和功能
/// 3. **状态查询**: 实时获取服务器状态和运行信息
/// 4. **事件通知**: 处理服务器状态变化的通知
/// 
/// ## 性能考量
/// 
/// - 消息调度器使用最小开销的设计，仅负责分发
/// - 实际业务逻辑在独立 Arbiter 中执行，避免阻塞调度
/// - 使用 oneshot 通道确保响应的及时传递
/// - 支持高并发场景下的稳定运行
pub struct CrossRuntimeBridge {
    /// 消息调度器，负责分发消息到独立的处理任务
    /// 
    /// 这是整个桥接器的核心组件，它：
    /// - 接收来自 GPUI 的各种类型消息
    /// - 快速分发消息到对应的处理器
    /// - 保持低延迟和高吞吐量
    /// - 支持动态消息类型扩展
    /// 
    /// 使用无界通道的原因：
    /// 1. GPUI 的消息通常是用户交互触发，频率不高
    /// 2. 避免因通道容量限制导致的 GPUI 阻塞
    /// 3. 简化错误处理逻辑
    dispatcher: mpsc::UnboundedSender<Box<dyn MessageHandler + Send>>,
}

/// 消息处理器特征
///
/// 每种消息类型都实现这个特征，提供独立的处理逻辑
trait MessageHandler {
    fn handle(self: Box<Self>);
}

/// 获取服务器实例消息处理器
struct GetInstanceHandler {
    server_id: String,
    response: oneshot::Sender<Option<McpServerInstance>>,
}

impl MessageHandler for GetInstanceHandler {
    fn handle(self: Box<Self>) {
        // 在独立的 Arbiter 中处理，不会阻塞其他消息
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

            // 发送响应，忽略接收端关闭的情况
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

impl CrossRuntimeBridge {
    /// 创建新的桥接器实例
    ///
    /// 启动消息调度器，负责将消息分发到独立的处理任务
    pub fn new() -> Self {
        let (dispatcher, mut receiver) =
            mpsc::unbounded_channel::<Box<dyn MessageHandler + Send>>();

        // 启动消息调度器
        // 这个调度器只负责快速分发消息，不进行实际的业务处理
        Arbiter::new().spawn(async move {
            while let Some(handler) = receiver.recv().await {
                // 每个消息都在独立的任务中处理，避免相互阻塞
                handler.handle();
            }
        });

        Self { dispatcher }
    }

    /// 异步获取指定服务器实例
    ///
    /// 创建独立的处理器，不会被其他请求阻塞
    pub async fn get_instance(&self, server_id: String) -> Option<McpServerInstance> {
        let (response_tx, response_rx) = oneshot::channel();

        let handler = Box::new(GetInstanceHandler {
            server_id,
            response: response_tx,
        });

        if self.dispatcher.send(handler).is_err() {
            return None;
        }

        response_rx.await.unwrap_or(None)
    }

    /// 异步调用 MCP 服务器工具
    ///
    /// 每个工具调用都是独立处理，支持并发调用
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
}

static CROSS_RUNTIME_BRIDGE: std::sync::OnceLock<CrossRuntimeBridge> = std::sync::OnceLock::new();

impl McpRegistry {
    /// 初始化跨运行时桥接器
    ///
    /// 这个方法应该在应用程序启动时调用，通常在 Actix System 启动后。
    /// 它会：
    ///
    /// 1. 创建桥接器实例和消息接收器
    /// 2. 将桥接器设置为全局单例
    /// 3. 启动消息处理循环，将消息路由到对应的 Actor 方法
    ///
    /// # 消息处理流程
    ///
    /// 消息处理器运行在独立的 tokio 任务中，它：
    /// - 接收来自 GPUI 的跨运行时消息
    /// - 将消息转换为对应的 Actor 消息
    /// - 调用 McpRegistry Actor 的方法
    /// - 将响应通过 oneshot 通道返回给 GPUI
    ///
    /// # 错误处理
    ///
    /// 处理器内部使用了完善的错误处理：
    /// - 如果 Actor 调用失败，会返回默认值或错误信息
    /// - 如果响应通道发送失败（接收端已关闭），会忽略错误继续处理
    /// - 整个处理循环是容错的，单个消息处理失败不会影响其他消息
    pub fn init_crb() {
        let bridge = CrossRuntimeBridge::new();

        // 设置全局桥接实例，如果已经设置过则忽略
        CROSS_RUNTIME_BRIDGE.set(bridge).ok();
    }

    /// 获取全局桥接器实例
    ///
    /// 返回全局桥接器的 Arc 引用，如果桥接器未初始化则返回 None。
    /// 调用者应该检查返回值，确保桥接器已正确初始化。
    ///
    /// # 返回值
    ///
    /// - `Some(Arc<CrossRuntimeBridge>)`: 桥接器实例
    /// - `None`: 桥接器未初始化
    pub fn get_bridge() -> Option<&'static CrossRuntimeBridge> {
        CROSS_RUNTIME_BRIDGE.get()
    }

    /// GUI 安全的静态获取实例方法
    ///
    /// 这是一个便利方法，封装了桥接器获取和错误处理逻辑。
    /// 可以直接在 GPUI 代码中调用，无需手动处理桥接器初始化检查。
    ///
    /// # 参数
    ///
    /// - `server_id`: MCP 服务器 ID
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(McpServerInstance))`: 服务器存在且运行正常
    /// - `Ok(None)`: 服务器不存在
    /// - `Err(anyhow::Error)`: 桥接器未初始化或通信失败
    pub async fn get_instance(server_id: &str) -> anyhow::Result<Option<McpServerInstance>> {
        if let Some(bridge) = Self::get_bridge() {
            Ok(bridge.get_instance(server_id.to_string()).await)
        } else {
            Err(anyhow::anyhow!("Cross-runtime bridge not initialized"))
        }
    }

    /// GUI 安全的静态工具调用方法
    ///
    /// 便利方法，用于在 GPUI 中直接调用 MCP 服务器工具。
    /// 内部处理了所有的桥接和错误转换逻辑。
    ///
    /// # 参数
    ///
    /// - `server_id`: 目标服务器 ID
    /// - `tool_name`: 工具名称
    /// - `arguments`: 工具参数（JSON 字符串）
    ///
    /// # 返回值
    ///
    /// - `Ok(McpCallToolResult)`: 工具调用成功
    /// - `Err(anyhow::Error)`: 桥接器未初始化、通信失败或工具执行失败
    pub async fn call_tool_static(
        server_id: &str,
        tool_name: &str,
        arguments: &str,
    ) -> anyhow::Result<McpCallToolResult> {
        if let Some(bridge) = Self::get_bridge() {
            bridge
                .call_tool(
                    server_id.to_string(),
                    tool_name.to_string(),
                    arguments.to_string(),
                )
                .await
                .map_err(|e| anyhow::anyhow!(e))
        } else {
            Err(anyhow::anyhow!("Cross-runtime bridge not initialized"))
        }
    }

    /// GUI 安全的静态获取所有实例方法
    ///
    /// 便利方法，用于在 GPUI 中获取所有 MCP 服务器的状态。
    /// 通常用于 GUI 的服务器列表显示。
    ///
    /// # 返回值
    ///
    /// - `Ok(HashMap<String, McpServerInstance>)`: 所有服务器实例
    /// - `Err(anyhow::Error)`: 桥接器未初始化或通信失败
    pub async fn get_all_instances_static() -> anyhow::Result<Vec<McpServerInstance>> {
        if let Some(bridge) = Self::get_bridge() {
            Ok(bridge.get_all_instances().await)
        } else {
            Err(anyhow::anyhow!("Cross-runtime bridge not initialized"))
        }
    }
}
