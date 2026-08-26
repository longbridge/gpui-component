//! Retained state that scripts hold by handle.
//!
//! The object model has three classes (design doc §7): values are copied,
//! element descriptions live for one script render, and **entities** live across
//! frames and are owned by GPUI. A script never holds an entity directly — it
//! holds a handle into a store, so a released entity produces a clear error
//! instead of a dangling reference.
//!
//! # One store per runtime
//!
//! The store is a field of the runtime, not a process- or thread-global. Two
//! runtimes in one process — a host with two plugins, a test that builds two —
//! must not be able to reach each other's state, and the way to guarantee that
//! is for there to be no shared store to reach through.
//!
//! A handle carries the store it came from in its high bits, so the invariant is
//! also *checked* rather than merely arranged: a handle from another runtime
//! resolves to nothing and reports itself, instead of quietly resolving to
//! whatever that index happens to hold here.

use std::{cell::Cell, collections::HashMap, rc::Rc};

use gpui::{App, AppContext as _, Entity, FocusHandle, Subscription, Window};
use gpui_base::input::{
    InputBaseState, InputEditorStyle, InputEvent, InputModeKind, InputState, TextareaState,
};

use crate::runtime::ApplicationGeneration;

/// A script-visible reference to retained state.
///
/// The high [`STORE_SHIFT`] bits name the store, the low bits are a monotonic
/// id that is never reused. The 53-bit layout stays exactly representable by a
/// JavaScript number.
pub type EntityHandle = u64;

const STORE_SHIFT: u32 = 32;
const MAX_STORE_ID: u32 = (1 << 21) - 1;
const ENTITY_ID_MASK: u64 = u32::MAX as u64;

/// What a handle points at. One variant per entity type the script can create.
enum Record {
    Input {
        state: Entity<InputState>,
        application: Option<Rc<ApplicationGeneration>>,
        /// Subscriptions are stored, not returned, because a dropped
        /// `Subscription` stops delivering: a script that registers a handler
        /// and moves on would otherwise silently receive nothing.
        subscriptions: Vec<Subscription>,
    },
    /// Multi-line text state.
    ///
    /// A separate variant rather than a flag on [`Record::Input`] because
    /// `TextareaState` is a different Rust type — the same editing engine
    /// specialized on a different mode — and `Textarea::new` will not accept an
    /// `InputState`. The two share their event type and almost all of their
    /// methods, which is why everything below this point treats them together.
    Textarea {
        state: Entity<TextareaState>,
        application: Option<Rc<ApplicationGeneration>>,
        subscriptions: Vec<Subscription>,
    },
    /// A focus handle the script created and hands to elements.
    ///
    /// Retained for the same reason an input's state is: focus is a fact about
    /// the window that outlives any one render, and an element rebuilt every
    /// frame cannot own it. It is what lets a script say *which* control the
    /// keyboard is on, and what a `Select` or a `DatePicker` will be
    /// constructed from — their focus handle is a required argument, not a
    /// builder call.
    Focus {
        handle: FocusHandle,
        application: Option<Rc<ApplicationGeneration>>,
    },
}

thread_local! {
    /// Handed out at construction so two stores never share an id. Thread-local
    /// because the VM and GPUI's `App` are both main-thread only.
    static NEXT_STORE_ID: Cell<u32> = const { Cell::new(0) };
}

/// The retained state of one runtime.
///
/// Created by the runtime and dropped with it, which is what releases every
/// entity the scripts of that runtime created — a store that outlived its app
/// would show up as a leaked handle at shutdown.
pub struct EntityStore {
    id: u32,
    next_id: u32,
    records: HashMap<u32, Record>,
}

impl EntityStore {
    /// Creates a store with a process-unique JavaScript-safe namespace.
    ///
    /// Exhaustion is reported instead of wrapping: reusing a namespace could
    /// make a stale handle from an earlier runtime name a new runtime's entity.
    pub fn try_new() -> Option<Self> {
        let id = NEXT_STORE_ID.with(|next| {
            let (id, following) = allocate_store_id(next.get())?;
            next.set(following);
            Some(id)
        })?;
        Some(Self {
            id,
            next_id: 0,
            records: HashMap::new(),
        })
    }

    /// Creates an input state and returns its handle.
    ///
    /// The editor style is installed here rather than left to the caller because
    /// `InputEditorStyle::default()` is entirely transparent: an input built
    /// without one renders invisible text, which is a failure no script author
    /// could diagnose. The shell owns the default palette, so it owns this too.
    pub fn create_input(
        &mut self,
        placeholder: Option<String>,
        value: Option<String>,
        application: Option<Rc<ApplicationGeneration>>,
        window: &mut Window,
        cx: &mut App,
    ) -> EntityHandle {
        let state = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            if let Some(placeholder) = placeholder {
                state = state.placeholder(placeholder);
            }
            state.set_editor_style(editor_style());
            state
        });

        if let Some(value) = value {
            state.update(cx, |state, cx| state.set_value(value, window, cx));
        }

        self.push(Record::Input {
            state,
            application,
            subscriptions: Vec::new(),
        })
    }

    /// The entity behind an input handle, if it is still live and belongs here.
    pub fn input(&self, handle: EntityHandle) -> Option<Entity<InputState>> {
        match self.record(handle) {
            Some(Record::Input { state, .. }) => Some(state.clone()),
            _ => None,
        }
    }

    /// Creates a multi-line text state and returns its handle.
    ///
    /// `rows` is offered at construction because the layout default is a single
    /// row *even for a textarea* — being multi-line is carried by the mode
    /// rather than by the layout — so a script that asked for a textarea and
    /// said nothing else would get something the height of an input.
    ///
    /// The editor style is installed for the same reason as in
    /// [`Self::create_input`]: the default one is entirely transparent.
    pub fn create_textarea(
        &mut self,
        placeholder: Option<String>,
        value: Option<String>,
        rows: Option<usize>,
        application: Option<Rc<ApplicationGeneration>>,
        window: &mut Window,
        cx: &mut App,
    ) -> EntityHandle {
        let state = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx);
            if let Some(placeholder) = placeholder {
                state = state.placeholder(placeholder);
            }
            if let Some(rows) = rows {
                state = state.rows(rows);
            }
            state.set_editor_style(editor_style());
            state
        });

        if let Some(value) = value {
            state.update(cx, |state, cx| state.set_value(value, window, cx));
        }

        self.push(Record::Textarea {
            state,
            application,
            subscriptions: Vec::new(),
        })
    }

    /// The entity behind a textarea handle, if it is still live and belongs
    /// here.
    pub fn textarea(&self, handle: EntityHandle) -> Option<Entity<TextareaState>> {
        match self.record(handle) {
            Some(Record::Textarea { state, .. }) => Some(state.clone()),
            _ => None,
        }
    }

    /// Creates a focus handle and returns its handle.
    ///
    /// Only `&App` is needed — GPUI's own [`App::focus_handle`] takes no window
    /// — but this is still refused during render for the same reason
    /// [`Self::create_input`] is: a handle created inside `render` would be a
    /// new one on every frame, so the focus a script thought it was tracking
    /// would be dropped by the next repaint.
    pub fn create_focus(
        &mut self,
        application: Option<Rc<ApplicationGeneration>>,
        cx: &mut App,
    ) -> EntityHandle {
        self.push(Record::Focus {
            handle: cx.focus_handle(),
            application,
        })
    }

    /// The focus handle behind a handle, if it is still live and belongs here.
    pub fn focus(&self, handle: EntityHandle) -> Option<FocusHandle> {
        match self.record(handle) {
            Some(Record::Focus { handle, .. }) => Some(handle.clone()),
            _ => None,
        }
    }

    /// Subscribes to one input event for as long as the handle lives.
    ///
    /// The subscription is owned by the store rather than by the script: a
    /// script has no place to keep it, and a handler that stops firing because a
    /// value was dropped is the kind of bug nobody finds.
    ///
    /// One method serves both text states: they emit the same [`InputEvent`],
    /// so only the entity's type differs, and that difference is confined to
    /// the two arms that hand it to [`subscribe_to_events`].
    pub fn subscribe_input(
        &mut self,
        handle: EntityHandle,
        event: InputEventName,
        window: &mut Window,
        cx: &mut App,
        handler: impl Fn(&InputEvent, &mut Window, &mut App) + 'static,
    ) -> bool {
        let subscription = match self.record(handle) {
            Some(Record::Input { state, .. }) => {
                subscribe_to_events(&state.clone(), event, window, cx, handler)
            }
            Some(Record::Textarea { state, .. }) => {
                subscribe_to_events(&state.clone(), event, window, cx, handler)
            }
            _ => return false,
        };

        match self.record_mut(handle) {
            Some(Record::Input { subscriptions, .. } | Record::Textarea { subscriptions, .. }) => {
                subscriptions.push(subscription);
                true
            }
            _ => false,
        }
    }

    /// Drops a handle. The entity itself is released when GPUI has no other
    /// owner.
    pub fn release(&mut self, handle: EntityHandle) -> bool {
        let Some(id) = self.entity_id(handle) else {
            return false;
        };
        self.records.remove(&id).is_some()
    }

    /// Releases every handle. The runtime dropping the store does this anyway;
    /// this is for a host that wants the entities gone before the VM is.
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Releases retained state created by one evaluated application.
    ///
    /// Dropping an input record also drops its GPUI subscriptions, so unload
    /// cannot leave a handler (and its persistent JavaScript function) behind
    /// in the runtime-wide store.
    pub(crate) fn release_application(&mut self, application: &Rc<ApplicationGeneration>) {
        self.records.retain(|_, record| {
            let owner = match record {
                Record::Input {
                    application: owner, ..
                }
                | Record::Textarea {
                    application: owner, ..
                }
                | Record::Focus {
                    application: owner, ..
                } => owner,
            };
            owner
                .as_ref()
                .is_none_or(|owner| !Rc::ptr_eq(owner, application))
        });
    }

    /// How many handles are live, for `gc_stats` and for tests that assert the
    /// store does not grow without bound.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Every focus handle this store holds, in creation order.
    ///
    /// Ordered because a test asserting *which* control the keyboard landed on
    /// has to be able to name the second one.
    #[cfg(test)]
    pub(crate) fn focus_handles(&self) -> Vec<FocusHandle> {
        let mut ids: Vec<u32> = self.records.keys().copied().collect();
        ids.sort_unstable();
        ids.iter()
            .filter_map(|id| match self.records.get(id) {
                Some(Record::Focus { handle, .. }) => Some(handle.clone()),
                _ => None,
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn first_input(&self) -> Option<Entity<InputState>> {
        self.records.values().find_map(|record| match record {
            Record::Input { state, .. } => Some(state.clone()),
            _ => None,
        })
    }

    /// Splits a handle into its entity id, refusing one that names another store.
    ///
    /// A cross-store handle is a host bug rather than a script mistake — a
    /// script can only ever have been given handles from its own runtime — so
    /// this logs rather than throwing, and resolves to nothing.
    fn entity_id(&self, handle: EntityHandle) -> Option<u32> {
        let store = (handle >> STORE_SHIFT) as u32;
        if store != self.id {
            tracing::error!(
                "entity handle {handle} belongs to store {store}, not to store {}",
                self.id
            );
            return None;
        }
        Some((handle & ENTITY_ID_MASK) as u32)
    }

    fn record(&self, handle: EntityHandle) -> Option<&Record> {
        self.records.get(&self.entity_id(handle)?)
    }

    fn record_mut(&mut self, handle: EntityHandle) -> Option<&mut Record> {
        let id = self.entity_id(handle)?;
        self.records.get_mut(&id)
    }

    fn push(&mut self, record: Record) -> EntityHandle {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("a shell runtime cannot create more than 2^32 retained entities");
        self.records.insert(id, record);
        (u64::from(self.id) << STORE_SHIFT) | u64::from(id)
    }
}

/// Delivers one named event from any of the text states to `handler`.
///
/// Generic over the mode marker rather than written once per state: the filter
/// and the subscription are identical, and duplicating them would let the two
/// drift into answering `change` differently.
fn subscribe_to_events<M: InputModeKind>(
    state: &Entity<InputBaseState<M>>,
    event: InputEventName,
    window: &mut Window,
    cx: &mut App,
    handler: impl Fn(&InputEvent, &mut Window, &mut App) + 'static,
) -> Subscription {
    window.subscribe(state, cx, move |_, emitted: &InputEvent, window, cx| {
        if event.matches(emitted) {
            handler(emitted, window, cx);
        }
    })
}

fn allocate_store_id(next: u32) -> Option<(u32, u32)> {
    (next <= MAX_STORE_ID).then(|| (next, next + 1))
}

/// The events a script can subscribe to, named for what they mean rather than
/// for the key that produced them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputEventName {
    Change,
    Submit,
    Focus,
    Blur,
}

impl InputEventName {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "change" => Some(Self::Change),
            "submit" => Some(Self::Submit),
            "focus" => Some(Self::Focus),
            "blur" => Some(Self::Blur),
            _ => None,
        }
    }

    pub const NAMES: &'static [&'static str] = &["change", "submit", "focus", "blur"];

    fn matches(self, event: &InputEvent) -> bool {
        matches!(
            (self, event),
            (Self::Change, InputEvent::Change)
                | (Self::Submit, InputEvent::PressEnter { .. })
                | (Self::Focus, InputEvent::Focus)
                | (Self::Blur, InputEvent::Blur)
        )
    }
}

fn editor_style() -> InputEditorStyle {
    let color =
        |name: &str, fallback: gpui::Hsla| crate::theme::token_color(name).unwrap_or(fallback);
    let foreground = color("foreground", gpui::rgb(0x10151d).into());
    let mut selection = color("accent", gpui::rgb(0xdde7fb).into());
    // A selection must not hide the glyphs it selects.
    selection.a = 0.4;

    InputEditorStyle {
        foreground,
        muted_foreground: color("muted_foreground", gpui::rgb(0x5a6577).into()),
        background: color("surface", gpui::rgb(0xffffff).into()),
        border: color("border", gpui::rgb(0xd4dbe6).into()),
        selection,
        caret: foreground,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext};

    #[test]
    fn a_released_handle_stops_resolving() {
        let mut store = EntityStore::try_new().expect("store id");
        // A handle that was never issued resolves to nothing rather than
        // panicking, which is what keeps a stale script reference reportable.
        let unissued = u64::from(store.id) << STORE_SHIFT;
        assert!(store.input(unissued).is_none());
        assert!(!store.release(unissued));
    }

    #[test]
    fn a_store_starts_empty() {
        let store = EntityStore::try_new().expect("store id");
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn a_handle_from_another_store_does_not_resolve() {
        let first = EntityStore::try_new().expect("first store id");
        let second = EntityStore::try_new().expect("second store id");
        assert_ne!(first.id, second.id, "stores must not share an id");

        // Slot 0 of the other store. Without the store bits this would be a
        // valid index here, which is exactly the confusion the bits prevent.
        let foreign = u64::from(second.id) << STORE_SHIFT;
        assert!(first.entity_id(foreign).is_none());
    }

    #[gpui::test]
    fn released_and_cleared_handles_are_never_reissued(cx: &mut TestAppContext) {
        let mut store = EntityStore::try_new().expect("store id");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);

        let first = context.update(|window, cx| store.create_input(None, None, None, window, cx));
        assert!(store.release(first));
        let second = context.update(|window, cx| store.create_input(None, None, None, window, cx));
        assert_ne!(first, second);
        assert!(store.input(first).is_none());
        assert!(store.input(second).is_some());

        store.clear();
        let third = context.update(|window, cx| store.create_input(None, None, None, window, cx));
        assert_ne!(second, third);
        assert!(store.input(second).is_none());
        assert!(store.input(third).is_some());
    }

    /// A focus handle is retained state like any other: released by handle,
    /// released with its application, and never confused with an input.
    #[gpui::test]
    fn a_focus_handle_is_retained_and_released_like_any_other_entity(cx: &mut TestAppContext) {
        let mut store = EntityStore::try_new().expect("store id");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);

        let focus = context.update(|_, cx| store.create_focus(None, cx));
        let input = context.update(|window, cx| store.create_input(None, None, None, window, cx));

        // The two kinds do not answer for each other, which is what stops a
        // script from rendering an input where it asked for a focus target.
        assert!(store.focus(focus).is_some());
        assert!(store.input(focus).is_none());
        assert!(store.focus(input).is_none());

        assert!(store.release(focus));
        assert!(store.focus(focus).is_none());
        assert!(store.input(input).is_some());
    }

    /// Single-line and multi-line state are two Rust types, and `Textarea::new`
    /// will not take an `InputState`. A handle that resolved as either would
    /// therefore be a crash waiting to be materialized, so the store keeps them
    /// apart even though everything else about them is shared.
    #[gpui::test]
    fn a_textarea_handle_is_never_mistaken_for_an_input(cx: &mut TestAppContext) {
        let mut store = EntityStore::try_new().expect("store id");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);

        let input = context.update(|window, cx| store.create_input(None, None, None, window, cx));
        let textarea = context
            .update(|window, cx| store.create_textarea(None, None, Some(4), None, window, cx));

        assert!(store.textarea(input).is_none());
        assert!(store.input(textarea).is_none());
        assert!(store.textarea(textarea).is_some());

        // Subscribing reaches both through the one method, which is the part
        // that would silently stop working if a variant were forgotten there.
        assert!(context.update(|window, cx| store.subscribe_input(
            textarea,
            InputEventName::Change,
            window,
            cx,
            |_, _, _| {}
        )));

        assert!(store.release(textarea));
        assert!(store.textarea(textarea).is_none());
        assert!(store.input(input).is_some());
    }

    #[gpui::test]
    fn releasing_an_application_drops_only_its_entities(cx: &mut TestAppContext) {
        let mut store = EntityStore::try_new().expect("store id");
        let first_application = ApplicationGeneration::new(1);
        let second_application = ApplicationGeneration::new(2);
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);

        let first = context.update(|window, cx| {
            store.create_input(None, None, Some(first_application.clone()), window, cx)
        });
        let second = context.update(|window, cx| {
            store.create_input(None, None, Some(second_application.clone()), window, cx)
        });
        let focus = context.update(|_, cx| store.create_focus(Some(first_application.clone()), cx));

        store.release_application(&first_application);

        assert!(store.input(first).is_none());
        assert!(store.focus(focus).is_none());
        assert!(store.input(second).is_some());
    }

    #[test]
    fn store_ids_stop_before_the_javascript_safe_namespace_would_wrap() {
        assert_eq!(
            allocate_store_id(MAX_STORE_ID),
            Some((MAX_STORE_ID, MAX_STORE_ID + 1))
        );
        assert_eq!(allocate_store_id(MAX_STORE_ID + 1), None);
    }
}
