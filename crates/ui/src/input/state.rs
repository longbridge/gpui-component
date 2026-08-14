use gpui::{App, Entity, FocusHandle, Focusable as _, SharedString, Window};
use gpui_base::OtpState;

use super::{EditorState, InputState, TextareaState};
use crate::Root;

/// Any input-like state, regardless of which input element renders it.
///
/// [`InputState`], [`TextareaState`], [`EditorState`] and [`OtpState`] are
/// separate types, so an API that refers to “whatever input is here” takes this
/// enum instead of one of them. [`crate::WindowExt::focused_input`] returns it.
///
/// Use [`Self::as_input`] and friends to get the concrete state back, or
/// [`Self::value`] and [`Self::focus_handle`] when the kind does not matter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnyInputState {
    /// A single-line [`crate::input::Input`] state.
    Input(Entity<InputState>),
    /// A multi-line [`crate::input::Textarea`] state.
    Textarea(Entity<TextareaState>),
    /// A source-code [`crate::input::Editor`] state.
    Editor(Entity<EditorState>),
    /// A one-time-code [`crate::input::OtpInput`] state.
    Otp(Entity<OtpState>),
}

impl AnyInputState {
    /// Returns the [`InputState`], if this is an `Input` state.
    pub fn as_input(&self) -> Option<&Entity<InputState>> {
        match self {
            Self::Input(state) => Some(state),
            _ => None,
        }
    }

    /// Returns the [`TextareaState`], if this is a `Textarea` state.
    pub fn as_textarea(&self) -> Option<&Entity<TextareaState>> {
        match self {
            Self::Textarea(state) => Some(state),
            _ => None,
        }
    }

    /// Returns the [`EditorState`], if this is an `Editor` state.
    pub fn as_editor(&self) -> Option<&Entity<EditorState>> {
        match self {
            Self::Editor(state) => Some(state),
            _ => None,
        }
    }

    /// Returns the [`OtpState`], if this is an `OtpInput` state.
    pub fn as_otp(&self) -> Option<&Entity<OtpState>> {
        match self {
            Self::Otp(state) => Some(state),
            _ => None,
        }
    }

    /// Returns the value of the input.
    ///
    /// A masked input returns its masked value, the same as what is rendered.
    pub fn value(&self, cx: &App) -> SharedString {
        match self {
            Self::Input(state) => state.read(cx).value(),
            Self::Textarea(state) => state.read(cx).value(),
            Self::Editor(state) => state.read(cx).value(),
            Self::Otp(state) => state.read(cx).value().clone(),
        }
    }

    /// Returns the focus handle of the input.
    pub fn focus_handle(&self, cx: &App) -> FocusHandle {
        match self {
            Self::Input(state) => state.focus_handle(cx),
            Self::Textarea(state) => state.focus_handle(cx),
            Self::Editor(state) => state.focus_handle(cx),
            Self::Otp(state) => state.focus_handle(cx),
        }
    }
}

impl From<Entity<InputState>> for AnyInputState {
    fn from(state: Entity<InputState>) -> Self {
        Self::Input(state)
    }
}

impl From<Entity<TextareaState>> for AnyInputState {
    fn from(state: Entity<TextareaState>) -> Self {
        Self::Textarea(state)
    }
}

impl From<Entity<EditorState>> for AnyInputState {
    fn from(state: Entity<EditorState>) -> Self {
        Self::Editor(state)
    }
}

impl From<Entity<OtpState>> for AnyInputState {
    fn from(state: Entity<OtpState>) -> Self {
        Self::Otp(state)
    }
}

/// Registers `state` as the window's focused input while it holds focus, and
/// unregisters it once focus moves elsewhere.
pub(super) fn sync_focused_input_registry(
    state: impl Into<AnyInputState>,
    window: &mut Window,
    cx: &mut App,
) {
    let state = state.into();
    let focused = state.focus_handle(cx).is_focused(window);
    Root::try_update(window, cx, |root, _, cx| {
        if focused {
            root.focused_input = Some(state.clone());
        } else if root.focused_input.as_ref() == Some(&state) {
            root.focused_input = None;
        }
        cx.notify();
    });
}
