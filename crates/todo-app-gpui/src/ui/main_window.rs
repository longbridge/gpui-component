use std::time::Duration;

use crate::{
    app::Open,
    backoffice::{cross_runtime::CrossRuntimeBridge, BoEvent},
};

use super::views::todolist::TodoList;
use gpui::*;
use gpui_component::ContextModal;

pub struct TodoMainWindow {
    root: Entity<TodoList>,
}

impl TodoMainWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.on_window_should_close(cx, |_win, app| {
            app.quit();
            true
        });
        cx.spawn_in(window, async move |_this, app: &mut AsyncWindowContext| {
            app.update(|_win, app| {
                app.dispatch_action(&Open);
            })
            .ok();
        })
        .detach();
        let root = TodoList::view(window, cx);
        let win_handle = window.window_handle();
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let subscription = CrossRuntimeBridge::global().subscribe(move |event: &BoEvent| {
            if event.is_notification() {
                tx.try_send(event.clone()).ok();
            }
        });
        cx.spawn(async move |_this, cx| {
            use tokio::sync::mpsc::error::TryRecvError;
            let _sub = subscription;
            let handle = win_handle;
            loop {
                Timer::after(Duration::from_millis(100)).await;
                match rx.try_recv() {
                    Ok(note) => {
                        let note = match note.to_notification() {
                            Some(n) => n,
                            None => continue,
                        };
                        handle
                            .update(cx, |_this, window, cx| {
                                window.push_notification(note, cx);
                            })
                            .ok();
                    }
                    Err(TryRecvError::Empty) => continue,
                    Err(TryRecvError::Disconnected) => break,
                }
            }
        })
        .detach();
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
