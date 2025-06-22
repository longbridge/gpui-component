use crate::app::Open;

use super::views::todolist::TodoList;
use gpui::*;

pub struct TodoMainWindow {
    root: Entity<TodoList>,
}

impl TodoMainWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.on_window_should_close(cx, |win,app|{
            app.quit();
            true
        });
       cx.spawn_in(window, async move |win,app|{
        app.update(|win,app|{
            app.dispatch_action(&Open);
        }).ok();
       }).detach();
        let root = TodoList::view(window, cx);

        Self { root }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for TodoMainWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().p_4().size_full().child(self.root.clone())
    }
}
