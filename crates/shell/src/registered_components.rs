//! Rust elements exported by a host module for scripts to place as opaque leaves.

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    fmt,
    rc::{Rc, Weak},
    sync::Arc,
};

use gpui::{AnyElement, App, SharedString, Window};

use crate::{HostValue, engine::ShellRuntime, spec::CallbackId};

type ComponentBuilder =
    Rc<dyn for<'a> Fn(RegisteredComponentArgs<'a>, &mut Window, &mut App) -> AnyElement>;

/// One Rust-built element constructor exported from a [`crate::HostModule`].
#[derive(Clone)]
pub struct RegisteredComponent {
    name: String,
    build: ComponentBuilder,
}

impl RegisteredComponent {
    pub fn new(
        name: impl Into<String>,
        build: impl for<'a> Fn(RegisteredComponentArgs<'a>, &mut Window, &mut App) -> AnyElement
        + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            build: Rc::new(build),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn build(
        &self,
        args: RegisteredComponentArgs<'_>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        (self.build)(args, window, cx)
    }
}

impl fmt::Debug for RegisteredComponent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RegisteredComponent")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl PartialEq for RegisteredComponent {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && Rc::ptr_eq(&self.build, &other.build)
    }
}

/// Inputs supplied while one registered component is materialized.
pub struct RegisteredComponentArgs<'a> {
    pub(crate) id: &'a SharedString,
    pub(crate) props: &'a HostValue,
    pub(crate) children: Vec<AnyElement>,
}

impl RegisteredComponentArgs<'_> {
    pub fn id(&self) -> &str {
        self.id
    }

    pub fn props(&self) -> &HostValue {
        self.props
    }

    pub fn children(&self) -> &[AnyElement] {
        &self.children
    }

    pub fn take_children(&mut self) -> Vec<AnyElement> {
        std::mem::take(&mut self.children)
    }
}

/// Send + Sync token used by callbacks whose GPUI API requires those bounds.
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
