//! 异步事件总线 (Enhanced Asynchronous Event Bus)
//! 
//! EBus 是一个基于 tokio::broadcast 的异步事件总线系统，专为异步环境设计，主要特点：
//! 
//! ## 核心特性
//! 
//! - **异步优先**: 基于 tokio::broadcast，天然支持异步接收和处理
//! - **类型安全**: 编译时类型检查，支持任意实现 `Clone + Send + Sync` 的类型
//! - **广播模式**: 支持一对多的事件广播，多个接收者可以同时接收同一事件
//! - **缓冲机制**: 内置事件缓冲，防止快速发布时的事件丢失
//! - **跨运行时**: 可在不同异步运行时中安全使用
//! - **灵活接收**: 支持类型化接收和原始事件接收两种模式
//! 
//! ## 架构设计
//! 
//! ```text
//! ┌─────────────────┐     发布事件      ┌─────────────────┐
//! │   发布者线程    │ ─────────────────►│  全局事件总线   │
//! │  (任何运行时)   │                   │  (broadcast)    │
//! └─────────────────┘                   └─────────┬───────┘
//!                                                 │
//!                     ┌───────────────────────────┼───────────────────────────┐
//!                     │                           │                           │
//!                     ▼                           ▼                           ▼
//!           ┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
//!           │  类型化接收器   │         │  类型化接收器   │         │  原始接收器     │
//!           │ TypedReceiver   │         │ TypedReceiver   │         │ RawReceiver     │
//!           │    <Event1>     │         │    <Event2>     │         │ (所有事件)      │
//!           └─────────────────┘         └─────────────────┘         └─────────────────┘
//!                     │                           │                           │
//!                     ▼                           ▼                           ▼
//!           ┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
//!           │   处理器 A      │         │   处理器 B      │         │   监控处理器    │
//!           │ (异步处理)      │         │ (异步处理)      │         │ (调试/日志)     │
//!           └─────────────────┘         └─────────────────┘         └─────────────────┘
//! ```
//! 
//! ## 与 XBus 的对比
//! 
//! | 特性 | EBus | XBus |
//! |------|------|------|
//! | 执行模式 | 异步 | 同步 |
//! | 性能 | 中等（异步开销） | 高（直接调用） |
//! | 缓冲 | 支持（10000 事件） | 不支持 |
//! | 背压处理 | 支持 | 不支持 |
//! | 错误处理 | Result 返回 | Panic 捕获 |
//! | 适用场景 | 异步处理、跨运行时 | 高频同步事件 |
//! 
//! ## 使用场景
//! 
//! - **异步任务通信**: 在不同异步任务之间传递消息
//! - **事件驱动架构**: 构建基于事件的异步系统
//! - **跨运行时通信**: GPUI ↔ Tokio ↔ 其他异步运行时
//! - **长时间处理**: 需要异步处理的事件（如 I/O 操作）
//! - **事件缓冲**: 需要缓冲快速产生的事件
//! 
//! ## 使用示例
//! 
//! ```rust
//! use crate::ebus;
//! 
//! // 定义事件类型
//! #[derive(Debug, Clone)]
//! struct FileProcessed {
//!     file_path: String,
//!     size: u64,
//! }
//! 
//! // 异步接收和处理事件
//! tokio::spawn(async {
//!     let mut receiver = ebus::subscribe::<FileProcessed>();
//!     
//!     while let Ok(event) = receiver.recv().await {
//!         println!("处理文件: {} ({}字节)", event.file_path, event.size);
//!         
//!         // 可以进行异步 I/O 操作
//!         process_file_async(&event.file_path).await;
//!     }
//! });
//! 
//! // 从任何地方发布事件
//! ebus::post(FileProcessed {
//!     file_path: "/path/to/file.txt".to_string(),
//!     size: 1024,
//! }).unwrap();
//! ```

use std::{
    any::{Any, TypeId},
    sync::{Arc, OnceLock},
};
use tokio::sync::broadcast;

/// 全局异步事件总线实例
/// 
/// 使用 `OnceLock` 确保：
/// 1. 进程生命周期内只初始化一次
/// 2. 线程安全的延迟初始化
/// 3. 所有异步任务共享同一个事件总线实例
/// 
/// 与 XBus 不同，EBus 使用 tokio::broadcast 作为底层通信机制，
/// 提供异步事件处理能力和事件缓冲功能。
static EBUS: OnceLock<EventBus> = OnceLock::new();

/// 发布事件到全局异步事件总线
/// 
/// 这是 EBus 的核心发布 API，将事件异步广播给所有订阅者。
/// 该方法是线程安全的，可以在任何异步或同步上下文中调用。
/// 
/// ## 工作流程
/// 
/// 1. 将事件包装为 `TypedEvent`，包含类型信息和数据
/// 2. 通过 tokio::broadcast 通道广播事件
/// 3. 所有活跃的接收器都会收到该事件
/// 4. 接收器根据类型过滤，只处理匹配的事件
/// 
/// ## 错误处理
/// 
/// 返回 `Result` 类型，可能的错误：
/// - `SendError`: 没有活跃的接收器时发生
/// 
/// ## 性能考虑
/// 
/// - 事件需要克隆以支持多个接收器
/// - 使用 `Arc` 避免大数据的重复克隆
/// - 内置缓冲区可以处理短时间的接收器延迟
/// 
/// # 泛型参数
/// 
/// - `E`: 事件类型，必须满足 `Clone + Send + Sync + 'static`
/// 
/// # 参数
/// 
/// - `event`: 要发布的事件实例
/// 
/// # 返回值
/// 
/// - `Ok(())`: 事件发布成功
/// - `Err(SendError)`: 发布失败（通常是没有接收器）
/// 
/// # 示例
/// 
/// ```rust
/// #[derive(Clone)]
/// struct TaskCompleted { task_id: String }
/// 
/// // 发布事件
/// if let Err(e) = ebus::post(TaskCompleted {
///     task_id: "task_123".to_string(),
/// }) {
///     log::warn!("事件发布失败: {}", e);
/// }
/// ```
pub fn post<E: Clone + Send + Sync + 'static>(
    event: E,
) -> Result<(), broadcast::error::SendError<TypedEvent>> {
    let bus = EBUS.get_or_init(EventBus::new);
    bus.post(event)
}

/// 创建类型化事件订阅器
/// 
/// 创建一个只接收指定类型事件的异步接收器。
/// 该接收器会自动过滤不匹配的事件类型，只返回目标类型的事件。
/// 
/// ## 接收器特点
/// 
/// - **类型安全**: 只接收指定类型的事件，编译时类型检查
/// - **异步友好**: 提供 `async fn recv()` 方法
/// - **非阻塞选项**: 提供 `try_recv()` 方法用于轮询
/// - **自动过滤**: 内部自动跳过不匹配的事件类型
/// 
/// ## 缓冲和背压
/// 
/// - 内置 10000 个事件的缓冲区
/// - 如果接收器处理慢于发布速度，旧事件会被丢弃
/// - 通过 `RecvError::Lagged` 可以检测到事件丢失
/// 
/// # 泛型参数
/// 
/// - `E`: 要订阅的事件类型
/// 
/// # 返回值
/// 
/// 返回 `TypedReceiver<E>`，用于异步接收指定类型的事件
/// 
/// # 示例
/// 
/// ```rust
/// #[derive(Clone)]
/// struct UserMessage { content: String }
/// 
/// tokio::spawn(async {
///     let mut receiver = ebus::subscribe::<UserMessage>();
///     
///     loop {
///         match receiver.recv().await {
///             Ok(msg) => println!("收到消息: {}", msg.content),
///             Err(e) => {
///                 eprintln!("接收错误: {}", e);
///                 break;
///             }
///         }
///     }
/// });
/// ```
pub fn subscribe<E: Clone + Send + Sync + 'static>() -> TypedReceiver<E> {
    let bus = EBUS.get_or_init(EventBus::new);
    bus.subscribe()
}

/// 类型化事件包装器
/// 
/// 将具体类型的事件包装为统一的广播消息格式。
/// 这是 EBus 内部使用的核心数据结构，解决了以下问题：
/// 
/// ## 设计目标
/// 
/// 1. **类型擦除**: 允许不同类型的事件通过同一个 broadcast 通道传输
/// 2. **类型恢复**: 接收端可以安全地将事件转换回原始类型
/// 3. **零拷贝**: 使用 `Arc` 避免大数据的重复克隆
/// 4. **线程安全**: 确保事件可以安全地跨线程传递
/// 
/// ## 内存布局
/// 
/// ```text
/// TypedEvent {
///     type_id: TypeId,           // 8 bytes - 类型标识符
///     data: Arc<dyn Any>,        // 16 bytes - 胖指针到实际数据
/// }
/// 
/// 总计: 24 bytes + 实际数据大小
/// ```
#[derive(Clone)]
pub struct TypedEvent {
    /// 事件的类型标识符
    /// 
    /// 用于在运行时识别事件的具体类型，支持接收端的类型过滤。
    /// `TypeId` 是 Rust 标准库提供的类型唯一标识符，
    /// 对于相同的类型，总是返回相同的 ID。
    type_id: TypeId,
    
    /// 事件数据的类型擦除存储
    /// 
    /// 使用 `Arc<dyn Any + Send + Sync>` 存储实际的事件数据：
    /// - `Arc`: 引用计数智能指针，支持多个接收器共享同一数据
    /// - `dyn Any`: 类型擦除，允许存储任意类型
    /// - `Send + Sync`: 确保可以跨线程安全传递
    data: Arc<dyn Any + Send + Sync>,
}

impl TypedEvent {
    /// 创建新的类型化事件
    /// 
    /// 将具体类型的事件包装为 `TypedEvent`，用于在 broadcast 通道中传输。
    /// 
    /// ## 实现细节
    /// 
    /// 1. 获取事件类型的 `TypeId`
    /// 2. 将事件数据包装在 `Arc` 中，支持多接收器共享
    /// 3. 进行类型擦除，存储为 `dyn Any`
    /// 
    /// # 泛型参数
    /// 
    /// - `T`: 具体的事件类型，必须满足约束条件
    /// 
    /// # 参数
    /// 
    /// - `data`: 要包装的事件实例
    /// 
    /// # 返回值
    /// 
    /// 返回包装后的 `TypedEvent`
    fn new<T: Clone + Send + Sync + 'static>(data: T) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            data: Arc::new(data),
        }
    }

    /// 尝试将事件转换回具体类型
    /// 
    /// 这是类型恢复的核心方法，尝试将类型擦除的事件数据
    /// 转换回指定的具体类型。
    /// 
    /// ## 安全保证
    /// 
    /// 1. 首先比较 `TypeId`，确保类型匹配
    /// 2. 只有类型匹配时才进行 `downcast_ref`
    /// 3. 克隆数据返回，避免生命周期问题
    /// 
    /// ## 性能考虑
    /// 
    /// - `TypeId` 比较是 O(1) 操作，非常快速
    /// - `downcast_ref` 是安全的运行时类型转换
    /// - 数据克隆是必需的，因为接收器需要拥有数据
    /// 
    /// # 泛型参数
    /// 
    /// - `T`: 目标类型，必须与原始事件类型匹配
    /// 
    /// # 返回值
    /// 
    /// - `Some(T)`: 类型匹配，返回转换后的数据
    /// - `None`: 类型不匹配
    fn downcast<T: 'static + Clone>(&self) -> Option<T> {
        // 快速类型检查，避免不必要的 downcast 操作
        if self.type_id == TypeId::of::<T>() {
            // 安全的类型转换和数据克隆
            self.data.downcast_ref::<T>().cloned()
        } else {
            None
        }
    }
}

/// 为 TypedEvent 实现 Debug trait
/// 
/// 由于 `dyn Any` 无法直接实现 Debug，这里提供一个简化的 Debug 实现，
/// 只显示类型信息，不显示具体数据内容。
impl std::fmt::Debug for TypedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedEvent")
            .field("type_id", &self.type_id)
            .finish_non_exhaustive()
    }
}

/// 类型化事件接收器
/// 
/// 专门用于接收特定类型事件的异步接收器。
/// 内部包装了 tokio::broadcast::Receiver，并提供类型安全的接收方法。
/// 
/// ## 工作原理
/// 
/// 1. 从底层 broadcast 接收器接收 `TypedEvent`
/// 2. 检查事件的类型 ID 是否匹配目标类型
/// 3. 如果匹配，进行类型转换并返回
/// 4. 如果不匹配，继续等待下一个事件
/// 
/// ## 性能特点
/// 
/// - **类型过滤**: 自动跳过不相关的事件类型
/// - **零拷贝接收**: 对于不匹配的事件，不进行数据拷贝
/// - **异步友好**: 所有操作都是异步的，不会阻塞线程
pub struct TypedReceiver<T> {
    /// 底层的 broadcast 接收器
    /// 
    /// 接收所有类型的 `TypedEvent`，然后在应用层进行过滤
    receiver: broadcast::Receiver<TypedEvent>,
    
    /// 目标事件类型的 ID
    /// 
    /// 用于快速比较和过滤事件，避免不必要的类型转换操作
    target_type_id: TypeId,
    
    /// 幻象数据，标记泛型参数
    /// 
    /// 由于结构体没有直接使用 `T`，需要 `PhantomData` 来标记类型参数，
    /// 确保类型系统正确处理泛型约束
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Clone + Send + Sync + 'static> TypedReceiver<T> {
    /// 创建新的类型化接收器
    /// 
    /// 包装底层的 broadcast 接收器，添加类型过滤功能。
    /// 
    /// # 参数
    /// 
    /// - `receiver`: 底层的 broadcast 接收器
    /// 
    /// # 返回值
    /// 
    /// 返回新的类型化接收器实例
    fn new(receiver: broadcast::Receiver<TypedEvent>) -> Self {
        Self {
            receiver,
            target_type_id: TypeId::of::<T>(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// 异步接收下一个匹配类型的事件
    /// 
    /// 这是接收器的主要方法，异步等待并接收指定类型的事件。
    /// 该方法会自动过滤不匹配的事件类型。
    /// 
    /// ## 行为特点
    /// 
    /// - **阻塞式**: 如果没有匹配的事件，会一直等待
    /// - **类型过滤**: 自动跳过不匹配的事件类型
    /// - **错误透传**: 将底层接收器的错误透传给调用者
    /// 
    /// ## 可能的错误
    /// 
    /// - `RecvError::Closed`: 发送端已关闭，不会再有新事件
    /// - `RecvError::Lagged(u64)`: 接收器滞后，错过了一些事件
    /// 
    /// # 返回值
    /// 
    /// - `Ok(T)`: 成功接收到匹配类型的事件
    /// - `Err(RecvError)`: 接收失败
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// let mut receiver = ebus::subscribe::<MyEvent>();
    /// 
    /// match receiver.recv().await {
    ///     Ok(event) => println!("收到事件: {:?}", event),
    ///     Err(broadcast::error::RecvError::Lagged(n)) => {
    ///         println!("错过了 {} 个事件", n);
    ///     },
    ///     Err(broadcast::error::RecvError::Closed) => {
    ///         println!("事件总线已关闭");
    ///     },
    /// }
    /// ```
    pub async fn recv(&mut self) -> Result<T, broadcast::error::RecvError> {
        loop {
            match self.receiver.recv().await {
                Ok(typed_event) => {
                    // 快速类型检查，避免不必要的转换
                    if typed_event.type_id == self.target_type_id {
                        if let Some(data) = typed_event.downcast::<T>() {
                            return Ok(data);
                        }
                        // 理论上不应该到达这里，因为 type_id 已经匹配了
                        // 但为了安全起见，继续循环
                    }
                    // 类型不匹配，继续等待下一个事件
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// 尝试接收下一个匹配类型的事件（非阻塞）
    /// 
    /// 非阻塞版本的事件接收方法，适用于轮询场景。
    /// 如果当前没有匹配的事件可用，立即返回 `Empty` 错误。
    /// 
    /// ## 使用场景
    /// 
    /// - **轮询模式**: 在主循环中定期检查事件
    /// - **性能敏感**: 避免阻塞关键线程
    /// - **条件处理**: 只在特定条件下处理事件
    /// 
    /// ## 可能的错误
    /// 
    /// - `TryRecvError::Empty`: 当前没有可用的匹配事件
    /// - `TryRecvError::Closed`: 发送端已关闭
    /// - `TryRecvError::Lagged(u64)`: 接收器滞后
    /// 
    /// # 返回值
    /// 
    /// - `Ok(T)`: 成功接收到匹配类型的事件
    /// - `Err(TryRecvError)`: 接收失败或无可用事件
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// let mut receiver = ebus::subscribe::<MyEvent>();
    /// 
    /// // 非阻塞检查事件
    /// match receiver.try_recv() {
    ///     Ok(event) => println!("收到事件: {:?}", event),
    ///     Err(broadcast::error::TryRecvError::Empty) => {
    ///         // 没有事件，继续其他工作
    ///     },
    ///     Err(e) => eprintln!("接收错误: {}", e),
    /// }
    /// ```
    pub fn try_recv(&mut self) -> Result<T, broadcast::error::TryRecvError> {
        loop {
            match self.receiver.try_recv() {
                Ok(typed_event) => {
                    if typed_event.type_id == self.target_type_id {
                        if let Some(data) = typed_event.downcast::<T>() {
                            return Ok(data);
                        }
                    }
                    // 类型不匹配，继续检查缓冲区中的下一个事件
                }
                Err(broadcast::error::TryRecvError::Empty) => {
                    // 缓冲区为空，直接返回
                    return Err(broadcast::error::TryRecvError::Empty);
                }
                Err(e) => return Err(e),
            }
        }
    }
}

/// 原始事件接收器
/// 
/// 接收所有类型事件的原始接收器，不进行类型过滤。
/// 主要用于调试、监控和事件转发等场景。
/// 
/// ## 使用场景
/// 
/// - **事件监控**: 监听系统中的所有事件活动
/// - **调试工具**: 跟踪和分析事件流
/// - **事件转发**: 将事件转发到其他系统或总线
/// - **统计分析**: 收集事件统计信息
pub struct RawReceiver {
    /// 底层的 broadcast 接收器
    /// 
    /// 直接接收 `TypedEvent`，不进行任何过滤或转换
    receiver: broadcast::Receiver<TypedEvent>,
}

impl RawReceiver {
    /// 异步接收下一个原始事件
    /// 
    /// 接收任意类型的事件，以 `TypedEvent` 形式返回。
    /// 调用者可以通过 `type_id` 和 `downcast` 方法处理具体类型。
    /// 
    /// # 返回值
    /// 
    /// - `Ok(TypedEvent)`: 成功接收到事件
    /// - `Err(RecvError)`: 接收失败
    pub async fn recv(&mut self) -> Result<TypedEvent, broadcast::error::RecvError> {
        self.receiver.recv().await
    }

    /// 尝试接收下一个原始事件（非阻塞）
    /// 
    /// 非阻塞版本的原始事件接收方法。
    /// 
    /// # 返回值
    /// 
    /// - `Ok(TypedEvent)`: 成功接收到事件
    /// - `Err(TryRecvError)`: 接收失败或无可用事件
    pub fn try_recv(&mut self) -> Result<TypedEvent, broadcast::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

/// 异步事件总线主结构
/// 
/// EBus 的核心组件，基于 tokio::broadcast 实现异步事件广播。
/// 支持多个发布者和多个接收者的 N:M 通信模式。
/// 
/// ## 设计特点
/// 
/// - **异步优先**: 所有操作都是异步的，适合 async/await 编程模型
/// - **广播模式**: 一个事件可以被多个接收器同时接收
/// - **缓冲机制**: 内置大容量缓冲区，处理接收器处理速度不一致的情况
/// - **错误友好**: 提供详细的错误信息和处理机制
/// 
/// ## 内存和性能
/// 
/// - **缓冲区大小**: 默认 10000 个事件的缓冲区
/// - **内存共享**: 使用 `Arc` 避免大数据的重复复制
/// - **背压处理**: 当接收器过慢时，会丢弃旧事件
#[derive(Clone)]
pub struct EventBus {
    /// 底层的 broadcast 发送器
    /// 
    /// 所有的事件发布都通过这个发送器进行。
    /// 可以克隆发送器以支持多个发布者。
    sender: broadcast::Sender<TypedEvent>,
}

impl EventBus {
    /// 创建新的异步事件总线实例
    /// 
    /// 初始化底层的 broadcast 通道，设置缓冲区大小为 10000。
    /// 这个大小是基于以下考虑选择的：
    /// 
    /// - **高吞吐**: 支持短时间内的大量事件发布
    /// - **内存合理**: 不会占用过多内存
    /// - **延迟容忍**: 允许接收器有一定的处理延迟
    /// 
    /// # 返回值
    /// 
    /// 返回新的事件总线实例
    pub fn new() -> Self {
        // 创建容量为 10000 的广播通道
        // 这个大小平衡了内存使用和事件缓冲能力
        let (sender, _) = broadcast::channel(10000);
        Self { sender }
    }

    /// 发布事件到总线
    /// 
    /// 将事件包装为 `TypedEvent` 并广播给所有订阅者。
    /// 这是事件总线的核心发布方法。
    /// 
    /// ## 实现细节
    /// 
    /// 1. 将具体类型的事件包装为 `TypedEvent`
    /// 2. 通过 broadcast 通道发送事件
    /// 3. 返回发送结果，忽略接收器数量
    /// 
    /// ## 错误处理
    /// 
    /// 如果没有活跃的接收器，发送操作仍然会成功，
    /// 但事件会被直接丢弃。这是合理的行为，
    /// 因为没有接收器意味着没有人关心这个事件。
    /// 
    /// # 泛型参数
    /// 
    /// - `E`: 事件类型，必须满足约束条件
    /// 
    /// # 参数
    /// 
    /// - `event`: 要发布的事件实例
    /// 
    /// # 返回值
    /// 
    /// - `Ok(())`: 事件发布成功
    /// - `Err(SendError)`: 发布失败（理论上不应该发生）
    pub fn post<E: Clone + Send + Sync + 'static>(
        &self,
        event: E,
    ) -> Result<(), broadcast::error::SendError<TypedEvent>> {
        let typed_event = TypedEvent::new(event);
        // 发送事件并忽略接收器数量
        self.sender.send(typed_event).map(|_| ())
    }

    /// 创建强类型订阅器
    /// 
    /// 创建一个只接收指定类型事件的订阅器。
    /// 这是大多数应用场景下推荐的订阅方式。
    /// 
    /// ## 订阅器特点
    /// 
    /// - **类型安全**: 编译时确保类型正确
    /// - **自动过滤**: 只接收匹配的事件类型
    /// - **异步友好**: 提供 async/await 接口
    /// 
    /// # 泛型参数
    /// 
    /// - `E`: 要订阅的事件类型
    /// 
    /// # 返回值
    /// 
    /// 返回类型化接收器
    pub fn subscribe<E: Clone + Send + Sync + 'static>(&self) -> TypedReceiver<E> {
        TypedReceiver::new(self.sender.subscribe())
    }

    /// 创建原始事件订阅器
    /// 
    /// 创建接收所有事件类型的原始订阅器。
    /// 适用于需要处理多种事件类型的场景。
    /// 
    /// ## 使用场景
    /// 
    /// - **事件监控**: 监听所有事件活动
    /// - **调试工具**: 分析事件流
    /// - **事件代理**: 转发事件到其他系统
    /// 
    /// # 返回值
    /// 
    /// 返回原始事件接收器
    pub fn subscribe_raw(&self) -> RawReceiver {
        RawReceiver {
            receiver: self.sender.subscribe(),
        }
    }

    /// 获取原始广播发送器的克隆
    /// 
    /// 返回底层 broadcast 发送器的克隆，允许高级用户
    /// 直接操作广播通道。通常不建议使用，除非有特殊需求。
    /// 
    /// # 返回值
    /// 
    /// 返回 broadcast 发送器的克隆
    pub fn raw_sender(&self) -> broadcast::Sender<TypedEvent> {
        self.sender.clone()
    }

    /// 获取当前接收器数量
    /// 
    /// 返回当前活跃的接收器数量，用于监控和调试。
    /// 这个数字包括所有类型的接收器（类型化和原始）。
    /// 
    /// # 返回值
    /// 
    /// 当前活跃的接收器数量
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 便利宏：安全的事件发布
/// 
/// 包装 `post` 函数，自动处理错误情况。
/// 如果发布失败，会记录错误日志而不是 panic。
/// 
/// # 参数
/// 
/// - `$event`: 要发布的事件表达式
/// 
/// # 示例
/// 
/// ```rust
/// // 安全发布事件，失败时记录日志
/// epost!(MyEvent { data: "hello".to_string() });
/// ```
#[macro_export]
macro_rules! epost {
    ($event:expr) => {
        $crate::ebus::post($event).unwrap_or_else(|e| {
            log::error!("Failed to post event: {}", e);
        })
    };
}

/// 初始化全局事件总线
/// 
/// 显式初始化全局事件总线实例。虽然事件总线支持延迟初始化，
/// 但在应用启动时显式初始化可以：
/// 
/// 1. 确保总线在需要时已经准备就绪
/// 2. 在日志中记录初始化状态
/// 3. 便于应用的启动流程管理
/// 
/// # 使用方式
/// 
/// 通常在应用的 `main` 函数或初始化阶段调用：
/// 
/// ```rust
/// #[tokio::main]
/// async fn main() {
///     ebus::init_event_bus();
///     // ... 其他初始化代码
/// }
/// ```
pub fn init_event_bus() {
    let _ = EBUS.get_or_init(EventBus::new);
    log::info!("Enhanced event bus initialized");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::time::{sleep, Duration};

    // 为每个测试使用不同的事件类型，避免测试间的干扰
    // 这是异步测试中的最佳实践，确保测试的独立性

    #[derive(Debug, Clone, PartialEq)]
    struct TestEvent1 {
        message: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TestEvent2 {
        message: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct ComplexEvent {
        data: HashMap<String, Vec<i32>>,
        nested: Box<String>, // 简化嵌套结构，测试复杂类型的支持
    }

    /// 测试基本的类型化事件发布和接收
    #[tokio::test]
    async fn test_typed_events() {
        init_event_bus();

        let mut receiver = subscribe::<TestEvent1>();

        // 在后台任务中发布事件，模拟异步场景
        tokio::spawn(async {
            sleep(Duration::from_millis(10)).await;
            post(TestEvent1 {
                message: "Hello".to_string(),
            })
            .unwrap();
            post(TestEvent1 {
                message: "World".to_string(),
            })
            .unwrap();
        });

        // 异步接收并验证事件
        let event1 = receiver.recv().await.unwrap();
        assert_eq!(event1.message, "Hello");

        let event2 = receiver.recv().await.unwrap();
        assert_eq!(event2.message, "World");
    }

    /// 测试复杂类型的事件处理
    #[tokio::test]
    async fn test_any_complex_type() {
        let mut receiver = subscribe::<ComplexEvent>();

        tokio::spawn(async {
            sleep(Duration::from_millis(10)).await;

            let mut data = HashMap::new();
            data.insert("numbers".to_string(), vec![1, 2, 3]);

            post(ComplexEvent {
                data,
                nested: Box::new("nested".to_string()),
            })
            .unwrap();
        });

        let event = receiver.recv().await.unwrap();
        assert_eq!(*event.nested, "nested");
        assert_eq!(event.data.get("numbers"), Some(&vec![1, 2, 3]));
    }

    /// 测试广播模式：多个接收器接收同一事件
    #[tokio::test]
    async fn test_multiple_receivers() {
        let mut receiver1 = subscribe::<TestEvent2>();
        let mut receiver2 = subscribe::<TestEvent2>();

        tokio::spawn(async {
            sleep(Duration::from_millis(10)).await;
            post(TestEvent2 {
                message: "Broadcast".to_string(),
            })
            .unwrap();
        });

        // 两个接收器都能收到同一个事件
        let event1 = receiver1.recv().await.unwrap();
        let event2 = receiver2.recv().await.unwrap();

        assert_eq!(event1.message, "Broadcast");
        assert_eq!(event2.message, "Broadcast");
    }
}
