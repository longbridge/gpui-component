//! Rust elements exported by a host module for scripts to place as opaque leaves.

use std::{fmt, rc::Rc};

use gpui::{AnyElement, App, SharedString, Window};

use crate::HostValue;

type ComponentBuilder = Rc<dyn for<'a> Fn(ComponentArgs<'a>, &mut Window, &mut App) -> AnyElement>;

/// Builds one Rust element exported by a [`crate::HostModule`].
#[derive(Clone)]
pub(crate) struct ComponentFactory {
    build: ComponentBuilder,
}

impl ComponentFactory {
    pub(crate) fn new(
        build: impl for<'a> Fn(ComponentArgs<'a>, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            build: Rc::new(build),
        }
    }

    pub(crate) fn build(
        &self,
        args: ComponentArgs<'_>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        (self.build)(args, window, cx)
    }
}

impl fmt::Debug for ComponentFactory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComponentFactory")
            .finish_non_exhaustive()
    }
}

impl PartialEq for ComponentFactory {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.build, &other.build)
    }
}

/// Inputs supplied while one module component is materialized.
pub struct ComponentArgs<'a> {
    pub(crate) id: &'a SharedString,
    pub(crate) props: &'a HostValue,
    pub(crate) children: Vec<AnyElement>,
}

impl ComponentArgs<'_> {
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
