use gpui::*;
use gpui::{div, AnyElement, App, Context, IntoElement, MouseButton, Render, SharedString, Window};
use gpui_component::{scroll::ScrollbarShow, Theme, TitleBar};
use std::rc::Rc;

pub struct NormalTitleBar {
    title: SharedString,
    child: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
}

impl NormalTitleBar {
    pub fn new(
        title: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        if cx.should_auto_hide_scrollbars() {
            Theme::global_mut(cx).scrollbar_show = ScrollbarShow::Scrolling;
        } else {
            Theme::global_mut(cx).scrollbar_show = ScrollbarShow::Hover;
        }

        Self {
            title: title.into(),
            child: Rc::new(|_, _| div().into_any_element()),
        }
    }

    pub fn child<F, E>(mut self, f: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.child = Rc::new(move |window, cx| f(window, cx).into_any_element());
        self
    }
}

impl Render for NormalTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            // left side
            .child(self.title.clone())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_2()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child((self.child.clone())(window, cx)),
            )
    }
}
