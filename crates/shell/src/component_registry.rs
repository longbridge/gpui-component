use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt,
    marker::PhantomData,
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub(crate) const MAX_RETAINED_COMPONENT_STATES: usize = 4096;

#[derive(Default)]
pub(crate) struct RetainedStateStore {
    next_handle: u64,
    values: HashMap<u64, RetainedStateEntry>,
}

struct RetainedStateEntry {
    kind: &'static str,
    owner: Option<Rc<crate::runtime::ApplicationGeneration>>,
    value: RetainedState,
}

impl RetainedStateStore {
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn insert(
        &mut self,
        kind: &'static str,
        owner: Option<Rc<crate::runtime::ApplicationGeneration>>,
        value: RetainedState,
    ) -> Result<u64, String> {
        if owner.as_ref().is_some_and(|owner| !owner.is_active()) {
            return Err("retained component state owner has already been released".into());
        }
        if self.values.len() >= MAX_RETAINED_COMPONENT_STATES {
            return Err(format!(
                "retained component state limit ({MAX_RETAINED_COMPONENT_STATES}) reached"
            ));
        }
        let handle = self.next_handle;
        self.next_handle = self
            .next_handle
            .checked_add(1)
            .ok_or_else(|| "retained component state handle space exhausted".to_owned())?;
        self.values
            .insert(handle, RetainedStateEntry { kind, owner, value });
        Ok(handle)
    }

    pub fn kind(&self, handle: u64) -> Option<&'static str> {
        self.values.get(&handle).and_then(|entry| {
            entry
                .owner
                .as_ref()
                .is_none_or(|owner| owner.is_active())
                .then_some(entry.kind)
        })
    }

    pub fn with<T: Any, R>(
        &self,
        handle: u64,
        kind: &'static str,
        body: impl FnOnce(&T) -> R,
    ) -> anyhow::Result<R> {
        let entry = self
            .values
            .get(&handle)
            .ok_or_else(|| anyhow::anyhow!("retained state handle has been released"))?;
        anyhow::ensure!(
            entry.owner.as_ref().is_none_or(|owner| owner.is_active()),
            "retained state owner has been released"
        );
        anyhow::ensure!(
            entry.kind == kind,
            "retained state kind mismatch: expected `{kind}`, found `{}`",
            entry.kind
        );
        let value = entry.value.downcast_ref::<T>().ok_or_else(|| {
            anyhow::anyhow!("adapter state type does not match registered kind `{kind}`")
        })?;
        Ok(body(value))
    }

    pub fn with_mut<T: Any, R>(
        &mut self,
        handle: u64,
        kind: &'static str,
        body: impl FnOnce(&mut T) -> R,
    ) -> anyhow::Result<R> {
        let entry = self
            .values
            .get_mut(&handle)
            .ok_or_else(|| anyhow::anyhow!("retained state handle has been released"))?;
        anyhow::ensure!(
            entry.owner.as_ref().is_none_or(|owner| owner.is_active()),
            "retained state owner has been released"
        );
        anyhow::ensure!(
            entry.kind == kind,
            "retained state kind mismatch: expected `{kind}`, found `{}`",
            entry.kind
        );
        let value = entry.value.downcast_mut::<T>().ok_or_else(|| {
            anyhow::anyhow!("adapter state type does not match registered kind `{kind}`")
        })?;
        Ok(body(value))
    }

    pub fn release_application(&mut self, application: &Rc<crate::runtime::ApplicationGeneration>) {
        self.values.retain(|_, entry| {
            !entry
                .owner
                .as_ref()
                .is_some_and(|owner| Rc::ptr_eq(owner, application))
        });
    }
}

use gpui::{
    AnyElement, App, ClickEvent, IntoElement, ParentElement, Refineable as _, StyleRefinement,
    Styled, Window,
};

pub const COMPONENT_REGISTRY_API_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ComponentId(u32);

impl ComponentId {
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Clone)]
pub struct ComponentPayload(Arc<dyn Any + Send + Sync>);

impl ComponentPayload {
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self(Arc::new(value))
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }
}

impl fmt::Debug for ComponentPayload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("ComponentPayload").finish()
    }
}

impl PartialEq for ComponentPayload {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ComponentArgument {
    String(String),
    Number(f64),
    Boolean(bool),
    Element(u32),
    Entity { kind: &'static str, handle: u64 },
    Callback(u64),
    Enum(String),
    Array(Vec<ComponentArgument>),
    Optional(Option<Box<ComponentArgument>>),
}

/// A value an adapter may pass back to a JavaScript component callback.
///
/// This intentionally stays closed: opaque shell handles and retained element
/// descriptions never cross back into the script event boundary.
#[derive(Clone, Debug, PartialEq)]
pub enum ComponentCallbackArgument {
    String(String),
    Number(f64),
    Boolean(bool),
}

/// One child description owned by a single [`MaterializeRequest`].
///
/// The token is deliberately opaque. It can only be consumed by the request
/// that issued it, and only registered component identities are exposed.
pub struct ComponentChild<'request> {
    runtime: Weak<crate::ShellRuntime>,
    request_id: u64,
    token: u64,
    component_name: Option<&'static str>,
    request_lifetime: PhantomData<&'request ()>,
}

impl ComponentChild<'_> {
    pub fn component_name(&self) -> Option<&'static str> {
        self.component_name
    }
}

static NEXT_MATERIALIZE_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChildLane {
    Unclaimed,
    Ordinary,
    Typed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArgumentSchema {
    String,
    Number,
    Boolean,
    Element,
    Entity(&'static str),
    Callback(&'static str),
    Enum(&'static [&'static str]),
    Array(Box<ArgumentSchema>),
    Optional(Box<ArgumentSchema>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgumentDescriptor {
    pub name: &'static str,
    pub schema: ArgumentSchema,
}

impl ArgumentDescriptor {
    pub const fn new(name: &'static str, schema: ArgumentSchema) -> Self {
        Self { name, schema }
    }
}

type PayloadFactory =
    dyn Fn(&[ComponentArgument]) -> Result<ComponentPayload, String> + Send + Sync + 'static;

/// A retained, adapter-owned value created on GPUI's application thread.
///
/// Unlike [`ComponentPayload`], the value intentionally need not be `Send` or
/// `Sync`: GPUI entities and component state are owned by the app thread.
pub type RetainedState = Box<dyn Any + 'static>;

type StateFactory = dyn Fn(&[ComponentArgument], &mut Window, &mut App) -> Result<RetainedState, String>
    + Send
    + Sync
    + 'static;

#[derive(Clone)]
pub struct StateDescriptor {
    pub export: &'static str,
    pub kind: &'static str,
    pub arguments: Vec<ArgumentDescriptor>,
    pub documentation: Option<&'static str>,
    factory: Arc<StateFactory>,
}

impl StateDescriptor {
    /// Registers an app-thread state factory.
    ///
    /// The shell validates arguments first, and only publishes the returned
    /// value after the factory succeeds. On error it rolls back callbacks
    /// recorded while validating arguments and inserts no retained-state slot.
    /// The factory itself is responsible for transactional behavior: it must
    /// not perform irreversible external side effects before returning `Ok`.
    pub fn new(
        export: &'static str,
        kind: &'static str,
        arguments: Vec<ArgumentDescriptor>,
        factory: impl Fn(&[ComponentArgument], &mut Window, &mut App) -> Result<RetainedState, String>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            export,
            kind,
            arguments,
            documentation: None,
            factory: Arc::new(factory),
        }
    }

    pub fn documented(mut self, documentation: &'static str) -> Self {
        self.documentation = Some(documentation);
        self
    }

    pub(crate) fn create(
        &self,
        arguments: &[ComponentArgument],
        window: &mut Window,
        cx: &mut App,
    ) -> Result<RetainedState, String> {
        (self.factory)(arguments, window, cx)
    }
}

impl fmt::Debug for StateDescriptor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StateDescriptor")
            .field("export", &self.export)
            .field("kind", &self.kind)
            .field("arguments", &self.arguments)
            .field("documentation", &self.documentation)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeprecationDescriptor {
    pub replacement: &'static str,
    pub message: &'static str,
}

#[derive(Clone)]
pub struct ConstructorDescriptor {
    pub export: &'static str,
    pub arguments: Vec<ArgumentDescriptor>,
    pub deprecation: Option<DeprecationDescriptor>,
    factory: Arc<PayloadFactory>,
}

impl ConstructorDescriptor {
    pub fn new(
        export: &'static str,
        arguments: Vec<ArgumentDescriptor>,
        factory: impl Fn(&[ComponentArgument]) -> Result<ComponentPayload, String>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            export,
            arguments,
            deprecation: None,
            factory: Arc::new(factory),
        }
    }

    pub fn deprecated(mut self, replacement: &'static str, message: &'static str) -> Self {
        self.deprecation = Some(DeprecationDescriptor {
            replacement,
            message,
        });
        self
    }

    pub(crate) fn payload(
        &self,
        arguments: &[ComponentArgument],
    ) -> Result<ComponentPayload, String> {
        (self.factory)(arguments)
    }
}

impl fmt::Debug for ConstructorDescriptor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConstructorDescriptor")
            .field("export", &self.export)
            .field("arguments", &self.arguments)
            .field("deprecation", &self.deprecation)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct MethodDescriptor {
    pub name: &'static str,
    pub arguments: Vec<ArgumentDescriptor>,
    pub documentation: Option<&'static str>,
    recorder: Arc<PayloadFactory>,
}

impl MethodDescriptor {
    pub fn new(
        name: &'static str,
        arguments: Vec<ArgumentDescriptor>,
        recorder: impl Fn(&[ComponentArgument]) -> Result<ComponentPayload, String>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            name,
            arguments,
            documentation: None,
            recorder: Arc::new(recorder),
        }
    }

    pub fn documented(mut self, documentation: &'static str) -> Self {
        self.documentation = Some(documentation);
        self
    }

    pub(crate) fn record(
        &self,
        arguments: &[ComponentArgument],
    ) -> Result<ComponentPayload, String> {
        (self.recorder)(arguments)
    }
}

impl fmt::Debug for MethodDescriptor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MethodDescriptor")
            .field("name", &self.name)
            .field("arguments", &self.arguments)
            .field("documentation", &self.documentation)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypeScriptDescriptor {
    pub documentation: Option<&'static str>,
}

impl TypeScriptDescriptor {
    pub const fn new(documentation: &'static str) -> Self {
        Self {
            documentation: Some(documentation),
        }
    }
}

pub struct MaterializeRequest<'a> {
    component_name: &'static str,
    payload: &'a ComponentPayload,
    operations: &'a [crate::spec::SpecOp],
    runtime: &'a Rc<crate::ShellRuntime>,
    resolve_element: &'a mut dyn FnMut(u32) -> anyhow::Result<AnyElement>,
    style: Option<StyleRefinement>,
    children: Vec<AnyElement>,
    child_specs: Vec<(u32, Option<&'static str>)>,
    issued_children: Vec<(u64, u32)>,
    next_child_token: u64,
    request_id: u64,
    child_lane: ChildLane,
    slots: Vec<(&'static str, AnyElement)>,
    disabled: bool,
    selected: bool,
    on_click: Option<crate::spec::CallbackId>,
}

pub(crate) struct MaterializeRequestInit<'a> {
    pub component_name: &'static str,
    pub payload: &'a ComponentPayload,
    pub operations: &'a [crate::spec::SpecOp],
    pub runtime: &'a Rc<crate::ShellRuntime>,
    pub resolve_element: &'a mut dyn FnMut(u32) -> anyhow::Result<AnyElement>,
    pub style: StyleRefinement,
    pub children: Vec<AnyElement>,
    pub child_specs: Vec<(u32, Option<&'static str>)>,
    pub slots: Vec<(&'static str, AnyElement)>,
    pub disabled: bool,
    pub selected: bool,
    pub on_click: Option<crate::spec::CallbackId>,
}

impl<'a> MaterializeRequest<'a> {
    pub(crate) fn new(init: MaterializeRequestInit<'a>) -> Self {
        Self {
            component_name: init.component_name,
            payload: init.payload,
            operations: init.operations,
            runtime: init.runtime,
            resolve_element: init.resolve_element,
            style: Some(init.style),
            children: init.children,
            child_specs: init.child_specs,
            issued_children: Vec::new(),
            next_child_token: 0,
            request_id: NEXT_MATERIALIZE_REQUEST_ID.fetch_add(1, Ordering::Relaxed),
            child_lane: ChildLane::Unclaimed,
            slots: init.slots,
            disabled: init.disabled,
            selected: init.selected,
            on_click: init.on_click,
        }
    }

    pub fn payload(&self) -> &ComponentPayload {
        self.payload
    }

    /// Reads adapter-owned retained state without allowing render-time mutation.
    pub fn with_state<T: Any, R>(
        &self,
        argument: &ComponentArgument,
        body: impl FnOnce(&T) -> R,
    ) -> anyhow::Result<R> {
        let ComponentArgument::Entity { kind, handle } = argument else {
            anyhow::bail!("component argument is not retained state");
        };
        self.runtime
            .with_component_state::<T, R>(*handle, kind, body)
    }

    /// Returns an opaque capability that may update state later from a GPUI event.
    pub fn state_handle<T: Any>(
        &self,
        argument: &ComponentArgument,
    ) -> anyhow::Result<ComponentState<T>> {
        let ComponentArgument::Entity { kind, handle } = argument else {
            anyhow::bail!("component argument is not retained state");
        };
        self.runtime
            .with_component_state::<T, _>(*handle, kind, |_| ())?;
        Ok(ComponentState {
            runtime: Rc::downgrade(self.runtime),
            kind,
            handle: *handle,
            state: PhantomData,
        })
    }

    pub fn methods(&self) -> impl Iterator<Item = &RecordedComponentMethod> {
        self.operations
            .iter()
            .filter_map(|operation| match operation {
                crate::spec::SpecOp::RegisteredMethod(method) => Some(method),
                _ => None,
            })
    }

    pub fn disabled(&self) -> bool {
        self.disabled
    }

    pub fn selected(&self) -> bool {
        self.selected
    }

    pub fn children_len(&self) -> usize {
        self.children.len() + self.child_specs.len() + self.issued_children.len()
    }

    pub fn take_children(&mut self) -> anyhow::Result<Vec<AnyElement>> {
        if self.child_lane == ChildLane::Typed {
            tracing::warn!(
                "{} already issued typed children; take_children is exclusive with take_typed_children",
                self.component_name
            );
            return Ok(Vec::new());
        }
        self.child_lane = ChildLane::Ordinary;
        let mut children = std::mem::take(&mut self.children);
        while let Some((id, _)) = self.child_specs.first().copied() {
            let child = (self.resolve_element)(id)?;
            self.child_specs.remove(0);
            children.push(child);
        }
        Ok(children)
    }

    /// Takes opaque child tokens for adapters whose Rust component requires
    /// typed children. This is exclusive with [`Self::take_children`].
    pub fn take_typed_children(&mut self) -> Vec<ComponentChild<'a>> {
        if self.child_lane == ChildLane::Ordinary {
            tracing::warn!(
                "{} already materialized ordinary children; take_typed_children is exclusive with take_children",
                self.component_name
            );
            return Vec::new();
        }
        self.child_lane = ChildLane::Typed;
        std::mem::take(&mut self.child_specs)
            .into_iter()
            .map(|(id, component_name)| {
                let token = self.next_child_token;
                self.next_child_token = self
                    .next_child_token
                    .checked_add(1)
                    .expect("a materialization request exhausted its child token space");
                self.issued_children.push((token, id));
                ComponentChild {
                    runtime: Rc::downgrade(self.runtime),
                    request_id: self.request_id,
                    token,
                    component_name,
                    request_lifetime: PhantomData,
                }
            })
            .collect()
    }

    /// Materializes one opaque child token exactly once.
    pub fn materialize_child(
        &mut self,
        child: &mut ComponentChild<'a>,
    ) -> anyhow::Result<AnyElement> {
        anyhow::ensure!(
            child.request_id == self.request_id
                && Weak::ptr_eq(&child.runtime, &Rc::downgrade(self.runtime)),
            "component child belongs to a different runtime or materialization request"
        );
        let index = self
            .issued_children
            .iter()
            .position(|(token, _)| *token == child.token)
            .ok_or_else(|| anyhow::anyhow!("component child was already consumed"))?;
        let id = self.issued_children[index].1;
        let element = (self.resolve_element)(id)?;
        self.issued_children.remove(index);
        Ok(element)
    }

    /// Takes a named, already-materialized slot.
    ///
    /// Slots remain separate from ordinary children because the registered
    /// component owns their placement. Any slot left unread is diagnosed when
    /// the request is dropped instead of disappearing silently.
    pub fn take_slot(&mut self, name: &str) -> Option<AnyElement> {
        let index = self.slots.iter().position(|(held, _)| *held == name)?;
        Some(self.slots.remove(index).1)
    }

    /// Takes all slots with `name`, preserving script order.
    pub fn take_slots(&mut self, name: &str) -> Vec<AnyElement> {
        let mut taken = Vec::new();
        let mut index = 0;
        while index < self.slots.len() {
            if self.slots[index].0 == name {
                taken.push(self.slots.remove(index).1);
            } else {
                index += 1;
            }
        }
        taken
    }

    pub fn take_style(&mut self) -> StyleRefinement {
        self.style.take().unwrap_or_default()
    }

    pub fn on_click(&self) -> Option<ComponentClickCallback> {
        self.on_click.map(|id| ComponentClickCallback {
            runtime: Rc::downgrade(self.runtime),
            id,
        })
    }

    /// Applies this node's style and ordinary children exactly once.
    pub fn finish<E>(mut self, mut element: E) -> anyhow::Result<AnyElement>
    where
        E: Styled + ParentElement + IntoElement + 'static,
    {
        element.style().refine(&self.take_style());
        if self.child_lane != ChildLane::Typed {
            element.extend(self.take_children()?);
        }
        Ok(element.into_any_element())
    }

    fn unread_parts(&self) -> (bool, usize, Vec<&'static str>) {
        (
            self.style.is_some(),
            self.children.len() + self.child_specs.len() + self.issued_children.len(),
            self.slots.iter().map(|(name, _)| *name).collect(),
        )
    }

    pub fn resolve_element(&mut self, argument: &ComponentArgument) -> anyhow::Result<AnyElement> {
        let ComponentArgument::Element(element) = argument else {
            anyhow::bail!("component argument is not an Element");
        };
        (self.resolve_element)(*element)
    }

    pub fn resolve_entity(
        &self,
        argument: &ComponentArgument,
    ) -> anyhow::Result<ComponentEntityRef> {
        let ComponentArgument::Entity { kind, handle } = argument else {
            anyhow::bail!("component argument is not an entity");
        };
        let actual = self
            .runtime
            .component_entity_kind(*handle)
            .ok_or_else(|| anyhow::anyhow!("component entity is no longer live"))?;
        anyhow::ensure!(
            actual == *kind,
            "component entity kind changed from `{kind}` to `{actual}`"
        );
        Ok(ComponentEntityRef {
            runtime: Rc::downgrade(self.runtime),
            kind,
            handle: *handle,
        })
    }

    pub fn resolve_callback(
        &self,
        argument: &ComponentArgument,
    ) -> anyhow::Result<ComponentCallback> {
        let ComponentArgument::Callback(callback) = argument else {
            anyhow::bail!("component argument is not a callback");
        };
        Ok(ComponentCallback {
            runtime: Rc::downgrade(self.runtime),
            id: *callback,
        })
    }
}

impl Drop for MaterializeRequest<'_> {
    fn drop(&mut self) {
        let (style, children, slots) = self.unread_parts();
        if style {
            tracing::warn!(
                "{} did not consume its style; call MaterializeRequest::finish or take_style",
                self.component_name
            );
        }
        if children != 0 {
            tracing::warn!(
                "{} did not consume {} child element(s)",
                self.component_name,
                children
            );
        }
        for name in slots {
            tracing::warn!(
                "{} has no `{name}` slot, so the element given to it is not rendered at all",
                self.component_name
            );
        }
    }
}

#[derive(Clone)]
pub struct ComponentEntityRef {
    runtime: Weak<crate::ShellRuntime>,
    kind: &'static str,
    handle: u64,
}

impl ComponentEntityRef {
    pub fn kind(&self) -> &'static str {
        self.kind
    }

    pub fn is_live(&self) -> bool {
        self.runtime
            .upgrade()
            .is_some_and(|runtime| runtime.component_entity_kind(self.handle) == Some(self.kind))
    }
}

/// Opaque authority to update one adapter-owned state value from a GPUI event.
pub struct ComponentState<T: Any> {
    runtime: Weak<crate::ShellRuntime>,
    kind: &'static str,
    handle: u64,
    state: PhantomData<fn() -> T>,
}

impl<T: Any> Clone for ComponentState<T> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            kind: self.kind,
            handle: self.handle,
            state: PhantomData,
        }
    }
}

impl<T: Any> ComponentState<T> {
    pub fn update<R>(
        &self,
        window: &mut Window,
        cx: &mut App,
        body: impl FnOnce(&mut T, &mut Window, &mut App) -> R,
    ) -> anyhow::Result<R> {
        let runtime = self
            .runtime
            .upgrade()
            .ok_or_else(|| anyhow::anyhow!("retained state runtime has been released"))?;
        runtime.update_component_state(self.handle, self.kind, window, cx, body)
    }
}

#[derive(Clone)]
pub struct ComponentCallback {
    runtime: Weak<crate::ShellRuntime>,
    id: u64,
}

#[derive(Clone)]
pub struct ComponentClickCallback {
    runtime: Weak<crate::ShellRuntime>,
    id: crate::spec::CallbackId,
}

impl ComponentClickCallback {
    pub fn invoke(&self, event: &ClickEvent, window: &mut Window, cx: &mut App) {
        let Some(runtime) = self.runtime.upgrade() else {
            tracing::debug!("component click callback runtime has been released");
            return;
        };
        runtime.dispatch_click(self.id, event, window, cx);
    }
}

impl ComponentCallback {
    pub fn invoke(&self, window: &mut Window, cx: &mut App) -> anyhow::Result<()> {
        self.invoke_with(&[], window, cx)
    }

    pub fn invoke_with(
        &self,
        arguments: &[ComponentCallbackArgument],
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<()> {
        let runtime = self
            .runtime
            .upgrade()
            .ok_or_else(|| anyhow::anyhow!("component callback runtime has been released"))?;
        runtime.dispatch_component_callback(self.id, arguments, window, cx)
    }

    /// Invokes the script callback and reports any failure through the shell's
    /// tracing subscriber.
    ///
    /// GPUI event closures cannot return an error. Adapters should use this
    /// entry point instead of discarding the [`Result`] from [`Self::invoke_with`].
    pub fn invoke_and_report_with(
        &self,
        context: &str,
        arguments: &[ComponentCallbackArgument],
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Err(error) = self.invoke_with(arguments, window, cx) {
            tracing::error!("{context}: {error:#}");
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordedComponentMethod {
    name: &'static str,
    payload: ComponentPayload,
}

impl RecordedComponentMethod {
    pub(crate) fn new(name: &'static str, payload: ComponentPayload) -> Self {
        Self { name, payload }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn payload(&self) -> &ComponentPayload {
        &self.payload
    }
}

pub trait ComponentMaterializer: Send + Sync + 'static {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement>;
}

pub struct ComponentDescriptor {
    pub name: &'static str,
    pub constructors: Vec<ConstructorDescriptor>,
    pub methods: Vec<MethodDescriptor>,
    pub typescript: TypeScriptDescriptor,
    pub materializer: Arc<dyn ComponentMaterializer>,
}

impl fmt::Debug for ComponentDescriptor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComponentDescriptor")
            .field("name", &self.name)
            .field("constructors", &self.constructors)
            .field("methods", &self.methods)
            .field("typescript", &self.typescript)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum RegistryError {
    IncompatibleApiVersion {
        expected: u32,
        actual: u32,
    },
    DuplicateComponent(&'static str),
    InvalidComponent(&'static str),
    DuplicateExport(&'static str),
    InvalidExport(&'static str),
    InvalidMethod {
        component: &'static str,
        method: &'static str,
    },
    InvalidArgument {
        component: &'static str,
        callable: &'static str,
        argument: &'static str,
    },
    DuplicateArgument {
        component: &'static str,
        callable: &'static str,
        argument: &'static str,
    },
    InvalidArgumentSchema {
        component: &'static str,
        callable: &'static str,
        argument: &'static str,
        reason: &'static str,
    },
    RequiredArgumentAfterOptional {
        component: &'static str,
        callable: &'static str,
        argument: &'static str,
    },
    DuplicateMethod {
        component: &'static str,
        method: &'static str,
    },
    InvalidDeprecationReplacement {
        component: &'static str,
        export: &'static str,
        replacement: &'static str,
    },
    EmptyConstructorList(&'static str),
    InvalidStateKind(&'static str),
    DuplicateStateKind(&'static str),
    Frozen,
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleApiVersion { expected, actual } => write!(
                formatter,
                "component registry API version {actual} is incompatible; expected {expected}"
            ),
            Self::DuplicateComponent(name) => {
                write!(formatter, "component `{name}` is already registered")
            }
            Self::InvalidComponent(name) => write!(
                formatter,
                "component name `{name}` is not a valid non-reserved identifier"
            ),
            Self::DuplicateExport(name) => {
                write!(
                    formatter,
                    "JavaScript export `{name}` is already registered"
                )
            }
            Self::InvalidExport(name) => write!(
                formatter,
                "JavaScript export `{name}` is not a valid non-reserved identifier"
            ),
            Self::InvalidMethod { component, method } => write!(
                formatter,
                "component `{component}` method `{method}` is not a valid non-reserved identifier"
            ),
            Self::InvalidArgument {
                component,
                callable,
                argument,
            } => write!(
                formatter,
                "component `{component}` callable `{callable}` argument `{argument}` is not a valid non-reserved identifier"
            ),
            Self::DuplicateArgument {
                component,
                callable,
                argument,
            } => write!(
                formatter,
                "component `{component}` callable `{callable}` repeats argument `{argument}`"
            ),
            Self::InvalidArgumentSchema {
                component,
                callable,
                argument,
                reason,
            } => write!(
                formatter,
                "component `{component}` callable `{callable}` argument `{argument}` has an invalid schema: {reason}"
            ),
            Self::RequiredArgumentAfterOptional {
                component,
                callable,
                argument,
            } => write!(
                formatter,
                "component `{component}` callable `{callable}` has required argument `{argument}` after an optional argument"
            ),
            Self::DuplicateMethod { component, method } => {
                write!(
                    formatter,
                    "component `{component}` registers method `{method}` twice"
                )
            }
            Self::InvalidDeprecationReplacement {
                component,
                export,
                replacement,
            } => write!(
                formatter,
                "component `{component}` deprecated export `{export}` must name another export from the same descriptor, not `{replacement}`"
            ),
            Self::EmptyConstructorList(name) => {
                write!(
                    formatter,
                    "component `{name}` has no JavaScript constructor"
                )
            }
            Self::InvalidStateKind(kind) => {
                write!(
                    formatter,
                    "state kind `{kind}` is not a valid non-reserved identifier"
                )
            }
            Self::DuplicateStateKind(kind) => {
                write!(formatter, "state kind `{kind}` is already registered")
            }
            Self::Frozen => formatter.write_str("component registry is frozen"),
        }
    }
}

impl std::error::Error for RegistryError {}

pub struct ComponentRegistry {
    descriptors: Vec<Arc<ComponentDescriptor>>,
    names: HashSet<&'static str>,
    exports: HashSet<&'static str>,
    states: Vec<Arc<StateDescriptor>>,
    state_kinds: HashSet<&'static str>,
    frozen: bool,
}

impl ComponentRegistry {
    pub fn new(api_version: u32) -> Result<Self, RegistryError> {
        if api_version != COMPONENT_REGISTRY_API_VERSION {
            return Err(RegistryError::IncompatibleApiVersion {
                expected: COMPONENT_REGISTRY_API_VERSION,
                actual: api_version,
            });
        }

        Ok(Self {
            descriptors: Vec::new(),
            names: HashSet::new(),
            exports: HashSet::new(),
            states: Vec::new(),
            state_kinds: HashSet::new(),
            frozen: false,
        })
    }

    pub fn register(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> Result<ComponentId, RegistryError> {
        if self.frozen {
            return Err(RegistryError::Frozen);
        }
        if self.names.contains(descriptor.name) {
            return Err(RegistryError::DuplicateComponent(descriptor.name));
        }
        if !is_javascript_identifier(descriptor.name) {
            return Err(RegistryError::InvalidComponent(descriptor.name));
        }
        if descriptor.constructors.is_empty() {
            return Err(RegistryError::EmptyConstructorList(descriptor.name));
        }

        let mut methods = HashSet::new();
        for method in &descriptor.methods {
            if !is_javascript_identifier(method.name) {
                return Err(RegistryError::InvalidMethod {
                    component: descriptor.name,
                    method: method.name,
                });
            }
            if !methods.insert(method.name) {
                return Err(RegistryError::DuplicateMethod {
                    component: descriptor.name,
                    method: method.name,
                });
            }
            validate_arguments(descriptor.name, method.name, &method.arguments)?;
        }
        let mut descriptor_exports = HashSet::new();
        for constructor in &descriptor.constructors {
            if !is_javascript_identifier(constructor.export) {
                return Err(RegistryError::InvalidExport(constructor.export));
            }
            validate_arguments(descriptor.name, constructor.export, &constructor.arguments)?;
            if !descriptor_exports.insert(constructor.export)
                || self.exports.contains(constructor.export)
            {
                return Err(RegistryError::DuplicateExport(constructor.export));
            }
        }
        for constructor in &descriptor.constructors {
            if let Some(deprecation) = &constructor.deprecation
                && (deprecation.replacement == constructor.export
                    || !descriptor_exports.contains(deprecation.replacement))
            {
                return Err(RegistryError::InvalidDeprecationReplacement {
                    component: descriptor.name,
                    export: constructor.export,
                    replacement: deprecation.replacement,
                });
            }
        }

        let id = ComponentId(self.descriptors.len() as u32);
        self.names.insert(descriptor.name);
        self.exports
            .extend(descriptor.constructors.iter().map(|entry| entry.export));
        self.descriptors.push(Arc::new(descriptor));
        Ok(id)
    }

    pub fn register_state(&mut self, descriptor: StateDescriptor) -> Result<(), RegistryError> {
        if self.frozen {
            return Err(RegistryError::Frozen);
        }
        if !is_javascript_identifier(descriptor.export) {
            return Err(RegistryError::InvalidExport(descriptor.export));
        }
        if !is_javascript_identifier(descriptor.kind) {
            return Err(RegistryError::InvalidStateKind(descriptor.kind));
        }
        if self.exports.contains(descriptor.export) {
            return Err(RegistryError::DuplicateExport(descriptor.export));
        }
        if self.state_kinds.contains(descriptor.kind) {
            return Err(RegistryError::DuplicateStateKind(descriptor.kind));
        }
        validate_arguments(descriptor.kind, descriptor.export, &descriptor.arguments)?;
        self.exports.insert(descriptor.export);
        self.state_kinds.insert(descriptor.kind);
        self.states.push(Arc::new(descriptor));
        Ok(())
    }

    pub fn freeze(&mut self) -> Result<FrozenComponentRegistry, RegistryError> {
        if self.frozen {
            return Err(RegistryError::Frozen);
        }
        self.frozen = true;
        Ok(FrozenComponentRegistry {
            descriptors: self.descriptors.clone().into(),
            states: self.states.clone().into(),
        })
    }
}

fn validate_arguments(
    component: &'static str,
    callable: &'static str,
    arguments: &[ArgumentDescriptor],
) -> Result<(), RegistryError> {
    let mut saw_optional = false;
    let mut names = HashSet::new();
    for argument in arguments {
        if !is_javascript_identifier(argument.name) {
            return Err(RegistryError::InvalidArgument {
                component,
                callable,
                argument: argument.name,
            });
        }
        if !names.insert(argument.name) {
            return Err(RegistryError::DuplicateArgument {
                component,
                callable,
                argument: argument.name,
            });
        }
        if let Err(reason) = validate_argument_schema(&argument.schema, true) {
            return Err(RegistryError::InvalidArgumentSchema {
                component,
                callable,
                argument: argument.name,
                reason,
            });
        }
        if matches!(argument.schema, ArgumentSchema::Optional(_)) {
            saw_optional = true;
        } else if saw_optional {
            return Err(RegistryError::RequiredArgumentAfterOptional {
                component,
                callable,
                argument: argument.name,
            });
        }
    }
    Ok(())
}

fn validate_argument_schema(schema: &ArgumentSchema, top_level: bool) -> Result<(), &'static str> {
    match schema {
        ArgumentSchema::String
        | ArgumentSchema::Number
        | ArgumentSchema::Boolean
        | ArgumentSchema::Element => Ok(()),
        ArgumentSchema::Entity(kind) if kind.trim().is_empty() => {
            Err("entity kind must not be empty")
        }
        ArgumentSchema::Entity(_) => Ok(()),
        ArgumentSchema::Callback(signature) if signature.trim().is_empty() => {
            Err("callback signature must not be empty")
        }
        ArgumentSchema::Callback(_) => Ok(()),
        ArgumentSchema::Enum([]) => Err("enum must contain at least one literal"),
        ArgumentSchema::Enum(values) if values.iter().any(|value| value.is_empty()) => {
            Err("enum literals must not be empty")
        }
        ArgumentSchema::Enum(values) => {
            let mut unique = HashSet::with_capacity(values.len());
            if values.iter().all(|value| unique.insert(*value)) {
                Ok(())
            } else {
                Err("enum literals must be unique")
            }
        }
        ArgumentSchema::Array(item) => validate_argument_schema(item, false),
        ArgumentSchema::Optional(_) if !top_level => {
            Err("optional schemas are only valid for top-level arguments")
        }
        ArgumentSchema::Optional(item) => validate_argument_schema(item, false),
    }
}

fn is_javascript_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first == '$' || first.is_ascii_alphabetic())
        || !chars.all(|character| {
            character == '_' || character == '$' || character.is_ascii_alphanumeric()
        })
    {
        return false;
    }
    !matches!(
        name,
        "arguments"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "eval"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "implements"
            | "import"
            | "in"
            | "instanceof"
            | "interface"
            | "let"
            | "new"
            | "null"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "return"
            | "static"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
    )
}

#[derive(Clone, Default)]
pub struct FrozenComponentRegistry {
    descriptors: Arc<[Arc<ComponentDescriptor>]>,
    states: Arc<[Arc<StateDescriptor>]>,
}

impl FrozenComponentRegistry {
    pub fn descriptors(&self) -> impl ExactSizeIterator<Item = &ComponentDescriptor> {
        self.descriptors.iter().map(Arc::as_ref)
    }

    pub fn descriptor(&self, id: ComponentId) -> Option<&ComponentDescriptor> {
        self.descriptors.get(id.0 as usize).map(Arc::as_ref)
    }

    pub fn states(&self) -> impl ExactSizeIterator<Item = &StateDescriptor> {
        self.states.iter().map(Arc::as_ref)
    }

    pub(crate) fn registered(
        &self,
    ) -> impl ExactSizeIterator<Item = (ComponentId, &ComponentDescriptor)> {
        self.descriptors
            .iter()
            .enumerate()
            .map(|(index, descriptor)| (ComponentId(index as u32), descriptor.as_ref()))
    }

    pub(crate) fn javascript_module_source(&self, state_proof: &str) -> String {
        let mut source = format!(
            "const __stateHandles = new WeakMap();\nconst __stateProof = {state_proof:?};\n\
             function __unwrapState(value) {{\n\
               if (__stateHandles.has(value)) return {{ __componentStateHandle: __stateHandles.get(value), __componentStateProof: __stateProof }};\n\
               if (Array.isArray(value)) return value.map(__unwrapState);\n\
               return value;\n\
             }}\n"
        );
        for state in self.states() {
            source.push_str("function ");
            source.push_str(state.export);
            source.push_str("(...args) { const handle = globalThis.__gpui[");
            source.push_str(&format!("{:?}", state.export));
            source.push_str("](args); const value = Object.freeze({}); __stateHandles.set(value, handle); return value; }\nexport { ");
            source.push_str(state.export);
            source.push_str(" };\n");
        }
        for descriptor in self.descriptors() {
            for constructor in &descriptor.constructors {
                source.push_str("function ");
                source.push_str(constructor.export);
                source
                    .push_str("(...args) { return globalThis.__gpui.__element(globalThis.__gpui[");
                source.push_str(&format!("{:?}", constructor.export));
                source.push_str("](args.map(__unwrapState))); }\nexport { ");
                source.push_str(constructor.export);
                source.push_str(" };\n");
            }
        }
        source
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::div;

    #[test]
    fn component_payloads_cross_the_adapter_boundary_as_send_sync_values() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<ComponentPayload>();
    }

    struct EmptyMaterializer;

    impl ComponentMaterializer for EmptyMaterializer {
        fn materialize(&self, _request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            Ok(div().into_any_element())
        }
    }

    #[test]
    fn materialize_request_keeps_named_slots_separate_until_the_adapter_takes_them() {
        let runtime = crate::ShellRuntime::new_isolated().unwrap();
        let payload = ComponentPayload::new(());
        let operations = [];
        let mut resolve_element = |_| anyhow::bail!("no element argument expected");
        let mut request = MaterializeRequest::new(MaterializeRequestInit {
            component_name: "Slotted",
            payload: &payload,
            operations: &operations,
            runtime: &runtime,
            resolve_element: &mut resolve_element,
            style: StyleRefinement::default(),
            children: vec![div().into_any_element()],
            child_specs: Vec::new(),
            slots: vec![("trigger", div().into_any_element())],
            disabled: false,
            selected: false,
            on_click: None,
        });

        assert_eq!(request.unread_parts(), (true, 1, vec!["trigger"]));
        assert!(request.take_slot("content").is_none());
        assert!(request.take_slot("trigger").is_some());
        assert_eq!(request.take_children().unwrap().len(), 1);
        let _ = request.take_style();
        assert_eq!(request.unread_parts(), (false, 0, Vec::new()));
    }

    #[test]
    fn materialize_request_takes_every_repeated_named_slot_in_order() {
        let runtime = crate::ShellRuntime::new_isolated().unwrap();
        let payload = ComponentPayload::new(());
        let operations = [];
        let mut resolve_element = |_| anyhow::bail!("no element argument expected");
        let mut request = MaterializeRequest::new(MaterializeRequestInit {
            component_name: "Slotted",
            payload: &payload,
            operations: &operations,
            runtime: &runtime,
            resolve_element: &mut resolve_element,
            style: StyleRefinement::default(),
            children: Vec::new(),
            child_specs: Vec::new(),
            slots: vec![
                ("content", div().into_any_element()),
                ("content", div().into_any_element()),
                ("trigger", div().into_any_element()),
            ],
            disabled: false,
            selected: false,
            on_click: None,
        });

        assert_eq!(request.take_slots("content").len(), 2);
        assert!(request.take_slot("content").is_none());
        assert!(request.take_slot("trigger").is_some());
        let _ = request.take_style();
        assert_eq!(request.unread_parts(), (false, 0, Vec::new()));
    }

    #[test]
    fn typed_child_failure_keeps_the_token_retryable_and_unread() {
        let runtime = crate::ShellRuntime::new_isolated().unwrap();
        let payload = ComponentPayload::new(());
        let operations = [];
        let attempts = std::cell::Cell::new(0);
        let mut resolve_element = |_| {
            attempts.set(attempts.get() + 1);
            if attempts.get() == 1 {
                anyhow::bail!("candidate failed")
            }
            Ok(div().into_any_element())
        };
        let mut request = MaterializeRequest::new(MaterializeRequestInit {
            component_name: "Parent",
            payload: &payload,
            operations: &operations,
            runtime: &runtime,
            resolve_element: &mut resolve_element,
            style: StyleRefinement::default(),
            children: Vec::new(),
            child_specs: vec![(7, Some("RegisteredChild"))],
            slots: Vec::new(),
            disabled: false,
            selected: false,
            on_click: None,
        });

        let mut child = request.take_typed_children().pop().unwrap();
        assert_eq!(child.component_name(), Some("RegisteredChild"));
        assert!(request.materialize_child(&mut child).is_err());
        assert_eq!(request.unread_parts().1, 1);
        assert!(request.materialize_child(&mut child).is_ok());
        assert_eq!(request.unread_parts().1, 0);
        assert!(request.materialize_child(&mut child).is_err());
    }

    #[test]
    fn repeated_child_spec_ids_receive_independent_exactly_once_tokens() {
        let runtime = crate::ShellRuntime::new_isolated().unwrap();
        let payload = ComponentPayload::new(());
        let operations = [];
        let resolutions = std::cell::Cell::new(0);
        let mut resolve_element = |_| {
            resolutions.set(resolutions.get() + 1);
            Ok(div().into_any_element())
        };
        let mut request = MaterializeRequest::new(MaterializeRequestInit {
            component_name: "RepeatedParent",
            payload: &payload,
            operations: &operations,
            runtime: &runtime,
            resolve_element: &mut resolve_element,
            style: StyleRefinement::default(),
            children: Vec::new(),
            child_specs: vec![(7, Some("RepeatedChild")), (7, Some("RepeatedChild"))],
            slots: Vec::new(),
            disabled: false,
            selected: false,
            on_click: None,
        });

        let mut children = request.take_typed_children();
        let mut first = children.remove(0);
        let mut second = children.remove(0);
        assert!(request.materialize_child(&mut first).is_ok());
        assert!(request.materialize_child(&mut first).is_err());
        assert!(request.materialize_child(&mut second).is_ok());
        assert_eq!(resolutions.get(), 2);
        assert_eq!(request.unread_parts().1, 0);
    }

    #[test]
    fn child_lane_stays_exclusive_after_either_lane_is_drained() {
        let runtime = crate::ShellRuntime::new_isolated().unwrap();
        let payload = ComponentPayload::new(());
        let operations = [];

        let mut ordinary_resolver = |_| Ok(div().into_any_element());
        let mut ordinary = MaterializeRequest::new(MaterializeRequestInit {
            component_name: "Ordinary",
            payload: &payload,
            operations: &operations,
            runtime: &runtime,
            resolve_element: &mut ordinary_resolver,
            style: StyleRefinement::default(),
            children: Vec::new(),
            child_specs: vec![(1, None)],
            slots: Vec::new(),
            disabled: false,
            selected: false,
            on_click: None,
        });
        assert_eq!(ordinary.take_children().unwrap().len(), 1);
        assert!(ordinary.take_typed_children().is_empty());
        assert_eq!(ordinary.child_lane, ChildLane::Ordinary);

        let mut typed_resolver = |_| Ok(div().into_any_element());
        let mut typed = MaterializeRequest::new(MaterializeRequestInit {
            component_name: "Typed",
            payload: &payload,
            operations: &operations,
            runtime: &runtime,
            resolve_element: &mut typed_resolver,
            style: StyleRefinement::default(),
            children: Vec::new(),
            child_specs: vec![(2, None)],
            slots: Vec::new(),
            disabled: false,
            selected: false,
            on_click: None,
        });
        let mut child = typed.take_typed_children().pop().unwrap();
        assert_eq!(child.component_name(), None, "built-ins stay opaque");
        assert!(typed.materialize_child(&mut child).is_ok());
        assert!(typed.take_children().unwrap().is_empty());
        assert_eq!(typed.child_lane, ChildLane::Typed);
    }

    #[test]
    fn ordinary_child_failure_propagates_through_the_materializer_boundary() {
        struct FinishingMaterializer;

        impl ComponentMaterializer for FinishingMaterializer {
            fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
                request.finish(div())
            }
        }

        let runtime = crate::ShellRuntime::new_isolated().unwrap();
        let payload = ComponentPayload::new(());
        let operations = [];
        let mut resolve_element = |_| anyhow::bail!("child adapter failed");
        let request = MaterializeRequest::new(MaterializeRequestInit {
            component_name: "Parent",
            payload: &payload,
            operations: &operations,
            runtime: &runtime,
            resolve_element: &mut resolve_element,
            style: StyleRefinement::default(),
            children: Vec::new(),
            child_specs: vec![(9, Some("FailingChild"))],
            slots: Vec::new(),
            disabled: false,
            selected: false,
            on_click: None,
        });

        let error = FinishingMaterializer
            .materialize(request)
            .err()
            .expect("a failed child must fail its parent materializer");
        assert_eq!(error.to_string(), "child adapter failed");
    }

    #[test]
    fn typed_child_rejects_a_different_runtime_and_request() {
        let runtime_a = crate::ShellRuntime::new_isolated().unwrap();
        let runtime_b = crate::ShellRuntime::new_isolated().unwrap();
        let payload = ComponentPayload::new(());
        let operations = [];
        let mut resolver_a = |_| Ok(div().into_any_element());
        let mut resolver_b = |_| Ok(div().into_any_element());
        let mut resolver_c = |_| Ok(div().into_any_element());
        let mut request_a = MaterializeRequest::new(MaterializeRequestInit {
            component_name: "A",
            payload: &payload,
            operations: &operations,
            runtime: &runtime_a,
            resolve_element: &mut resolver_a,
            style: StyleRefinement::default(),
            children: Vec::new(),
            child_specs: vec![(3, Some("Child"))],
            slots: Vec::new(),
            disabled: false,
            selected: false,
            on_click: None,
        });
        let mut request_b = MaterializeRequest::new(MaterializeRequestInit {
            component_name: "B",
            payload: &payload,
            operations: &operations,
            runtime: &runtime_b,
            resolve_element: &mut resolver_b,
            style: StyleRefinement::default(),
            children: Vec::new(),
            child_specs: vec![(3, Some("Child"))],
            slots: Vec::new(),
            disabled: false,
            selected: false,
            on_click: None,
        });
        let mut request_c = MaterializeRequest::new(MaterializeRequestInit {
            component_name: "C",
            payload: &payload,
            operations: &operations,
            runtime: &runtime_a,
            resolve_element: &mut resolver_c,
            style: StyleRefinement::default(),
            children: Vec::new(),
            child_specs: vec![(3, Some("Child"))],
            slots: Vec::new(),
            disabled: false,
            selected: false,
            on_click: None,
        });

        let mut child = request_a.take_typed_children().pop().unwrap();
        let error = request_b
            .materialize_child(&mut child)
            .err()
            .expect("foreign child must fail");
        assert!(
            error
                .to_string()
                .contains("different runtime or materialization request")
        );
        let error = request_c
            .materialize_child(&mut child)
            .err()
            .expect("foreign request in the same runtime must fail");
        assert!(
            error
                .to_string()
                .contains("different runtime or materialization request")
        );
        assert_eq!(request_a.unread_parts().1, 1);
    }

    fn empty_descriptor(
        name: &'static str,
        constructors: Vec<ConstructorDescriptor>,
    ) -> ComponentDescriptor {
        ComponentDescriptor {
            name,
            constructors,
            methods: Vec::new(),
            typescript: TypeScriptDescriptor::default(),
            materializer: Arc::new(EmptyMaterializer),
        }
    }

    fn nullary(export: &'static str) -> ConstructorDescriptor {
        ConstructorDescriptor::new(export, Vec::new(), |_| Ok(ComponentPayload::new(())))
    }

    fn state(export: &'static str, kind: &'static str) -> StateDescriptor {
        StateDescriptor::new(export, kind, Vec::new(), |_, _, _| Ok(Box::new(0usize)))
    }

    #[test]
    fn state_exports_share_the_component_export_namespace() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        registry
            .register_state(state("InputState", "InputState"))
            .unwrap();

        assert_eq!(
            registry
                .register(empty_descriptor("Input", vec![nullary("InputState")]))
                .unwrap_err(),
            RegistryError::DuplicateExport("InputState")
        );
    }

    #[test]
    fn state_descriptors_validate_export_kind_and_arguments() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        assert_eq!(
            registry
                .register_state(state("class", "State"))
                .unwrap_err(),
            RegistryError::InvalidExport("class")
        );
        assert_eq!(
            registry
                .register_state(state("State", "bad-kind"))
                .unwrap_err(),
            RegistryError::InvalidStateKind("bad-kind")
        );
        assert_eq!(
            registry
                .register_state(StateDescriptor::new(
                    "State",
                    "State",
                    vec![ArgumentDescriptor::new("class", ArgumentSchema::String)],
                    |_, _, _| Ok(Box::new(())),
                ))
                .unwrap_err(),
            RegistryError::InvalidArgument {
                component: "State",
                callable: "State",
                argument: "class",
            }
        );

        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        registry
            .register_state(state("FirstState", "SharedState"))
            .unwrap();
        assert_eq!(
            registry
                .register_state(state("SecondState", "SharedState"))
                .unwrap_err(),
            RegistryError::DuplicateStateKind("SharedState")
        );
    }

    #[test]
    fn frozen_registry_module_declares_state_exports() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        registry
            .register_state(state("InputState", "InputState"))
            .unwrap();

        let source = registry
            .freeze()
            .unwrap()
            .javascript_module_source("test-proof");

        assert!(source.contains("export { InputState }"), "{source}");
    }

    #[test]
    fn retained_state_store_enforces_its_limit_without_overwriting_live_state() {
        let mut store = RetainedStateStore::default();
        for value in 0..MAX_RETAINED_COMPONENT_STATES {
            store.insert("State", None, Box::new(value)).unwrap();
        }
        let error = store.insert("State", None, Box::new(())).unwrap_err();
        assert!(error.contains("limit"), "{error}");
        assert_eq!(store.kind(0), Some("State"));
    }

    #[test]
    fn releasing_an_application_recovers_retained_state_capacity() {
        let owner = crate::runtime::ApplicationGeneration::new(91);
        let mut store = RetainedStateStore::default();
        for value in 0..MAX_RETAINED_COMPONENT_STATES {
            store
                .insert("State", Some(owner.clone()), Box::new(value))
                .unwrap();
        }
        store.release_application(&owner);
        assert_eq!(store.len(), 0);
        store.insert("State", None, Box::new(())).unwrap();
    }

    #[test]
    fn duplicate_methods_are_rejected_with_the_component_name() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        let error = registry
            .register(ComponentDescriptor {
                name: "Button",
                constructors: vec![nullary("Button")],
                methods: vec![
                    MethodDescriptor::new("disabled", Vec::new(), |_| {
                        Ok(ComponentPayload::new(()))
                    }),
                    MethodDescriptor::new("disabled", Vec::new(), |_| {
                        Ok(ComponentPayload::new(()))
                    }),
                ],
                typescript: TypeScriptDescriptor::default(),
                materializer: Arc::new(EmptyMaterializer),
            })
            .unwrap_err();

        assert_eq!(
            error,
            RegistryError::DuplicateMethod {
                component: "Button",
                method: "disabled",
            }
        );
    }

    #[test]
    fn duplicate_exports_inside_one_descriptor_are_rejected() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        let error = registry
            .register(empty_descriptor(
                "Button",
                vec![nullary("Button"), nullary("Button")],
            ))
            .unwrap_err();

        assert_eq!(error, RegistryError::DuplicateExport("Button"));
    }

    #[test]
    fn invalid_and_reserved_javascript_exports_are_rejected() {
        for export in ["not-valid", "class"] {
            let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
            let error = registry
                .register(empty_descriptor("Button", vec![nullary(export)]))
                .unwrap_err();
            assert_eq!(error, RegistryError::InvalidExport(export));
        }
    }

    #[test]
    fn method_and_argument_names_must_be_javascript_identifiers() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        let error = registry
            .register(empty_descriptor("not-valid", vec![nullary("Button")]))
            .unwrap_err();
        assert_eq!(error, RegistryError::InvalidComponent("not-valid"));

        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        let error = registry
            .register(ComponentDescriptor {
                name: "Button",
                constructors: vec![nullary("Button")],
                methods: vec![MethodDescriptor::new("not-valid", Vec::new(), |_| {
                    Ok(ComponentPayload::new(()))
                })],
                typescript: TypeScriptDescriptor::default(),
                materializer: Arc::new(EmptyMaterializer),
            })
            .unwrap_err();
        assert_eq!(
            error,
            RegistryError::InvalidMethod {
                component: "Button",
                method: "not-valid",
            }
        );

        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        let error = registry
            .register(empty_descriptor(
                "Button",
                vec![ConstructorDescriptor::new(
                    "Button",
                    vec![ArgumentDescriptor::new("class", ArgumentSchema::String)],
                    |_| Ok(ComponentPayload::new(())),
                )],
            ))
            .unwrap_err();
        assert_eq!(
            error,
            RegistryError::InvalidArgument {
                component: "Button",
                callable: "Button",
                argument: "class",
            }
        );
    }

    #[test]
    fn required_arguments_cannot_follow_optional_arguments() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        let error = registry
            .register(empty_descriptor(
                "Button",
                vec![ConstructorDescriptor::new(
                    "Button",
                    vec![
                        ArgumentDescriptor::new(
                            "label",
                            ArgumentSchema::Optional(Box::new(ArgumentSchema::String)),
                        ),
                        ArgumentDescriptor::new("id", ArgumentSchema::String),
                    ],
                    |_| Ok(ComponentPayload::new(())),
                )],
            ))
            .unwrap_err();
        assert_eq!(
            error,
            RegistryError::RequiredArgumentAfterOptional {
                component: "Button",
                callable: "Button",
                argument: "id",
            }
        );
    }

    #[test]
    fn argument_schemas_are_validated_recursively() {
        for (schema, reason) in [
            (
                ArgumentSchema::Array(Box::new(ArgumentSchema::Enum(&[]))),
                "enum must contain at least one literal",
            ),
            (
                ArgumentSchema::Optional(Box::new(ArgumentSchema::Array(Box::new(
                    ArgumentSchema::Enum(&["quiet", "quiet"]),
                )))),
                "enum literals must be unique",
            ),
            (
                ArgumentSchema::Array(Box::new(ArgumentSchema::Enum(&[""]))),
                "enum literals must not be empty",
            ),
            (
                ArgumentSchema::Array(Box::new(ArgumentSchema::Optional(Box::new(
                    ArgumentSchema::String,
                )))),
                "optional schemas are only valid for top-level arguments",
            ),
            (ArgumentSchema::Entity(""), "entity kind must not be empty"),
            (
                ArgumentSchema::Callback(" "),
                "callback signature must not be empty",
            ),
        ] {
            let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
            let error = registry
                .register(empty_descriptor(
                    "Button",
                    vec![ConstructorDescriptor::new(
                        "Button",
                        vec![ArgumentDescriptor::new("value", schema)],
                        |_| Ok(ComponentPayload::new(())),
                    )],
                ))
                .unwrap_err();
            assert_eq!(
                error,
                RegistryError::InvalidArgumentSchema {
                    component: "Button",
                    callable: "Button",
                    argument: "value",
                    reason,
                }
            );
        }
    }

    #[test]
    fn one_signature_cannot_repeat_an_argument_name() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        let error = registry
            .register(empty_descriptor(
                "Button",
                vec![ConstructorDescriptor::new(
                    "Button",
                    vec![
                        ArgumentDescriptor::new("value", ArgumentSchema::String),
                        ArgumentDescriptor::new("value", ArgumentSchema::Boolean),
                    ],
                    |_| Ok(ComponentPayload::new(())),
                )],
            ))
            .unwrap_err();
        assert_eq!(
            error,
            RegistryError::DuplicateArgument {
                component: "Button",
                callable: "Button",
                argument: "value",
            }
        );
    }

    #[test]
    fn deprecated_exports_must_name_another_export_from_the_same_descriptor() {
        for replacement in ["MissingButton", "OldButton"] {
            let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
            let error = registry
                .register(empty_descriptor(
                    "Button",
                    vec![
                        nullary("Button"),
                        nullary("OldButton").deprecated(replacement, "Use Button."),
                    ],
                ))
                .unwrap_err();
            assert_eq!(
                error,
                RegistryError::InvalidDeprecationReplacement {
                    component: "Button",
                    export: "OldButton",
                    replacement,
                }
            );
        }
    }
}
