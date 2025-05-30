use super::views::todolist::TodoList;
use gpui::*;

pub struct TodoMainView {
    root: Entity<TodoList>,
}

impl TodoMainView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let root = TodoList::view(window, cx);

        Self { root }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for TodoMainView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().p_4().size_full().child(self.root.clone())
    }
}
