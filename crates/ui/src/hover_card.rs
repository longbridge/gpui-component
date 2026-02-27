use gpui::{
    AnyElement, App, Bounds, Context, ElementId, InteractiveElement as _, IntoElement,
    ParentElement, Pixels, Render, RenderOnce, StatefulInteractiveElement, StyleRefinement, Styled,
    Task, Window, div, prelude::FluentBuilder as _,
};
use std::rc::Rc;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(target_arch = "wasm32")]
use web_time::Duration;

use crate::{Anchor, ElementExt, StyledExt as _, popover::Popover};

/// A hover card element that displays content when hovering over a trigger element.
///
/// Similar to Popover but triggered by mouse hover instead of click, with configurable delays
/// for showing and hiding the content.
#[derive(IntoElement)]
pub struct HoverCard {
    id: ElementId,
    style: StyleRefinement,
    anchor: Anchor,
    trigger: Option<Box<dyn FnOnce(&mut Window, &App) -> AnyElement + 'static>>,
    content: Option<
        Rc<
            dyn Fn(&mut HoverCardState, &mut Window, &mut Context<HoverCardState>) -> AnyElement
                + 'static,
        >,
    >,
    children: Vec<AnyElement>,
    open_delay: Duration,
    close_delay: Duration,
    appearance: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl HoverCard {
    /// Create a new HoverCard.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            anchor: Anchor::TopCenter,
            trigger: None,
            content: None,
            children: vec![],
            open_delay: Duration::from_secs_f64(0.6),
            close_delay: Duration::from_secs_f64(0.3),
            appearance: true,
            on_open_change: None,
        }
    }

    /// Set the anchor corner of the hover card, default is [`Anchor::TopCenter`].
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Set the trigger element of the hover card.
    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: IntoElement + 'static,
    {
        self.trigger = Some(Box::new(|_, _| trigger.into_any_element()));
        self
    }

    /// Set the content builder of the hover card.
    ///
    /// The builder function receives the HoverCardState, Window, and Context as parameters.
    pub fn content<F, E>(mut self, content: F) -> Self
    where
        F: Fn(&mut HoverCardState, &mut Window, &mut Context<HoverCardState>) -> E + 'static,
        E: IntoElement + 'static,
    {
        self.content = Some(Rc::new(move |state, window, cx| {
            content(state, window, cx).into_any_element()
        }));
        self
    }

    /// Set the delay before showing the hover card in milliseconds, default is 600ms.
    pub fn open_delay(mut self, duration: Duration) -> Self {
        self.open_delay = duration;
        self
    }

    /// Set the delay before hiding the hover card in milliseconds, default is 300ms.
    pub fn close_delay(mut self, duration: Duration) -> Self {
        self.close_delay = duration;
        self
    }

    /// Set whether to apply default appearance styles, default is `true`.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Set a callback to be called when the open state changes.
    pub fn on_open_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_open_change = Some(Rc::new(callback));
        self
    }
}

impl Styled for HoverCard {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for HoverCard {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// State management for HoverCard component.
pub struct HoverCardState {
    open: bool,
    trigger_bounds: Bounds<Pixels>,
    open_delay: Duration,
    close_delay: Duration,

    // Timer management
    open_task: Option<Task<()>>,
    close_task: Option<Task<()>>,
    epoch: usize, // Used to cancel stale timers

    // Hover state tracking
    is_hovering_trigger: bool,
    is_hovering_content: bool,

    // Callbacks
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl HoverCardState {
    fn new(open_delay: Duration, close_delay: Duration) -> Self {
        Self {
            open: false,
            trigger_bounds: Bounds::default(),
            open_delay,
            close_delay,
            open_task: None,
            close_task: None,
            epoch: 0,
            is_hovering_trigger: false,
            is_hovering_content: false,
            on_open_change: None,
        }
    }

    /// Check if the hover card is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Schedule opening the hover card after the configured delay.
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

    /// Schedule closing the hover card after the configured delay.
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
        self.epoch += 1; // Invalidate all pending timers
        self.open_task = None;
        self.close_task = None;
    }

    fn next_epoch(&mut self) -> usize {
        self.epoch += 1;
        self.epoch
    }

    fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.open == open {
            return;
        }

        self.open = open;
        cx.notify();
    }

    /// Handle hover state change on the trigger element.
    fn on_trigger_hover(&mut self, hovering: bool, cx: &mut Context<Self>) {
        self.is_hovering_trigger = hovering;

        if hovering {
            self.schedule_open(cx);
        } else {
            // Only close if not hovering content
            if !self.is_hovering_content {
                self.schedule_close(cx);
            }
        }
    }

    /// Handle hover state change on the content element.
    fn on_content_hover(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.is_hovering_content = hovered;

        if hovered {
            self.cancel_tasks();
        } else {
            // Only close if not hovering trigger
            if !self.is_hovering_trigger {
                self.schedule_close(cx);
            }
        }
    }
}

impl Render for HoverCardState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div() // Empty render
    }
}

impl RenderOnce for HoverCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| {
            HoverCardState::new(self.open_delay, self.close_delay)
        });

        // Update state and track if controlled mode changed the open state
        let prev_open = state.read(cx).open;
        state.update(cx, |state, _| {
            state.open_delay = self.open_delay;
            state.close_delay = self.close_delay;
            state.on_open_change = self.on_open_change.clone();
        });

        let open = state.read(cx).open;
        let trigger_bounds = state.read(cx).trigger_bounds;

        // Trigger callback if state changed in controlled mode
        if prev_open != open {
            if let Some(ref callback) = self.on_open_change {
                callback(&open, window, cx);
            }
        }

        let Some(trigger) = self.trigger else {
            return div().id("empty");
        };

        let root = div().id(self.id).child(
            div()
                .id("trigger")
                .child((trigger)(window, cx))
                .on_hover(window.listener_for(&state, |state, hovered, _, cx| {
                    state.on_trigger_hover(*hovered, cx);
                }))
                .on_prepaint({
                    let state = state.clone();
                    move |bounds, _, cx| {
                        state.update(cx, |state, _| {
                            state.trigger_bounds = bounds;
                        });
                    }
                }),
        );

        if !open {
            return root;
        }

        let popover_content =
            Popover::render_popover_content(self.anchor, self.appearance, window, cx)
                .overflow_hidden()
                .on_hover(window.listener_for(&state, |state, hovered, _, cx| {
                    state.on_content_hover(*hovered, cx);
                }))
                .when_some(self.content, |this, content| {
                    this.child(state.update(cx, |state, cx| (content)(state, window, cx)))
                })
                .children(self.children)
                .refine_style(&self.style);

        root.child(Popover::render_popover(
            self.anchor,
            trigger_bounds,
            popover_content,
            window,
            cx,
        ))
    }
}
