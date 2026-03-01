use gpui::{App, AppContext as _, Context, Entity, IntoElement, Render, Styled, Window};

use gpui_component::input::*;

const EXAMPLE_CODE: &str = include_str!("./editor_story.rs");

pub struct EditorStory {
    editor_state: Entity<InputState>,
}

impl super::Story for EditorStory {
    fn title() -> &'static str {
        "Editor"
    }

    fn description() -> &'static str {
        "Code editor with syntax highlighting by tree-sitter."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl EditorStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("rust")
                .multi_line(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    ..Default::default()
                })
                .default_value(EXAMPLE_CODE)
        });

        Self { editor_state }
    }
}

impl Render for EditorStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.editor_state).size_full()
    }
}
