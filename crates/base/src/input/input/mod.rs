use gpui::{App, Entity, IntoElement, RenderOnce, Window};

use super::{InputBaseState, InputMode};

/// State for a single-line text input.
///
/// This is the shared editing engine in its single-line kind. Multi-line
/// layout, auto-grow, and code-editor configuration do not exist on this type —
/// those methods live on [`super::TextareaState`] and [`super::EditorState`].
pub type InputState = InputBaseState<InputMode>;

/// An unstyled single-line text input.
///
/// Applications that need a fully styled control can wrap this state with
/// their own presentation or use `gpui-component::Input`.
#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
}

impl Input {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for Input {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.state
    }
}
