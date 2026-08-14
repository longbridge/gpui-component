use std::rc::Rc;

use gpui::{
    Anchor, AnyElement, App, Context, ElementId, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, RenderOnce, Stateful, StatefulInteractiveElement as _, Task,
    Window, div, prelude::FluentBuilder as _,
};
use instant::Duration;

use crate::Popup;

type ContentBuilder = Box<
    dyn FnOnce(
        &mut HoverCardState,
        &mut Window,
        &mut Context<HoverCardState>,
    ) -> Stateful<gpui::Div>,
>;
type OpenChangeHandler = Rc<dyn Fn(&bool, &mut Window, &mut App)>;

/// An unstyled hover-triggered popup with delayed open and close behavior.
#[derive(IntoElement)]
pub struct HoverCard {
    id: ElementId,
    anchor: Anchor,
    trigger: Option<AnyElement>,
    content: Option<ContentBuilder>,
    open_delay: Duration,
    close_delay: Duration,
    on_open_change: Option<OpenChangeHandler>,
}

impl HoverCard {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            anchor: Anchor::TopCenter,
            trigger: None,
            content: None,
            open_delay: Duration::from_secs_f64(0.6),
            close_delay: Duration::from_secs_f64(0.3),
            on_open_change: None,
        }
    }

    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    pub fn content<F>(mut self, content: F) -> Self
    where
        F: FnOnce(
                &mut HoverCardState,
                &mut Window,
                &mut Context<HoverCardState>,
            ) -> Stateful<gpui::Div>
            + 'static,
    {
        self.content = Some(Box::new(content));
        self
    }

    pub fn open_delay(mut self, duration: Duration) -> Self {
        self.open_delay = duration;
        self
    }

    pub fn close_delay(mut self, duration: Duration) -> Self {
        self.close_delay = duration;
        self
    }

    pub fn on_open_change(
        mut self,
        callback: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(callback));
        self
    }
}

/// State exposed to a [`HoverCard::content`] builder.
pub struct HoverCardState {
    open: bool,
    open_delay: Duration,
    close_delay: Duration,
    open_task: Option<Task<()>>,
    close_task: Option<Task<()>>,
    epoch: usize,
    is_hovering_trigger: bool,
    is_hovering_content: bool,
}

impl HoverCardState {
    fn new(open_delay: Duration, close_delay: Duration) -> Self {
        Self {
            open: false,
            open_delay,
            close_delay,
            open_task: None,
            close_task: None,
            epoch: 0,
            is_hovering_trigger: false,
            is_hovering_content: false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    fn sync(&mut self, open_delay: Duration, close_delay: Duration) {
        self.open_delay = open_delay;
        self.close_delay = close_delay;
    }

    fn schedule_open(&mut self, cx: &mut Context<Self>) {
        self.cancel_tasks();
        let epoch = self.next_epoch();
        let delay = self.open_delay;
        self.open_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |state, cx| {
                if state.epoch == epoch {
                    state.set_open(true, cx);
                }
            });
        }));
    }

    fn schedule_close(&mut self, cx: &mut Context<Self>) {
        self.cancel_tasks();
        let epoch = self.next_epoch();
        let delay = self.close_delay;
        self.close_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |state, cx| {
                if state.epoch == epoch && !state.is_hovering_trigger && !state.is_hovering_content
                {
                    state.set_open(false, cx);
                }
            });
        }));
    }

    fn cancel_tasks(&mut self) {
        self.epoch += 1;
        self.open_task = None;
        self.close_task = None;
    }

    fn next_epoch(&mut self) -> usize {
        self.epoch += 1;
        self.epoch
    }

    fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.open != open {
            self.open = open;
            cx.notify();
        }
    }

    fn on_trigger_hover(&mut self, hovering: bool, cx: &mut Context<Self>) {
        self.is_hovering_trigger = hovering;
        if hovering {
            self.schedule_open(cx);
        } else if !self.is_hovering_content {
            self.schedule_close(cx);
        }
    }

    fn on_content_hover(&mut self, hovering: bool, cx: &mut Context<Self>) {
        self.is_hovering_content = hovering;
        if hovering {
            self.cancel_tasks();
        } else if !self.is_hovering_trigger {
            self.schedule_close(cx);
        }
    }
}

impl Render for HoverCardState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl RenderOnce for HoverCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| {
            HoverCardState::new(self.open_delay, self.close_delay)
        });
        let was_open = state.read(cx).is_open();
        state.update(cx, |state, _| state.sync(self.open_delay, self.close_delay));
        let open = state.read(cx).is_open();
        if was_open != open {
            if let Some(callback) = &self.on_open_change {
                callback(&open, window, cx);
            }
        }

        let trigger = self.trigger.unwrap_or_else(|| div().into_any_element());
        let popup = Popup::new(
            self.id,
            div().id("trigger").child(trigger).on_hover(
                window.listener_for(&state, |state, hovered, _, cx| {
                    state.on_trigger_hover(*hovered, cx)
                }),
            ),
        )
        .anchor(self.anchor);

        if !open {
            return popup;
        }

        popup.when_some(self.content, |popup, content| {
            let hover = window.listener_for(&state, |state, hovered, _, cx| {
                state.on_content_hover(*hovered, cx)
            });
            popup.content(state.update(cx, |state, cx| content(state, window, cx).on_hover(hover)))
        })
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Context, InteractiveElement as _, Render, Styled as _, TestAppContext, point, px};

    use super::*;

    struct Harness;

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let delay = Duration::from_millis(100);
            HoverCard::new("hover-card")
                .open_delay(delay)
                .close_delay(delay)
                .trigger(
                    div()
                        .debug_selector(|| "hover-card-trigger".into())
                        .size(px(20.)),
                )
                .content(|_, _, _| {
                    div()
                        .id("hover-card-content")
                        .debug_selector(|| "hover-card-content".into())
                        .size(px(10.))
                })
        }
    }

    #[gpui::test]
    fn public_hover_card_owns_delayed_open_and_close(cx: &mut TestAppContext) {
        let delay = Duration::from_millis(100);
        let (_, cx) = cx.add_window_view(|_, _| Harness);
        cx.update(|window, cx| window.draw(cx).clear(cx));

        cx.simulate_mouse_move(point(px(10.), px(10.)), None, gpui::Modifiers::default());
        cx.executor().advance_clock(delay);
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.draw(cx).clear(cx);
        });
        assert!(cx.debug_bounds("hover-card-content").is_some());

        cx.simulate_mouse_move(point(px(100.), px(100.)), None, gpui::Modifiers::default());
        cx.executor().advance_clock(delay);
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("hover-card-content").is_none());
    }
}
