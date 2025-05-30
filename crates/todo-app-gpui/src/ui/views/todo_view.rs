use gpui::*;

pub struct TodoView {
    root: SharedString,
}

impl TodoView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            root: SharedString::new("hello todo"),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for TodoView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().p_4().size_full().child(self.root.clone())
    }
}
