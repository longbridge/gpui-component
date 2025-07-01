use std::{
    any::{Any, TypeId},
    fmt::Debug,
    sync::{Arc, OnceLock},
};
use tokio::sync::broadcast;

static EBUS: OnceLock<EventBus> = OnceLock::new();

// 公共API - 直接使用 TypedEvent
pub fn post<E: Clone + Send + Sync + 'static>(
    event: E,
) -> Result<(), broadcast::error::SendError<TypedEvent>> {
    let bus = EBUS.get_or_init(EventBus::new);
    bus.post(event)
}

pub fn subscribe<E: Clone + Send + Sync + 'static>() -> TypedReceiver<E> {
    let bus = EBUS.get_or_init(EventBus::new);
    bus.subscribe()
}

// 简化的事件类型 - 直接作为广播类型
#[derive(Clone)]
pub struct TypedEvent {
    type_id: TypeId,
    data: Arc<dyn Any + Send + Sync>,
}

impl TypedEvent {
    fn new<T: Clone + Send + Sync + 'static>(data: T) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            data: Arc::new(data),
        }
    }

    fn downcast<T: 'static + Clone>(&self) -> Option<T> {
        if self.type_id == TypeId::of::<T>() {
            self.data.downcast_ref::<T>().cloned()
        } else {
            None
        }
    }
}

impl std::fmt::Debug for TypedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedEvent")
            .field("type_id", &self.type_id)
            .finish_non_exhaustive()
    }
}

// 类型化接收器 - 直接处理 TypedEvent
pub struct TypedReceiver<T> {
    receiver: broadcast::Receiver<TypedEvent>,
    target_type_id: TypeId,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Clone + Send + Sync + 'static> TypedReceiver<T> {
    fn new(receiver: broadcast::Receiver<TypedEvent>) -> Self {
        Self {
            receiver,
            target_type_id: TypeId::of::<T>(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// 接收下一个匹配类型的事件
    pub async fn recv(&mut self) -> Result<T, broadcast::error::RecvError> {
        loop {
            match self.receiver.recv().await {
                Ok(typed_event) => {
                    if typed_event.type_id == self.target_type_id {
                        if let Some(data) = typed_event.downcast::<T>() {
                            return Ok(data);
                        }
                    }
                    // 继续循环等待匹配的事件
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// 尝试接收下一个匹配类型的事件（非阻塞）
    pub fn try_recv(&mut self) -> Result<T, broadcast::error::TryRecvError> {
        loop {
            match self.receiver.try_recv() {
                Ok(typed_event) => {
                    if typed_event.type_id == self.target_type_id {
                        if let Some(data) = typed_event.downcast::<T>() {
                            return Ok(data);
                        }
                    }
                    // 继续循环等待匹配的事件
                }
                Err(broadcast::error::TryRecvError::Empty) => {
                    return Err(broadcast::error::TryRecvError::Empty);
                }
                Err(e) => return Err(e),
            }
        }
    }
}

// 原始接收器 - 直接接收 TypedEvent
pub struct RawReceiver {
    receiver: broadcast::Receiver<TypedEvent>,
}

impl RawReceiver {
    pub async fn recv(&mut self) -> Result<TypedEvent, broadcast::error::RecvError> {
        self.receiver.recv().await
    }

    pub fn try_recv(&mut self) -> Result<TypedEvent, broadcast::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

// 事件总线主结构 - 直接使用 TypedEvent
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<TypedEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(10000);
        Self { sender }
    }

    // 发布事件
    pub fn post<E: Clone + Send + Sync + 'static>(
        &self,
        event: E,
    ) -> Result<(), broadcast::error::SendError<TypedEvent>> {
        let typed_event = TypedEvent::new(event);
        self.sender.send(typed_event).map(|_| ())
    }

    // 强类型订阅
    pub fn subscribe<E: Clone + Send + Sync + 'static>(&self) -> TypedReceiver<E> {
        TypedReceiver::new(self.sender.subscribe())
    }

    // 原始订阅（接收所有事件）
    pub fn subscribe_raw(&self) -> RawReceiver {
        RawReceiver {
            receiver: self.sender.subscribe(),
        }
    }

    // 获取原始广播发送器
    pub fn raw_sender(&self) -> broadcast::Sender<TypedEvent> {
        self.sender.clone()
    }

    // 统计信息
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// 便利宏
#[macro_export]
macro_rules! epost {
    ($event:expr) => {
        $crate::ebus::post($event).unwrap_or_else(|e| {
            log::error!("Failed to post event: {}", e);
        })
    };
}

// 初始化函数
pub fn init_event_bus() {
    let _ = EBUS.get_or_init(EventBus::new);
    log::info!("Enhanced event bus initialized");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::time::{sleep, Duration};

    // 为每个测试使用不同的事件类型
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
        nested: Box<String>, // 简化嵌套结构
    }

    #[tokio::test]
    async fn test_typed_events() {
        init_event_bus();

        let mut receiver = subscribe::<TestEvent1>();

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

        let event1 = receiver.recv().await.unwrap();
        assert_eq!(event1.message, "Hello");

        let event2 = receiver.recv().await.unwrap();
        assert_eq!(event2.message, "World");
    }

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
