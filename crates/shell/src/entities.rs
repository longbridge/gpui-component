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

use std::{cell::Cell, collections::HashMap};

use gpui::{App, AppContext as _, Entity, Subscription, Window};
use gpui_base::input::{InputEditorStyle, InputEvent, InputState};

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
        /// Subscriptions are stored, not returned, because a dropped
        /// `Subscription` stops delivering: a script that registers a handler
        /// and moves on would otherwise silently receive nothing.
        subscriptions: Vec<Subscription>,
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
            subscriptions: Vec::new(),
        })
    }

    /// The entity behind an input handle, if it is still live and belongs here.
    pub fn input(&self, handle: EntityHandle) -> Option<Entity<InputState>> {
        match self.record(handle) {
            Some(Record::Input { state, .. }) => Some(state.clone()),
            None => None,
        }
    }

    /// Subscribes to one input event for as long as the handle lives.
    ///
    /// The subscription is owned by the store rather than by the script: a
    /// script has no place to keep it, and a handler that stops firing because a
    /// value was dropped is the kind of bug nobody finds.
    pub fn subscribe_input(
        &mut self,
        handle: EntityHandle,
        event: InputEventName,
        window: &mut Window,
        cx: &mut App,
        handler: impl Fn(&InputEvent, &mut Window, &mut App) + 'static,
    ) -> bool {
        let Some(state) = self.input(handle) else {
            return false;
        };

        let subscription =
            window.subscribe(&state, cx, move |_, emitted: &InputEvent, window, cx| {
                if event.matches(emitted) {
                    handler(emitted, window, cx);
                }
            });

        match self.record_mut(handle) {
            Some(Record::Input { subscriptions, .. }) => {
                subscriptions.push(subscription);
                true
            }
            None => false,
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

    #[cfg(test)]
    pub(crate) fn first_input(&self) -> Option<Entity<InputState>> {
        self.records.values().find_map(|record| match record {
            Record::Input { state, .. } => Some(state.clone()),
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

        let first = context.update(|window, cx| store.create_input(None, None, window, cx));
        assert!(store.release(first));
        let second = context.update(|window, cx| store.create_input(None, None, window, cx));
        assert_ne!(first, second);
        assert!(store.input(first).is_none());
        assert!(store.input(second).is_some());

        store.clear();
        let third = context.update(|window, cx| store.create_input(None, None, window, cx));
        assert_ne!(second, third);
        assert!(store.input(second).is_none());
        assert!(store.input(third).is_some());
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
