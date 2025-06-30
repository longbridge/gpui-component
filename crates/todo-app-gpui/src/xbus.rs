use crossbeam_skiplist::SkipMap;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    sync::{atomic::AtomicUsize, Arc, OnceLock, Weak},
};

static BUS: OnceLock<EventBus> = OnceLock::new();

pub fn post<E: Any + 'static + Debug + Send + Sync>(event: E) {
    let bus = BUS.get_or_init(EventBus::new);
    bus.post(event);
}

pub fn subscribe<E: Any + 'static + Debug + Send + Sync, F: Fn(&E) + Send + Sync + 'static>(
    f: F,
) -> Subscription {
    let bus = BUS.get_or_init(EventBus::new);
    bus.subscribe(f)
}

struct Subscriber {
    id: usize,
    callback: Arc<dyn Fn(&dyn Any) + Send + Sync + 'static>,
}

// 现在可以安全地实现 Sync，因为 callback 是 Arc<dyn Fn + Send + Sync>
unsafe impl Sync for Subscriber {}

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

pub struct Subscription {
    tyid: TypeId,
    id: usize,
    bus: Weak<EventBusInner>, // 使用 Weak 引用避免循环引用
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(bus) = self.bus.upgrade() {
            if let Some(list) = bus.subscribers.get(&self.tyid) {
                list.value().remove(&self.id);

                // 如果订阅者列表为空，清理 TypeId 条目
                if list.value().is_empty() {
                    bus.subscribers.remove(&self.tyid);
                }
            }
        }
    }
}

struct EventBusInner {
    subscribers: SkipMap<TypeId, Arc<SkipMap<usize, Subscriber>>>,

    idgen: AtomicUsize,
}

#[derive(Clone)]
pub struct EventBus {
    inner: Arc<EventBusInner>,
}

impl EventBus {
    pub fn new() -> EventBus {
        EventBus {
            inner: Arc::new(EventBusInner {
                subscribers: SkipMap::new(),
                idgen: AtomicUsize::new(0),
            }),
        }
    }

    fn next_id(&self) -> usize {
        self.inner
            .idgen
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn subscribe<E: Any + Send + Sync, F: Fn(&E) + Send + Sync + 'static>(
        &self,
        f: F,
    ) -> Subscription {
        let tyid = TypeId::of::<E>();
        let callback = Arc::new(move |e: &dyn Any| {
            if let Some(e) = e.downcast_ref::<E>() {
                f(e);
            }
        });

        let list = self
            .inner
            .subscribers
            .get_or_insert(tyid, Arc::new(SkipMap::new()));
        let id = self.next_id();
        let subscriber = Subscriber { id, callback };
        list.value().insert(id, subscriber);

        Subscription {
            tyid,
            id,
            bus: Arc::downgrade(&self.inner),
        }
    }

    pub fn post<E: Any + Debug + Send + Sync + 'static>(&self, event: E) {
        let tyid = TypeId::of::<E>();
        if let Some(list) = self.inner.subscribers.get(&tyid) {
            let event_ref = &event as &dyn Any;

            // 收集所有回调以避免在迭代时持有锁
            let callbacks: Vec<_> = list
                .value()
                .iter()
                .map(|entry| entry.value().callback.clone())
                .collect();

            // 执行回调
            for callback in callbacks {
                if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    callback(event_ref);
                })) {
                    eprintln!("Event handler panicked: {:?}", e);
                }
            }
        }
    }

    /// 获取指定类型的订阅者数量
    pub fn subscriber_count<E: Any + 'static>(&self) -> usize {
        let tyid = TypeId::of::<E>();
        self.inner
            .subscribers
            .get(&tyid)
            .map(|list| list.value().len())
            .unwrap_or(0)
    }

    /// 清空所有订阅者
    pub fn clear(&self) {
        self.inner.subscribers.clear();
    }

    /// 获取所有事件类型的订阅者统计
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

// 可选：添加一些便利的宏
#[macro_export]
macro_rules! subscribe {
    ($event_type:ty, $handler:expr) => {
        $crate::xbus::subscribe::<$event_type, _>($handler)
    };
}

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
    struct TestEvent {
        message: String,
    }

    #[test]
    fn test_subscribe_and_post() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        let _subscription = subscribe::<TestEvent, _>(move |event| {
            received_clone.lock().unwrap().push(event.message.clone());
        });

        post(TestEvent {
            message: "Hello".to_string(),
        });
        post(TestEvent {
            message: "World".to_string(),
        });

        let messages = received.lock().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], "Hello");
        assert_eq!(messages[1], "World");
    }

    #[test]
    fn test_subscription_drop() {
        let bus = EventBus::new();

        {
            let _subscription = bus.subscribe::<TestEvent, _>(|_| {});
            assert_eq!(bus.subscriber_count::<TestEvent>(), 1);
        }

        // 订阅应该在 drop 后被清理
        assert_eq!(bus.subscriber_count::<TestEvent>(), 0);
    }
}
