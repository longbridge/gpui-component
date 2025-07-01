//! 跨运行时桥接器 (Cross-Runtime Bridge)
//!
//! 该模块提供了一个桥接机制，用于在不同的异步运行时之间进行安全通信。
//! 主要解决 GPUI 运行时与 Actix Actor 运行时之间的数据交换问题。
//!
//! ## 架构设计
//!
//! ```text
//! ┌─────────────────┐     消息通道      ┌─────────────────┐
//! │   GPUI 运行时   │ ────────────────► │  Actix 运行时   │
//! │  (UI 界面)      │     oneshot       │  (MCP 管理)     │
//! │                 │ ◄──────────────── │                 │
//! └─────────────────┘     响应          └─────────────────┘
//!          │                                      │
//!          ▼                                      ▼
//!   CrossRuntimeBridge                    McpRegistry Actor
//!     (发送请求)                           (处理业务逻辑)
//! ```
//!
//! ## 核心特性
//!
//! - **异步安全**: 使用 tokio 的 mpsc/oneshot 通道进行线程安全通信
//! - **类型安全**: 通过枚举定义明确的消息类型，避免运行时错误
//! - **全局访问**: 提供全局单例，方便在任何地方访问桥接功能
//! - **错误处理**: 完善的错误处理机制，避免运行时崩溃
//! - **请求-响应模式**: 支持同步等待响应，简化异步编程复杂度

use crate::backoffice::mcp::server::McpServerInstance;
use crate::backoffice::mcp::{
    GetAllInstances, GetServerInstance, McpCallToolRequest, McpCallToolResult, McpRegistry,
};
use std::{collections::HashMap, sync::Arc};
use actix::Arbiter;
use tokio::sync::{mpsc, oneshot};

/// 跨运行时通信消息定义
///
/// 定义了所有支持的跨运行时通信消息类型。每个消息都包含：
/// 1. 请求参数
/// 2. oneshot::Sender 用于发送响应
///
/// 这种设计确保了每个请求都能得到对应的响应，实现了异步的请求-响应模式。
#[derive(Debug)]
pub enum CrossRuntimeMessage {
    /// 获取指定服务器实例
    ///
    /// 用于从 GPUI 查询特定 MCP 服务器的运行状态和配置信息
    GetInstance {
        /// MCP 服务器的唯一标识符
        server_id: String,
        /// 响应通道，返回服务器实例或 None（如果不存在）
        response: oneshot::Sender<Option<McpServerInstance>>,
    },

    /// 调用 MCP 服务器工具
    ///
    /// 用于从 GPUI 调用指定 MCP 服务器上的工具函数
    CallTool {
        /// 目标 MCP 服务器 ID
        server_id: String,
        /// 要调用的工具名称
        tool_name: String,
        /// 工具参数（JSON 字符串格式）
        arguments: String,
        /// 响应通道，返回工具调用结果或错误信息
        response: oneshot::Sender<Result<McpCallToolResult, String>>,
    },

    /// 获取所有服务器实例
    ///
    /// 用于从 GPUI 获取当前所有 MCP 服务器的状态列表
    GetAllInstances {
        /// 响应通道，返回服务器 ID 到实例的映射
        response: oneshot::Sender<HashMap<String, McpServerInstance>>,
    },
}

/// 跨运行时通信桥接器
///
/// 该结构体封装了跨运行时通信的复杂性，为 GPUI 运行时提供简单的异步 API。
/// 内部使用 mpsc 无界通道将消息发送到 Actix 运行时进行处理。
///
/// ## 工作原理
///
/// 1. GPUI 调用桥接器方法
/// 2. 桥接器创建 oneshot 通道用于接收响应
/// 3. 将请求消息通过 mpsc 通道发送到处理器
/// 4. 处理器在 Actix 运行时中执行实际业务逻辑
/// 5. 通过 oneshot 通道返回结果给 GPUI
/// 6. GPUI 异步等待并处理响应
pub struct CrossRuntimeBridge {
    /// 用于发送跨运行时消息的无界通道发送端
    ///
    /// 使用无界通道避免在高并发场景下的阻塞问题，
    /// 同时简化了错误处理逻辑
    sender: mpsc::UnboundedSender<CrossRuntimeMessage>,
}

impl CrossRuntimeBridge {
    /// 创建新的桥接器实例
    ///
    /// 返回桥接器实例和消息接收端。接收端应该在 Actix 运行时中
    /// 启动一个处理循环来处理传入的消息。
    ///
    /// # 返回值
    ///
    /// - `CrossRuntimeBridge`: 用于发送消息的桥接器实例
    /// - `mpsc::UnboundedReceiver<CrossRuntimeMessage>`: 用于接收消息的接收端
    pub fn new() -> (Self, mpsc::UnboundedReceiver<CrossRuntimeMessage>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self { sender }, receiver)
    }

    /// 异步获取指定服务器实例
    ///
    /// 这是一个 GUI 安全的方法，可以在 GPUI 运行时中安全调用。
    /// 内部处理了所有的通道通信和错误处理。
    ///
    /// # 参数
    ///
    /// - `server_id`: MCP 服务器的唯一标识符
    ///
    /// # 返回值
    ///
    /// - `Some(McpServerInstance)`: 如果服务器存在且正在运行
    /// - `None`: 如果服务器不存在或通信失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// if let Some(bridge) = McpRegistry::get_bridge() {
    ///     if let Some(instance) = bridge.get_instance("my-server".to_string()).await {
    ///         println!("服务器状态: {:?}", instance.status);
    ///     }
    /// }
    /// ```
    pub async fn get_instance(&self, server_id: String) -> Option<McpServerInstance> {

        let (response_tx, response_rx) = oneshot::channel();
        println!("请求获取服务器实例 in GUI: {}", server_id);
        // 发送请求到 Actix 运行时
        if self
            .sender
            .send(CrossRuntimeMessage::GetInstance {
                server_id,
                response: response_tx,
            })
            .is_err()
        {
            // 如果发送失败，说明接收端已关闭或桥接器未初始化
            return None;
        }

        // 异步等待响应，如果接收失败返回 None
        response_rx.await.unwrap_or(None)
    }

    /// 异步调用 MCP 服务器工具
    ///
    /// 允许 GPUI 运行时调用运行在 Actix 运行时中的 MCP 服务器工具。
    /// 这是实现 GUI 与 MCP 服务器交互的核心方法。
    ///
    /// # 参数
    ///
    /// - `server_id`: 目标 MCP 服务器 ID
    /// - `tool_name`: 要调用的工具名称
    /// - `arguments`: 工具参数，JSON 字符串格式
    ///
    /// # 返回值
    ///
    /// - `Ok(McpCallToolResult)`: 工具调用成功，包含返回结果
    /// - `Err(String)`: 工具调用失败，包含错误描述
    ///
    /// # 示例
    ///
    /// ```rust
    /// let result = bridge.call_tool(
    ///     "file-server".to_string(),
    ///     "read_file".to_string(),
    ///     r#"{"path": "/tmp/test.txt"}"#.to_string(),
    /// ).await;
    ///
    /// match result {
    ///     Ok(tool_result) => println!("工具执行结果: {}", tool_result.content),
    ///     Err(e) => eprintln!("工具执行失败: {}", e),
    /// }
    /// ```
    pub async fn call_tool(
        &self,
        server_id: String,
        tool_name: String,
        arguments: String,
    ) -> Result<McpCallToolResult, String> {
        let (response_tx, response_rx) = oneshot::channel();

        // 发送工具调用请求
        if self
            .sender
            .send(CrossRuntimeMessage::CallTool {
                server_id,
                tool_name,
                arguments,
                response: response_tx,
            })
            .is_err()
        {
            return Err("Failed to send message to actor runtime".to_string());
        }

        // 等待工具执行结果
        response_rx
            .await
            .map_err(|_| "Failed to receive response from actor runtime".to_string())?
    }

    /// 异步获取所有服务器实例
    ///
    /// 用于 GUI 显示当前所有 MCP 服务器的状态列表。
    /// 返回一个从服务器 ID 到服务器实例的映射。
    ///
    /// # 返回值
    ///
    /// - `HashMap<String, McpServerInstance>`: 服务器 ID 到实例的映射
    /// - 如果通信失败，返回空的 HashMap
    ///
    /// # 示例
    ///
    /// ```rust
    /// let instances = bridge.get_all_instances().await;
    /// for (id, instance) in instances {
    ///     println!("服务器 {}: {:?}", id, instance.status);
    /// }
    /// ```
    pub async fn get_all_instances(&self) -> HashMap<String, McpServerInstance> {
        let (response_tx, response_rx) = oneshot::channel();

        if self
            .sender
            .send(CrossRuntimeMessage::GetAllInstances {
                response: response_tx,
            })
            .is_err()
        {
            return HashMap::new();
        }

        response_rx.await.unwrap_or_default()
    }
}

/// 全局桥接实例
///
/// 使用 OnceLock 确保桥接器在整个应用程序生命周期中只初始化一次，
/// 并且可以在多线程环境中安全访问。
static CROSS_RUNTIME_BRIDGE: std::sync::OnceLock<Arc<CrossRuntimeBridge>> =
    std::sync::OnceLock::new();

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
        let (bridge, mut receiver) = CrossRuntimeBridge::new();
        let bridge = Arc::new(bridge);

        // 设置全局桥接实例，如果已经设置过则忽略
        CROSS_RUNTIME_BRIDGE.set(bridge.clone()).ok();
        // 启动桥接消息处理器
        // 这个任务会一直运行，直到应用程序结束
      Arbiter::new().spawn(async move {
            while let Some(message) = receiver.recv().await {
                match message {
                    // 处理获取服务器实例请求
                    CrossRuntimeMessage::GetInstance {
                        server_id,
                        response,
                    } => {
                        println!("请求获取服务器实例 in Actix({}): {}",actix::System::current().id(), server_id);
                        let registry = McpRegistry::global();
                        let result = registry.send(GetServerInstance { server_id }).await;
                        let instance = result.unwrap_or(None);
                        println!("获取服务器实例结果: {:?}", instance);
                        // 忽略发送错误，因为接收端可能已经超时或取消
                        if let Err(err)= response.send(instance) {
                            eprintln!("Failed to send response for GetInstance: {:?}", err);
                        }
                    }

                    // 处理工具调用请求
                    CrossRuntimeMessage::CallTool {
                        server_id,
                        tool_name,
                        arguments,
                        response,
                    } => {
                        let registry = McpRegistry::global();
                        let result = registry
                            .send(McpCallToolRequest {
                                id: server_id,
                                name: tool_name,
                                arguments,
                            })
                            .await;

                        // 将 Actor 错误转换为字符串错误
                        let result = match result {
                            Ok(tool_result) => Ok(tool_result),
                            Err(e) => Err(e.to_string()),
                        };
                        let _ = response.send(result);
                    }

                    // 处理获取所有实例请求
                    CrossRuntimeMessage::GetAllInstances { response } => {
                        let registry = McpRegistry::global();
                        let result = registry.send(GetAllInstances).await;
                        let instances = result.unwrap_or_default();
                        let _ = response.send(instances);
                    }
                }
            }
        });
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
    pub fn get_bridge() -> Option<Arc<CrossRuntimeBridge>> {
        CROSS_RUNTIME_BRIDGE.get().cloned()
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
    pub async fn get_all_instances_static() -> anyhow::Result<HashMap<String, McpServerInstance>> {
        if let Some(bridge) = Self::get_bridge() {
            Ok(bridge.get_all_instances().await)
        } else {
            Err(anyhow::anyhow!("Cross-runtime bridge not initialized"))
        }
    }
}
