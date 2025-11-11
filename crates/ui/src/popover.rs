use gpui::{
    anchored, canvas, deferred, div, prelude::FluentBuilder as _, px, AnyElement, App, Bounds,
    Context, Corner, DismissEvent, ElementId, EventEmitter, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement, Pixels, Point,
    Render, RenderOnce, StyleRefinement, Styled, Subscription, Window,
};
use std::rc::Rc;

use crate::{actions::Cancel, v_flex, Selectable, StyledExt as _};

const CONTEXT: &str = "Popover";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some(CONTEXT))])
}

/// A popover element that can be triggered by a button or any other element.
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    style: StyleRefinement,
    anchor: Corner,
    tracked_focus_handle: Option<FocusHandle>,
    trigger: Option<Box<dyn FnOnce(bool, &Window, &App) -> AnyElement + 'static>>,
    content: Option<
        Rc<
            dyn Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> AnyElement
                + 'static,
        >,
    >,
    children: Vec<AnyElement>,
    /// Style for trigger element.
    /// This is used for hotfix the trigger element style to support w_full.
    trigger_style: Option<StyleRefinement>,
    mouse_button: MouseButton,
    appearance: bool,
}

impl Popover {
    /// Create a new Popover with `view` mode.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            anchor: Corner::TopLeft,
            trigger: None,
            trigger_style: None,
            content: None,
            tracked_focus_handle: None,
            children: vec![],
            mouse_button: MouseButton::Left,
            appearance: true,
        }
    }

    /// Set the anchor corner of the popover, default is `Corner::TopLeft`.
    pub fn anchor(mut self, anchor: Corner) -> Self {
        self.anchor = anchor;
        self
    }

    /// Set the mouse button to trigger the popover, default is `MouseButton::Left`.
    pub fn mouse_button(mut self, mouse_button: MouseButton) -> Self {
        self.mouse_button = mouse_button;
        self
    }

    /// Set the trigger element of the popover.
    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: Selectable + IntoElement + 'static,
    {
        self.trigger = Some(Box::new(|is_open, _, _| {
            let selected = trigger.is_selected();
            trigger.selected(selected || is_open).into_any_element()
        }));
        self
    }

    /// Set the style for the trigger element.
    pub fn trigger_style(mut self, style: StyleRefinement) -> Self {
        self.trigger_style = Some(style);
        self
    }

    /// Set the content of the popover.
    pub fn content<F, E>(mut self, content: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> E + 'static,
    {
        self.content = Some(Rc::new(move |state, window, cx| {
            content(state, window, cx).into_any_element()
        }));
        self
    }

    /// Set whether the popover no style, default is `false`.
    ///
    /// If no style:
    ///
    /// - The popover will not have a bg, border, shadow, or padding.
    /// - The click out of the popover will not dismiss it.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Bind the focus handle to track focus inside the popover.
    ///
    /// If popover is opened, the focus will be moved to the focus handle.
    pub fn track_focus(mut self, handle: &FocusHandle) -> Self {
        self.tracked_focus_handle = Some(handle.clone());
        self
    }

    fn resolved_corner(anchor: Corner, bounds: Bounds<Pixels>) -> Point<Pixels> {
        bounds.corner(match anchor {
            Corner::TopLeft => Corner::BottomLeft,
            Corner::TopRight => Corner::BottomRight,
            Corner::BottomLeft => Corner::TopLeft,
            Corner::BottomRight => Corner::TopRight,
        }) + Point {
            x: px(0.),
            y: -bounds.size.height,
        }
    }
}

impl ParentElement for Popover {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Popover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

pub struct PopoverState {
    focus_handle: FocusHandle,
    pub(crate) tracked_focus_handle: Option<FocusHandle>,
    trigger_bounds: Option<Bounds<Pixels>>,
    previous_focus: Option<FocusHandle>,
    open: bool,

    _dismiss_subscription: Option<Subscription>,
}

impl PopoverState {
    pub fn new(cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            tracked_focus_handle: None,
            trigger_bounds: None,
            previous_focus: None,
            open: false,
            _dismiss_subscription: None,
        }
    }

    /// Check if the popover is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Dismiss the popover if it is open.
    pub fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.toggle_open(window, cx);
        }
    }

    /// Open the popover if it is closed.
    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.toggle_open(window, cx);
        }
    }

    fn toggle_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        if self.open {
            let state = cx.entity();
            self.previous_focus = window.focused(cx);
            self.focus_handle(cx).focus(window);

            self._dismiss_subscription =
                Some(
                    window.subscribe(&cx.entity(), cx, move |_, _: &DismissEvent, window, cx| {
                        state.update(cx, |state, cx| {
                            state.dismiss(window, cx);
                        });
                        window.refresh();
                    }),
                );
        } else {
            if let Some(previous_focus) = self.previous_focus.take() {
                window.focus(&previous_focus);
            }
            self._dismiss_subscription = None;
        }
        cx.notify();
        window.refresh();
    }

    fn on_action_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss(window, cx);
    }
}

impl Focusable for PopoverState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        if let Some(tracked_focus_handle) = &self.tracked_focus_handle {
            tracked_focus_handle.clone()
        } else {
            self.focus_handle.clone()
        }
    }
}

impl Render for PopoverState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl EventEmitter<DismissEvent> for PopoverState {}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| PopoverState::new(cx));
        if let Some(tracked_focus_handle) = self.tracked_focus_handle.clone() {
            state.update(cx, |state, _| {
                state.tracked_focus_handle = Some(tracked_focus_handle);
            })
        }

        let open = state.read(cx).open;
        let focus_handle = state.read(cx).focus_handle.clone();
        let trigger_bounds = state.read(cx).trigger_bounds;

        let Some(trigger) = self.trigger else {
            return div().id("empty");
        };

        let parent_view_id = window.current_view();

        let el = div()
            .id(self.id)
            .child((trigger)(open, window, cx))
            .on_mouse_down(self.mouse_button, {
                let state = state.clone();
                move |_, window, cx| {
                    state.update(cx, |state, cx| {
                        state.toggle_open(window, cx);
                    });
                    cx.notify(parent_view_id);
                }
            })
            .child(
                canvas(
                    {
                        let state = state.clone();
                        move |bounds, _, cx| {
                            state.update(cx, |state, _| {
                                state.trigger_bounds = Some(bounds);
                            })
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            );

        if !open {
            return el;
        }

        el.child(
            deferred(
                anchored()
                    .snap_to_window_with_margin(px(8.))
                    .anchor(self.anchor)
                    .when_some(trigger_bounds, |this, trigger_bounds| {
                        this.position(Self::resolved_corner(self.anchor, trigger_bounds))
                    })
                    .child(
                        v_flex()
                            .id("content")
                            .key_context(CONTEXT)
                            .track_focus(&focus_handle)
                            .on_action(window.listener_for(&state, PopoverState::on_action_cancel))
                            .size_full()
                            .occlude()
                            .tab_group()
                            .when(self.appearance, |this| this.popover_style(cx).p_4())
                            .map(|this| match self.anchor {
                                Corner::TopLeft | Corner::TopRight => this.top_1(),
                                Corner::BottomLeft | Corner::BottomRight => this.bottom_1(),
                            })
                            .when_some(self.content, |this, content| {
                                this.child(
                                    state.update(cx, |state, cx| (content)(state, window, cx)),
                                )
                            })
                            .children(self.children)
                            .when(self.appearance, |this| {
                                let state = state.clone();
                                this.on_mouse_down_out(move |_, window, cx| {
                                    state.update(cx, |state, cx| {
                                        state.toggle_open(window, cx);
                                    });
                                    cx.notify(parent_view_id);
                                })
                            })
                            .on_mouse_down_out({
                                let state = state.clone();
                                move |_, window, cx| {
                                    state.update(cx, |state, cx| {
                                        state.dismiss(window, cx);
                                    });
                                    cx.notify(parent_view_id);
                                }
                            })
                            .refine_style(&self.style),
                    ),
            )
            .with_priority(1),
        )
    }
}
