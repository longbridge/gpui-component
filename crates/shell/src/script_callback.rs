//! Routes callbacks through GPUI APIs that require `Send + Sync` closures.

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Weak,
    sync::Arc,
};

use gpui::{App, Window};

use crate::{HostValue, engine::ShellRuntime, spec::CallbackId};

#[derive(Clone)]
pub(crate) struct ScriptCallbackRoute(Arc<RouteId>);

struct RouteId(u64);

struct RouteTarget {
    runtime: Weak<ShellRuntime>,
    callback: CallbackId,
}

thread_local! {
    static NEXT_ROUTE: Cell<u64> = const { Cell::new(1) };
    static ROUTES: RefCell<BTreeMap<u64, RouteTarget>> = const { RefCell::new(BTreeMap::new()) };
}

impl Drop for RouteId {
    fn drop(&mut self) {
        ROUTES.with_borrow_mut(|routes| {
            routes.remove(&self.0);
        });
    }
}

impl ScriptCallbackRoute {
    pub(crate) fn new(runtime: Weak<ShellRuntime>, callback: CallbackId) -> Self {
        let id = NEXT_ROUTE.with(|next| {
            let id = next.get();
            next.set(
                id.checked_add(1)
                    .expect("script callback route id space exhausted"),
            );
            id
        });
        ROUTES.with_borrow_mut(|routes| {
            routes.insert(id, RouteTarget { runtime, callback });
        });
        Self(Arc::new(RouteId(id)))
    }

    pub(crate) fn emit(&self, payload: HostValue, window: &mut Window, cx: &mut App) {
        let target = ROUTES.with_borrow(|routes| {
            routes
                .get(&self.0.0)
                .map(|target| (target.runtime.clone(), target.callback))
        });
        let Some((runtime, callback)) = target else {
            return;
        };
        if let Some(runtime) = runtime.upgrade() {
            runtime.dispatch_host_event(callback, payload, window, cx);
        }
    }
}
