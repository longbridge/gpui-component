//! Host-built elements that scripts may place in their element trees.

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use gpui::{AnyElement, App, SharedString, Window};

use crate::{
    engine::ShellRuntime,
    host_modules::{HostError, HostValue},
    spec::CallbackId,
};

type HostComponentBuilder =
    Rc<dyn for<'a> Fn(HostComponentArgs<'a>, &mut Window, &mut App) -> AnyElement>;

/// An element built by the Rust host and placed by script.
#[derive(Clone)]
pub struct HostComponent {
    name: String,
    declarations: Option<String>,
    build: HostComponentBuilder,
}

impl HostComponent {
    pub fn new(
        name: &str,
        build: impl for<'a> Fn(HostComponentArgs<'a>, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            name: name.to_owned(),
            declarations: None,
            build: Rc::new(build),
        }
    }

    /// Adds TypeScript declarations emitted into `gpui.d.ts` for this component's props.
    pub fn declarations(mut self, typescript: &str) -> Self {
        self.declarations = Some(typescript.to_owned());
        self
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }
    pub(crate) fn declared(&self) -> Option<&str> {
        self.declarations.as_deref()
    }
    pub(crate) fn build(
        &self,
        args: HostComponentArgs<'_>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        (self.build)(args, window, cx)
    }
}

/// Values supplied while one host component is materialized.
pub struct HostComponentArgs<'a> {
    pub id: &'a SharedString,
    pub props: &'a HostValue,
    pub children: Vec<AnyElement>,
    pub events: HostComponentEvents,
}

/// A cloneable route from a host component callback to script `.on(name, handler)` handlers.
#[derive(Clone)]
pub struct HostComponentEvents {
    route: Arc<EventRoute>,
}

struct EventRoute {
    id: u64,
}

struct EventTarget {
    runtime: Weak<ShellRuntime>,
    handlers: Vec<(SharedString, CallbackId)>,
}

impl Drop for EventRoute {
    fn drop(&mut self) {
        EVENT_TARGETS.with_borrow_mut(|targets| {
            targets.remove(&self.id);
        });
    }
}

thread_local! {
    static NEXT_EVENT_ROUTE: Cell<u64> = const { Cell::new(1) };
    static EVENT_TARGETS: RefCell<BTreeMap<u64, EventTarget>> = const { RefCell::new(BTreeMap::new()) };
}

impl HostComponentEvents {
    pub(crate) fn new(
        runtime: Weak<ShellRuntime>,
        handlers: Vec<(SharedString, CallbackId)>,
    ) -> Self {
        let id = NEXT_EVENT_ROUTE.with(|next| {
            let id = next.get();
            next.set(id.wrapping_add(1));
            id
        });
        EVENT_TARGETS.with_borrow_mut(|targets| {
            targets.insert(id, EventTarget { runtime, handlers });
        });
        Self {
            route: Arc::new(EventRoute { id }),
        }
    }

    pub fn emit(&self, event: &str, payload: HostValue, window: &mut Window, cx: &mut App) {
        EVENT_TARGETS.with_borrow(|targets| {
            let Some(target) = targets.get(&self.route.id) else {
                return;
            };
            let Some(runtime) = target.runtime.upgrade() else {
                return;
            };
            for (_, callback) in target
                .handlers
                .iter()
                .filter(|(name, _)| name.as_ref() == event)
            {
                runtime.dispatch_host_event(*callback, payload.clone(), window, cx);
            }
        });
    }
}

thread_local! {
    static COMPONENTS: RefCell<BTreeMap<String, HostComponent>> = const { RefCell::new(BTreeMap::new()) };
}

pub(crate) fn add(component: HostComponent) -> Result<(), HostError> {
    let name = component.name().to_owned();
    if crate::spec::Component::is_builtin_name(&name) {
        return Err(HostError::new(format!(
            "HostComponent `{name}` conflicts with built-in component `{name}`"
        )));
    }
    COMPONENTS.with_borrow_mut(|components| {
        if components.contains_key(&name) {
            return Err(HostError::new(format!(
                "HostComponent `{name}` is already registered"
            )));
        }
        components.insert(name, component);
        Ok(())
    })
}

pub(crate) fn get(name: &str) -> Option<HostComponent> {
    COMPONENTS.with_borrow(|components| components.get(name).cloned())
}

pub(crate) fn all() -> Vec<HostComponent> {
    COMPONENTS.with_borrow(|components| components.values().cloned().collect())
}

pub(crate) fn clear() {
    COMPONENTS.with_borrow_mut(BTreeMap::clear);
}
