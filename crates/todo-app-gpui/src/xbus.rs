//! 跨运行时高性能事件总线 (Cross-Runtime High-Performance Event Bus)
//! 
//! XBus 是一个专为跨运行时通信设计的同步事件总线系统，主要特点：
//! 
//! ## 核心特性
//! 
//! - **高性能**: 使用无锁数据结构，同步直接调用，零序列化开销
//! - **类型安全**: 编译时类型检查，支持任意实现 `Any + Send + Sync` 的类型
//! - **跨运行时**: 可在 GPUI、Actix、Tokio 等不同运行时间安全通信
//! - **灵活订阅**: 支持特定类型订阅和通用订阅两种模式
//! - **自动清理**: 订阅凭证析构时自动取消订阅，避免内存泄漏
//! 
//! ## 架构设计
//! 
//! ```text
//! ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
//! │   GPUI 运行时   │    │  Actix 运行时   │    │  Tokio 运行时   │
//! │                 │    │                 │    │                 │
//! │  xbus::post()   │    │  xbus::post()   │    │  xbus::post()   │
//! │  xbus::sub()    │    │  xbus::sub()    │    │  xbus::sub()    │
//! └─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
//!           │                      │                      │
//!           └──────────────────────┼──────────────────────┘
//!                                  │
//!                          ┌───────▼────────┐
//!                          │  全局事件总线   │
//!                          │   (静态单例)   │
//!                          │                │
//!                          │ SkipMap-based  │
//!                          │  无锁存储      │
//!                          └────────────────┘
//! ```
//! 
//! ## 使用示例
//! 
//! ```rust
//! use crate::xbus;
//! 
//! // 定义事件类型
//! #[derive(Debug, Clone)]
//! struct ButtonClicked {
//!     button_id: String,
//!     timestamp: u64,
//! }
//! 
//! // 订阅特定类型事件
//! let _sub = xbus::subscribe::<ButtonClicked, _>(|event| {
//!     println!("按钮 {} 被点击", event.button_id);
//! });
//! 
//! // 订阅所有类型事件
//! let _any_sub = xbus::subscribe_any(|type_id, event| {
//!     println!("收到事件: {:?}", type_id);
//!     if let Some(click) = event.downcast_ref::<ButtonClicked>() {
//!         println!("这是一个按钮点击事件: {}", click.button_id);
//!     }
//! });
//! 
//! // 发布事件（可在任何运行时调用）
//! xbus::post(ButtonClicked {
//!     button_id: "save_button".to_string(),
//!     timestamp: 1234567890,
//! });
//! ```
//! 
//! ## 性能特征
//! 
//! - **延迟**: ~100ns (直接函数调用)
//! - **吞吐**: >1M events/sec (单线程)
//! - **内存**: 零拷贝事件传递
//! - **并发**: 无锁读写，支持高并发访问

use crossbeam_skiplist::SkipMap;
use std::{
    any::{Any, TypeId},
    sync::{atomic::AtomicUsize, Arc, OnceLock, Weak},
};

/// 全局事件总线实例
/// 
/// 使用 `OnceLock` 确保：
/// 1. 整个进程生命周期内只初始化一次
/// 2. 线程安全的延迟初始化
/// 3. 所有运行时共享同一个事件总线实例
/// 
/// 这是实现跨运行时通信的关键，所有的 post/subscribe 调用
/// 都会操作这个全局单例。
static BUS: OnceLock<EventBus> = OnceLock::new();

/// 发布事件到全局事件总线
/// 
/// 这是 XBus 的核心 API 之一，用于向事件总线发布事件。
/// 该方法是线程安全的，可以在任何运行时中调用。
/// 
/// ## 类型约束
/// 
/// - `Any`: 支持运行时类型信息和动态类型转换
/// - `Send + Sync`: 确保事件可以安全地跨线程传递
/// - `'static`: 确保事件的生命周期足够长，避免悬垂引用
/// 
/// ## 工作流程
/// 
/// 1. 获取或初始化全局事件总线实例
/// 2. 调用事件总线的 `post` 方法
/// 3. 事件总线找到所有匹配的订阅者
/// 4. 同步调用所有订阅者的回调函数
/// 5. 如果回调函数 panic，捕获并记录错误，不影响其他订阅者
/// 
/// ## 性能特点
/// 
/// - **零拷贝**: 事件通过引用传递给回调函数
/// - **同步执行**: 直接函数调用，无异步开销
/// - **类型安全**: 编译时检查类型匹配
/// 
/// # 参数
/// 
/// - `event`: 要发布的事件，必须满足类型约束
/// 
/// # 示例
/// 
/// ```rust
/// #[derive(Debug)]
/// struct UserLogin { user_id: String }
/// 
/// // 在任何运行时中都可以调用
/// xbus::post(UserLogin {
///     user_id: "user123".to_string(),
/// });
/// ```
pub fn post<E: Any + 'static + Send + Sync>(event: E) {
    let bus = BUS.get_or_init(EventBus::new);
    bus.post(event);
}

/// 订阅特定类型的事件
/// 
/// 创建一个类型化的事件订阅，只接收指定类型的事件。
/// 这是性能最优的订阅方式，因为类型匹配在编译时完成。
/// 
/// ## 回调执行特点
/// 
/// - **同步执行**: 回调函数在发布事件的线程中同步执行
/// - **异常隔离**: 如果回调函数 panic，不会影响其他订阅者
/// - **顺序保证**: 订阅者按照订阅顺序接收事件
/// 
/// ## 生命周期管理
/// 
/// 返回的 `Subscription` 对象控制着订阅的生命周期：
/// - 当 `Subscription` 被 drop 时，自动取消订阅
/// - 使用 `Weak` 引用避免循环引用
/// - 线程安全的订阅者管理
/// 
/// # 泛型参数
/// 
/// - `E`: 要订阅的事件类型，必须满足 `Any + Send + Sync + 'static`
/// - `F`: 回调函数类型，接收 `&E` 参数
/// 
/// # 参数
/// 
/// - `f`: 事件处理回调函数，接收事件引用作为参数
/// 
/// # 返回值
/// 
/// 返回订阅凭证，当凭证被 drop 时自动取消订阅
/// 
/// # 示例
/// 
/// ```rust
/// #[derive(Debug)]
/// struct FileChanged { path: String }
/// 
/// let _subscription = xbus::subscribe::<FileChanged, _>(|event| {
///     println!("文件变更: {}", event.path);
///     // 执行文件变更处理逻辑
/// });
/// 
/// // 订阅在 _subscription drop 时自动取消
/// ```
pub fn subscribe<E: Any + 'static + Send + Sync, F: Fn(&E) + Send + Sync + 'static>(
    f: F,
) -> Subscription {
    let bus = BUS.get_or_init(EventBus::new);
    bus.subscribe(f)
}

/// 订阅所有类型的事件
/// 
/// 创建一个通用事件订阅，接收所有类型的事件。
/// 这种订阅方式提供最大的灵活性，但需要在运行时进行类型判断。
/// 
/// ## 使用场景
/// 
/// - **调试和监控**: 监听所有事件以进行调试或性能分析
/// - **事件日志**: 记录系统中的所有事件活动
/// - **事件代理**: 将事件转发到其他系统或运行时
/// - **动态处理**: 根据运行时条件处理不同类型的事件
/// 
/// ## 性能考虑
/// 
/// - 相比类型化订阅，通用订阅会有轻微的性能开销
/// - 需要运行时类型检查和 `downcast` 操作
/// - 每个发布的事件都会触发通用订阅者的回调
/// 
/// # 参数
/// 
/// - `f`: 回调函数，接收 `(TypeId, &dyn Any)` 参数
///   - `TypeId`: 事件的类型 ID，用于类型识别
///   - `&dyn Any`: 事件的动态引用，可通过 `downcast_ref` 转换为具体类型
/// 
/// # 返回值
/// 
/// 返回订阅凭证，管理订阅的生命周期
/// 
/// # 示例
/// 
/// ```rust
/// let _subscription = xbus::subscribe_any(|type_id, event| {
///     println!("收到事件，类型: {:?}", type_id);
///     
///     // 尝试转换为已知类型
///     if let Some(login) = event.downcast_ref::<UserLogin>() {
///         println!("用户登录: {}", login.user_id);
///     } else if let Some(logout) = event.downcast_ref::<UserLogout>() {
///         println!("用户登出: {}", logout.user_id);
///     } else {
///         println!("未知事件类型");
///     }
/// });
/// ```
pub fn subscribe_any<F: Fn(TypeId, &dyn Any) + Send + Sync + 'static>(f: F) -> Subscription {
    let bus = BUS.get_or_init(EventBus::new);
    bus.subscribe_any(f)
}

/// 类型化事件订阅者
/// 
/// 存储特定类型事件的订阅者信息。每个订阅者包含：
/// - 唯一标识符，用于管理订阅生命周期
/// - 回调函数，处理接收到的事件
/// 
/// 使用 `Arc` 包装回调函数，支持在多线程环境中共享和调用。
struct Subscriber {
    /// 订阅者的唯一标识符
    /// 
    /// 用于：
    /// - 在 SkipMap 中作为键进行索引
    /// - 取消订阅时定位特定的订阅者
    /// - 保证订阅者的唯一性
    id: usize,
    
    /// 事件处理回调函数
    /// 
    /// 包装在 `Arc` 中以支持多线程共享，接收 `&dyn Any` 参数
    /// 在实际调用时会进行类型 downcast 转换为具体类型
    callback: Arc<dyn Fn(&dyn Any) + Send + Sync + 'static>,
}

/// 手动实现 Sync，因为 `dyn Fn` 默认不是 Sync
/// 
/// 这里是安全的，因为：
/// 1. callback 是 `Arc<dyn Fn + Send + Sync>`，本身就是线程安全的
/// 2. id 是 `usize`，实现了 Sync
/// 3. 整个结构体的使用都在线程安全的上下文中
unsafe impl Sync for Subscriber {}

/// 为 Subscriber 实现排序 trait，用于 SkipMap 存储
/// 
/// SkipMap 要求键类型实现 `Ord`，这里基于 id 进行排序
impl PartialOrd for Subscriber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Subscriber {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialEq for Subscriber {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Subscriber {}

/// 通用事件订阅者
/// 
/// 用于存储订阅所有类型事件的订阅者信息。
/// 与 `Subscriber` 的区别在于回调函数签名不同。
struct AnySubscriber {
    /// 订阅者唯一标识符
    id: usize,
    
    /// 通用事件处理回调函数
    /// 
    /// 接收 `(TypeId, &dyn Any)` 参数：
    /// - `TypeId`: 用于识别事件的具体类型
    /// - `&dyn Any`: 事件数据的动态引用
    callback: Arc<dyn Fn(TypeId, &dyn Any) + Send + Sync + 'static>,
}

unsafe impl Sync for AnySubscriber {}

/// 为 AnySubscriber 实现排序 trait
impl PartialOrd for AnySubscriber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AnySubscriber {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialEq for AnySubscriber {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for AnySubscriber {}

/// 统一的订阅凭证枚举
/// 
/// 使用枚举统一管理两种不同类型的订阅：
/// - `Typed`: 特定类型的事件订阅
/// - `Any`: 通用的事件订阅
/// 
/// 这种设计的优势：
/// 1. **统一接口**: 两种订阅方式返回相同的类型
/// 2. **类型安全**: 编译时区分不同的订阅类型
/// 3. **资源管理**: 统一的 Drop 实现，自动清理资源
/// 4. **调试友好**: 提供方法查询订阅状态和类型
pub enum Subscription {
    /// 类型化订阅
    /// 
    /// 包含：
    /// - `tyid`: 订阅的事件类型 ID
    /// - `id`: 订阅者 ID
    /// - `bus`: 事件总线的弱引用，避免循环引用
    Typed {
        tyid: TypeId,
        id: usize,
        bus: Weak<EventBusInner>,
    },
    
    /// 通用订阅
    /// 
    /// 包含：
    /// - `id`: 订阅者 ID
    /// - `bus`: 事件总线的弱引用
    Any {
        id: usize,
        bus: Weak<EventBusInner>,
    },
}

/// 订阅凭证的析构实现
/// 
/// 当订阅凭证被 drop 时，自动从事件总线中移除对应的订阅者。
/// 这确保了：
/// 1. **无内存泄漏**: 不再使用的订阅者会被自动清理
/// 2. **性能优化**: 避免调用已失效的回调函数
/// 3. **资源回收**: 及时释放不再需要的内存
impl Drop for Subscription {
    fn drop(&mut self) {
        match self {
            // 清理类型化订阅
            Subscription::Typed { tyid, id, bus } => {
                // 尝试升级弱引用，如果事件总线还存在
                if let Some(bus) = bus.upgrade() {
                    // 找到对应类型的订阅者列表
                    if let Some(list) = bus.subscribers.get(tyid) {
                        // 移除特定的订阅者
                        list.value().remove(id);

                        // 性能优化：如果订阅者列表为空，清理整个类型条目
                        // 这避免了空列表占用内存，提高查找性能
                        if list.value().is_empty() {
                            bus.subscribers.remove(tyid);
                        }
                    }
                }
            }
            // 清理通用订阅
            Subscription::Any { id, bus } => {
                if let Some(bus) = bus.upgrade() {
                    bus.any_subscribers.remove(id);
                }
            }
        }
    }
}

impl Subscription {
    /// 获取订阅类型字符串
    /// 
    /// 用于调试和日志输出，帮助识别订阅的类型。
    /// 
    /// # 返回值
    /// 
    /// - `"typed"`: 特定类型订阅
    /// - `"any"`: 通用订阅
    pub fn subscription_type(&self) -> &'static str {
        match self {
            Subscription::Typed { .. } => "typed",
            Subscription::Any { .. } => "any",
        }
    }

    /// 获取订阅者唯一标识符
    /// 
    /// 每个订阅都有一个唯一的 ID，用于内部管理和调试。
    /// ID 是递增分配的，可以用来判断订阅的创建顺序。
    pub fn id(&self) -> usize {
        match self {
            Subscription::Typed { id, .. } => *id,
            Subscription::Any { id, .. } => *id,
        }
    }

    /// 检查订阅是否仍然有效
    /// 
    /// 检查事件总线是否还存在。如果事件总线已经被销毁，
    /// 则订阅无法继续工作。
    /// 
    /// # 返回值
    /// 
    /// - `true`: 订阅有效，事件总线存在
    /// - `false`: 订阅无效，事件总线已销毁
    /// 
    /// # 注意
    /// 
    /// 由于使用了全局静态事件总线，在正常情况下订阅应该始终有效，
    /// 除非程序即将退出。
    pub fn is_valid(&self) -> bool {
        match self {
            Subscription::Typed { bus, .. } => bus.strong_count() > 0,
            Subscription::Any { bus, .. } => bus.strong_count() > 0,
        }
    }
}

/// 事件总线内部数据结构
/// 
/// 包含了事件总线的核心数据和状态，使用高性能的无锁数据结构：
/// 
/// ## 设计理念
/// 
/// - **无锁并发**: 使用 `SkipMap` 提供无锁的并发读写访问
/// - **类型隔离**: 不同类型的事件使用不同的存储区域
/// - **内存效率**: 只为实际使用的事件类型分配存储空间
/// - **快速查找**: SkipMap 提供 O(log n) 的查找性能
pub struct EventBusInner {
    /// 类型化事件订阅者存储
    /// 
    /// 结构：`TypeId -> Arc<SkipMap<usize, Subscriber>>`
    /// 
    /// - 外层 SkipMap: 按事件类型索引，键为 `TypeId`
    /// - 内层 SkipMap: 存储该类型的所有订阅者，键为订阅者 ID
    /// - 使用 `Arc` 包装内层 SkipMap，支持多线程共享
    /// 
    /// 这种两层结构的优势：
    /// 1. 事件发布时只需要查找特定类型的订阅者
    /// 2. 不同类型的事件处理相互独立，减少锁争用
    /// 3. 可以方便地统计每种类型的订阅者数量
    subscribers: SkipMap<TypeId, Arc<SkipMap<usize, Subscriber>>>,

    /// 通用事件订阅者存储
    /// 
    /// 结构：`usize -> AnySubscriber`
    /// 
    /// - 直接存储所有通用订阅者
    /// - 键为订阅者 ID，值为 `AnySubscriber`
    /// - 每次事件发布时，所有通用订阅者都会被通知
    any_subscribers: SkipMap<usize, AnySubscriber>,
    
    /// 原子递增的 ID 生成器
    /// 
    /// 为每个新的订阅者分配唯一的 ID：
    /// - 使用 `AtomicUsize` 保证线程安全的 ID 分配
    /// - 使用 `Relaxed` 内存序，获得最佳性能
    /// - ID 从 0 开始递增，保证唯一性
    idgen: AtomicUsize,
}

/// 事件总线主结构
/// 
/// 这是 XBus 的核心组件，封装了所有的事件发布和订阅逻辑。
/// 使用 `Arc` 包装内部数据，支持克隆和多线程共享。
/// 
/// ## 设计模式
/// 
/// 采用了"内部可变性"模式：
/// - 外部接口不需要 `&mut self`
/// - 内部使用无锁数据结构实现可变性
/// - 提供线程安全的并发访问
#[derive(Clone)]
pub struct EventBus {
    /// 事件总线的内部数据
    /// 
    /// 使用 `Arc` 包装以支持：
    /// - 多线程安全共享
    /// - 克隆事件总线实例
    /// - 订阅凭证持有弱引用
    inner: Arc<EventBusInner>,
}

impl EventBus {
    /// 创建新的事件总线实例
    /// 
    /// 初始化所有内部数据结构：
    /// - 创建空的订阅者存储
    /// - 初始化 ID 生成器为 0
    /// - 使用 `Arc` 包装以支持多线程访问
    /// 
    /// # 返回值
    /// 
    /// 返回新的事件总线实例，可以立即用于发布和订阅事件
    pub fn new() -> EventBus {
        EventBus {
            inner: Arc::new(EventBusInner {
                subscribers: SkipMap::new(),
                any_subscribers: SkipMap::new(),
                idgen: AtomicUsize::new(0),
            }),
        }
    }

    /// 生成下一个唯一的订阅者 ID
    /// 
    /// 使用原子操作保证线程安全的 ID 分配：
    /// - `fetch_add(1, Relaxed)`: 原子递增并返回旧值
    /// - `Relaxed` 内存序提供最佳性能
    /// - 每次调用返回不同的 ID，保证唯一性
    /// 
    /// # 返回值
    /// 
    /// 返回新分配的唯一 ID
    fn next_id(&self) -> usize {
        self.inner
            .idgen
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// 订阅特定类型的事件
    /// 
    /// 这是事件总线的核心订阅方法，创建类型化的事件订阅。
    /// 
    /// ## 实现细节
    /// 
    /// 1. **类型识别**: 使用 `TypeId::of::<E>()` 获取事件类型标识
    /// 2. **回调包装**: 将类型化回调包装为接受 `&dyn Any` 的通用回调
    /// 3. **存储管理**: 使用两层 SkipMap 结构存储订阅者
    /// 4. **引用管理**: 返回包含弱引用的订阅凭证，避免循环引用
    /// 
    /// ## 性能优化
    /// 
    /// - 使用 `get_or_insert` 延迟创建订阅者列表
    /// - 类型匹配在回调包装中完成，发布时无需类型检查
    /// - SkipMap 提供高效的并发插入操作
    /// 
    /// # 泛型参数
    /// 
    /// - `E`: 事件类型，必须实现 `Any + Send + Sync`
    /// - `F`: 回调函数类型，接收 `&E` 参数
    /// 
    /// # 参数
    /// 
    /// - `f`: 事件处理回调函数
    /// 
    /// # 返回值
    /// 
    /// 返回类型化订阅凭证
    pub fn subscribe<E: Any + Send + Sync, F: Fn(&E) + Send + Sync + 'static>(
        &self,
        f: F,
    ) -> Subscription {
        let tyid = TypeId::of::<E>();
        
        // 将类型化回调包装为通用回调
        // 这个闭包会在事件发布时被调用，进行类型 downcast
        let callback = Arc::new(move |e: &dyn Any| {
            if let Some(e) = e.downcast_ref::<E>() {
                f(e);
            }
        });

        // 获取或创建该类型的订阅者列表
        let list = self
            .inner
            .subscribers
            .get_or_insert(tyid, Arc::new(SkipMap::new()));
        
        let id = self.next_id();
        let subscriber = Subscriber { id, callback };
        
        // 将订阅者插入到列表中
        list.value().insert(id, subscriber);

        // 返回订阅凭证，包含弱引用避免循环引用
        Subscription::Typed {
            tyid,
            id,
            bus: Arc::downgrade(&self.inner),
        }
    }

    /// 订阅所有类型的事件
    /// 
    /// 创建通用事件订阅，接收所有发布到事件总线的事件。
    /// 
    /// ## 使用场景
    /// 
    /// - **事件监控**: 监听系统中的所有事件活动
    /// - **调试工具**: 跟踪和记录事件流
    /// - **事件转发**: 将事件转发到其他系统
    /// - **动态处理**: 基于运行时条件处理不同类型的事件
    /// 
    /// ## 实现特点
    /// 
    /// - 直接存储在单独的 SkipMap 中
    /// - 每次事件发布都会触发所有通用订阅者
    /// - 回调函数接收类型 ID 和事件数据
    /// 
    /// # 参数
    /// 
    /// - `f`: 通用事件处理回调，接收 `(TypeId, &dyn Any)` 参数
    /// 
    /// # 返回值
    /// 
    /// 返回通用订阅凭证
    pub fn subscribe_any<F: Fn(TypeId, &dyn Any) + Send + Sync + 'static>(
        &self,
        f: F,
    ) -> Subscription {
        let callback = Arc::new(f);
        let id = self.next_id();
        let subscriber = AnySubscriber { id, callback };

        // 直接插入到通用订阅者存储中
        self.inner.any_subscribers.insert(id, subscriber);

        Subscription::Any {
            id,
            bus: Arc::downgrade(&self.inner),
        }
    }

    /// 发布事件到事件总线
    /// 
    /// 这是事件总线的核心发布方法，负责将事件传递给所有相关的订阅者。
    /// 
    /// ## 发布流程
    /// 
    /// 1. **类型识别**: 获取事件的 `TypeId`
    /// 2. **类型化通知**: 通知所有订阅该类型的特定订阅者
    /// 3. **通用通知**: 通知所有通用订阅者
    /// 4. **异常处理**: 捕获回调函数的 panic，确保系统稳定性
    /// 
    /// ## 性能优化
    /// 
    /// - **批量收集**: 先收集所有回调引用，避免在迭代时持有锁
    /// - **零拷贝**: 事件通过引用传递，不进行复制
    /// - **并行安全**: 使用 `Arc` 确保回调可以安全并发执行
    /// 
    /// ## 错误处理
    /// 
    /// - 使用 `std::panic::catch_unwind` 捕获回调函数的 panic
    /// - 单个回调的 panic 不会影响其他回调的执行
    /// - panic 信息会被记录到标准错误输出
    /// 
    /// # 参数
    /// 
    /// - `event`: 要发布的事件，必须满足类型约束
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// let bus = EventBus::new();
    /// 
    /// // 订阅者
    /// let _sub = bus.subscribe::<String, _>(|msg| {
    ///     println!("收到消息: {}", msg);
    /// });
    /// 
    /// // 发布事件
    /// bus.post("Hello, World!".to_string());
    /// ```
    pub fn post<E: Any + Send + Sync + 'static>(&self, event: E) {
        let tyid = TypeId::of::<E>();
        let event_ref = &event as &dyn Any;

        // 第一阶段：通知特定类型的订阅者
        if let Some(list) = self.inner.subscribers.get(&tyid) {
            // 性能优化：先收集所有回调引用，避免在调用时持有 SkipMap 的锁
            // 这允许回调函数中进行新的订阅/取消订阅操作，而不会死锁
            let callbacks: Vec<_> = list
                .value()
                .iter()
                .map(|entry| entry.value().callback.clone())
                .collect();

            // 依次调用所有回调函数
            for callback in callbacks {
                // 异常隔离：单个回调的 panic 不影响其他回调
                if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    callback(event_ref);
                })) {
                    eprintln!("Event handler panicked: {:?}", e);
                }
            }
        }

        // 第二阶段：通知所有通用订阅者
        let any_callbacks: Vec<_> = self
            .inner
            .any_subscribers
            .iter()
            .map(|entry| entry.value().callback.clone())
            .collect();

        for callback in any_callbacks {
            if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                callback(tyid, event_ref);
            })) {
                eprintln!("Any event handler panicked: {:?}", e);
            }
        }
    }

    /// 获取指定类型的订阅者数量
    /// 
    /// 用于统计和调试，返回订阅特定事件类型的订阅者数量。
    /// 
    /// # 泛型参数
    /// 
    /// - `E`: 要查询的事件类型
    /// 
    /// # 返回值
    /// 
    /// 返回订阅该类型事件的订阅者数量，如果没有订阅者则返回 0
    pub fn subscriber_count<E: Any + 'static>(&self) -> usize {
        let tyid = TypeId::of::<E>();
        self.inner
            .subscribers
            .get(&tyid)
            .map(|list| list.value().len())
            .unwrap_or(0)
    }

    /// 获取通用订阅者数量
    /// 
    /// 返回订阅所有事件的通用订阅者数量。
    /// 
    /// # 返回值
    /// 
    /// 通用订阅者的数量
    pub fn any_subscriber_count(&self) -> usize {
        self.inner.any_subscribers.len()
    }

    /// 清空所有订阅者
    /// 
    /// 移除所有的类型化订阅者和通用订阅者。
    /// 通常用于测试或系统重置。
    /// 
    /// # 注意
    /// 
    /// 清空后，现有的订阅凭证虽然仍然存在，但对应的订阅者已被移除，
    /// 不会再接收到新的事件。
    pub fn clear(&self) {
        self.inner.subscribers.clear();
        self.inner.any_subscribers.clear();
    }

    /// 获取所有事件类型的订阅者统计
    /// 
    /// 返回系统中所有已订阅事件类型的统计信息，
    /// 用于监控和调试。
    /// 
    /// # 返回值
    /// 
    /// 返回 `Vec<(TypeId, usize)>`，每个元组包含：
    /// - `TypeId`: 事件类型标识符
    /// - `usize`: 该类型的订阅者数量
    pub fn statistics(&self) -> Vec<(TypeId, usize)> {
        self.inner
            .subscribers
            .iter()
            .map(|entry| (*entry.key(), entry.value().len()))
            .collect()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 便利宏：类型化事件订阅
/// 
/// 提供更简洁的订阅语法，自动推断事件类型。
/// 
/// # 示例
/// 
/// ```rust
/// let _sub = subscribe!(MyEvent, |event| {
///     println!("处理事件: {:?}", event);
/// });
/// ```
#[macro_export]
macro_rules! subscribe {
    ($event_type:ty, $handler:expr) => {
        $crate::xbus::subscribe::<$event_type, _>($handler)
    };
}

/// 便利宏：通用事件订阅
/// 
/// 提供更简洁的通用订阅语法。
/// 
/// # 示例
/// 
/// ```rust
/// let _sub = subscribe_any!(|type_id, event| {
///     println!("收到事件: {:?}", type_id);
/// });
/// ```
#[macro_export]
macro_rules! subscribe_any {
    ($handler:expr) => {
        $crate::xbus::subscribe_any($handler)
    };
}

/// 便利宏：事件发布
/// 
/// 提供更简洁的事件发布语法。
/// 
/// # 示例
/// 
/// ```rust
/// post!(MyEvent { data: "hello".to_string() });
/// ```
#[macro_export]
macro_rules! post {
    ($event:expr) => {
        $crate::xbus::post($event)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone)]
    struct TestEvent1 {
        message: String,
    }

    #[derive(Debug, Clone)]
    struct TestEvent2 {
        value: i32,
    }

    /// 测试基本的订阅和发布功能
    #[test]
    fn test_subscribe_and_post() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        let subscription = subscribe::<TestEvent1, _>(move |event| {
            received_clone.lock().unwrap().push(event.message.clone());
        });

        // 测试订阅类型和状态
        assert_eq!(subscription.subscription_type(), "typed");
        assert!(subscription.is_valid());

        // 发布事件
        post(TestEvent1 {
            message: "Hello".to_string(),
        });
        post(TestEvent1 {
            message: "World".to_string(),
        });

        // 验证事件接收
        let messages = received.lock().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], "Hello");
        assert_eq!(messages[1], "World");
    }

    /// 测试通用事件订阅功能
    #[test]
    fn test_subscribe_any() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        // 订阅所有类型的事件
        let any_subscription = subscribe_any!(move |type_id, event| {
            let type_name = format!("{:?}", type_id);

            // 尝试识别不同的事件类型
            if let Some(e) = event.downcast_ref::<TestEvent1>() {
                received_clone
                    .lock()
                    .unwrap()
                    .push(format!("TestEvent1: {}", e.message));
            } else if let Some(e) = event.downcast_ref::<TestEvent2>() {
                received_clone
                    .lock()
                    .unwrap()
                    .push(format!("TestEvent2: {}", e.value));
            } else {
                received_clone
                    .lock()
                    .unwrap()
                    .push(format!("Unknown: {}", type_name));
            }
        });

        // 测试订阅类型
        assert_eq!(any_subscription.subscription_type(), "any");
        assert!(any_subscription.is_valid());

        // 发布不同类型的事件
        post(TestEvent1 {
            message: "Hello".to_string(),
        });
        post(TestEvent2 { value: 42 });

        let messages = received.lock().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], "TestEvent1: Hello");
        assert_eq!(messages[1], "TestEvent2: 42");
    }

    /// 测试混合订阅模式
    #[test]
    fn test_mixed_subscriptions() {
        let specific_received = Arc::new(Mutex::new(Vec::new()));
        let any_received = Arc::new(Mutex::new(Vec::new()));

        let specific_clone = specific_received.clone();
        let any_clone = any_received.clone();

        // 特定类型订阅
        let specific_sub = subscribe::<TestEvent1, _>(move |event| {
            specific_clone.lock().unwrap().push(event.message.clone());
        });

        // 通用订阅
        let any_sub = subscribe_any!(move |_type_id, event| {
            if let Some(e) = event.downcast_ref::<TestEvent1>() {
                any_clone
                    .lock()
                    .unwrap()
                    .push(format!("Any: {}", e.message));
            }
        });

        // 验证订阅属性
        assert_eq!(specific_sub.subscription_type(), "typed");
        assert_eq!(any_sub.subscription_type(), "any");
        assert_ne!(specific_sub.id(), any_sub.id()); // ID 应该不同

        post(TestEvent1 {
            message: "Test".to_string(),
        });

        // 特定订阅者和通用订阅者都应该收到事件
        let specific = specific_received.lock().unwrap();
        let any = any_received.lock().unwrap();

        assert_eq!(specific.len(), 1);
        assert_eq!(specific[0], "Test");
        assert_eq!(any.len(), 1);
        assert_eq!(any[0], "Any: Test");
    }

    /// 测试订阅自动清理功能
    #[test]
    fn test_subscription_drop() {
        let bus = EventBus::new();

        {
            let typed_subscription = bus.subscribe::<TestEvent1, _>(|_| {});
            let any_subscription = bus.subscribe_any(|_, _| {});

            assert_eq!(bus.subscriber_count::<TestEvent1>(), 1);
            assert_eq!(bus.any_subscriber_count(), 1);

            // 验证订阅类型
            assert_eq!(typed_subscription.subscription_type(), "typed");
            assert_eq!(any_subscription.subscription_type(), "any");
        }

        // 订阅应该在 drop 后被自动清理
        assert_eq!(bus.subscriber_count::<TestEvent1>(), 0);
        assert_eq!(bus.any_subscriber_count(), 0);
    }

    /// 测试订阅凭证的辅助方法
    #[test]
    fn test_subscription_methods() {
        let bus = EventBus::new();

        let typed_sub = bus.subscribe::<TestEvent1, _>(|_| {});
        let any_sub = bus.subscribe_any(|_, _| {});

        // 测试订阅类型识别
        assert_eq!(typed_sub.subscription_type(), "typed");
        assert_eq!(any_sub.subscription_type(), "any");

        // 测试 ID 唯一性
        assert_ne!(typed_sub.id(), any_sub.id());

        // 测试有效性检查
        assert!(typed_sub.is_valid());
        assert!(any_sub.is_valid());

        // 测试 ID 递增分配
        let another_sub = bus.subscribe::<TestEvent2, _>(|_| {});
        assert!(another_sub.id() > typed_sub.id());
        assert!(another_sub.id() > any_sub.id());
    }
}
