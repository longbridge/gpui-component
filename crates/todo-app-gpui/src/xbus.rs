use crossbeam_skiplist::SkipMap;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    sync::{atomic::AtomicUsize, Arc, OnceLock},
};

static BUS: OnceLock<EventBus> = OnceLock::new();

pub fn post<E: Any + 'static + Debug>(event: &E) {
    let bus = BUS.get_or_init(|| EventBus::new());
    bus.post(event);
}

pub fn subscribe<'a, E: Any + 'static + Debug, F: Fn(&E) + Send + 'static>(f: F) -> Subscription {
    let bus = BUS.get_or_init(|| EventBus::new());
    bus.subscribe(f)
}

struct Subscriber {
    id: usize,
    callback: Box<dyn Fn(&dyn Any) + Send + 'static>,
}

//TODO .............
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
    bus: EventBus,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(list) = self.bus.subscribers.get(&self.tyid) {
            list.value().remove(&self.id);
        }
    }
}

#[derive(Clone)]
pub struct EventBus {
    subscribers: Arc<SkipMap<TypeId, Arc<SkipMap<usize, Subscriber>>>>,
    idgen: Arc<AtomicUsize>,
}

impl EventBus {
    pub fn new() -> EventBus {
        EventBus {
            subscribers: Arc::new(SkipMap::new()),
            idgen: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn next_id(&self) -> usize {
        self.idgen
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

impl EventBus {
    pub fn subscribe<E: Any, F: Fn(&E) + Send + 'static>(&self, f: F) -> Subscription {
        let tyid = TypeId::of::<E>();
        let value_any = Box::new(move |e: &dyn Any| {
            if let Some(e) = e.downcast_ref::<E>() {
                f(e);
            }
        });
        let list = self
            .subscribers
            .get_or_insert(tyid, Arc::new(SkipMap::new()));
        let id = self.next_id();
        let subscriber = Subscriber {
            id,
            callback: value_any,
        };
        list.value().insert(id, subscriber);
        Subscription {
            tyid,
            id,
            bus: self.clone(),
        }
    }

    pub fn post<E: Any + Debug + 'static>(&self, event: &E) {
        let value_any = event as &dyn Any;
        let tyid = TypeId::of::<E>();
        if let Some(list) = self.subscribers.get(&tyid) {
            for subscriber in list.value().iter() {
                (subscriber.value().callback)(value_any);
            }
        }
    }
}
