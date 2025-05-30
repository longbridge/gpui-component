use gpui::*;
use gpui_component::{input::TextInput, *};

pub struct TodoFormView {
    title: String,
    completed: bool,
}

impl TodoFormView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            title: String::new(),
            completed: false,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for TodoFormView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().p_4().child("")
    }
}
