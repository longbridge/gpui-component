//! Retained state that scripts hold by handle.
//!
//! The object model has three classes (design doc §7): values are copied,
//! element descriptions live for one render pass, and **entities** live across
//! frames and are owned by GPUI. A script never holds an entity directly — it
//! holds a handle into this store, so a released entity produces a clear error
//! instead of a dangling reference, and so the engines share one representation.
//!
//! The store is thread-local because the VM and GPUI's `App` are both
//! main-thread only.

use std::cell::RefCell;

use gpui::{App, AppContext as _, Entity, Subscription, Window};
use gpui_base::input::{InputEditorStyle, InputEvent, InputState};

/// A script-visible reference to retained state.
pub type EntityHandle = u32;

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
    static STORE: RefCell<Vec<Option<Record>>> = const { RefCell::new(Vec::new()) };
}

/// Creates an input state and returns its handle.
///
/// The editor style is installed here rather than left to the caller because
/// `InputEditorStyle::default()` is entirely transparent: an input built
/// without one renders invisible text, which is a failure no script author
/// could diagnose. The shell owns the default palette, so it owns this too.
pub fn create_input(
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

    push(Record::Input {
        state,
        subscriptions: Vec::new(),
    })
}

/// The entity behind an input handle, if it is still live.
pub fn input(handle: EntityHandle) -> Option<Entity<InputState>> {
    STORE.with(|store| match store.borrow().get(handle as usize) {
        Some(Some(Record::Input { state, .. })) => Some(state.clone()),
        _ => None,
    })
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

/// Subscribes to one input event for as long as the handle lives.
///
/// The subscription is owned by the store rather than by the script: a script
/// has no place to keep it, and a handler that stops firing because a value was
/// dropped is the kind of bug nobody finds.
pub fn subscribe_input(
    handle: EntityHandle,
    event: InputEventName,
    window: &mut Window,
    cx: &mut App,
    handler: impl Fn(&InputEvent, &mut Window, &mut App) + 'static,
) -> bool {
    let Some(state) = input(handle) else {
        return false;
    };

    let subscription = window.subscribe(&state, cx, move |_, emitted: &InputEvent, window, cx| {
        if event.matches(emitted) {
            handler(emitted, window, cx);
        }
    });

    STORE.with(|store| match store.borrow_mut().get_mut(handle as usize) {
        Some(Some(Record::Input { subscriptions, .. })) => {
            subscriptions.push(subscription);
            true
        }
        _ => false,
    })
}

/// Drops a handle. The entity itself is released when GPUI has no other owner.
pub fn release(handle: EntityHandle) -> bool {
    STORE.with(|store| {
        let mut store = store.borrow_mut();
        match store.get_mut(handle as usize) {
            Some(slot @ Some(_)) => {
                *slot = None;
                true
            }
            _ => false,
        }
    })
}

/// Releases every handle. Called when a runtime shuts down so a stored entity
/// cannot outlive the app it belongs to.
pub fn clear() {
    STORE.with(|store| store.borrow_mut().clear());
}

/// How many handles are live, for `gc_stats` and for tests that assert the
/// store does not grow without bound.
pub fn len() -> usize {
    STORE.with(|store| store.borrow().iter().filter(|slot| slot.is_some()).count())
}

fn push(record: Record) -> EntityHandle {
    STORE.with(|store| {
        let mut store = store.borrow_mut();
        // Reuse a released slot before growing: a long-lived app that opens and
        // closes many inputs should not leak handle space.
        if let Some(index) = store.iter().position(Option::is_none) {
            store[index] = Some(record);
            return index as EntityHandle;
        }
        store.push(Some(record));
        (store.len() - 1) as EntityHandle
    })
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

    #[test]
    fn a_released_handle_stops_resolving() {
        clear();
        // A handle that was never issued resolves to nothing rather than
        // panicking, which is what keeps a stale script reference reportable.
        assert!(input(0).is_none());
        assert!(!release(0));
    }

    #[test]
    fn the_store_starts_empty() {
        clear();
        assert_eq!(len(), 0);
    }
}
