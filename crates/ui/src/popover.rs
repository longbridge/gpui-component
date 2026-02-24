use gpui::{
    AnyElement, App, Bounds, Context, Deferred, DismissEvent, Div, ElementId, EventEmitter,
    FocusHandle, Focusable, Half, InteractiveElement as _, IntoElement, KeyBinding, MouseButton,
    ParentElement, Pixels, Point, Render, RenderOnce, Stateful, StyleRefinement, Styled,
    Subscription, Window, deferred, div, prelude::FluentBuilder as _, px,
};
use std::rc::Rc;

use crate::{
    Anchor, ElementExt, Selectable, StyledExt as _, actions::Cancel, anchored,
    global_state::GlobalState, v_flex,
};

const CONTEXT: &str = "Popover";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some(CONTEXT))])
}

/// A popover element that can be triggered by a button or any other element.
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    style: StyleRefinement,
    anchor: Anchor,
    default_open: bool,
    open: Option<bool>,
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
    overlay_closable: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl Popover {
    /// Create a new Popover with `view` mode.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            anchor: Anchor::TopLeft,
            trigger: None,
            trigger_style: None,
            content: None,
            tracked_focus_handle: None,
            children: vec![],
            mouse_button: MouseButton::Left,
            appearance: true,
            overlay_closable: true,
            default_open: false,
            open: None,
            on_open_change: None,
        }
    }

    /// Set the anchor corner of the popover, default is `Corner::TopLeft`.
    ///
    /// This method is kept for backward compatibility with `Corner` type.
    /// Internally, it converts `Corner` to `Anchor`.
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
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

    /// Set the default open state of the popover, default is `false`.
    ///
    /// This is only used to initialize the open state of the popover.
    ///
    /// And please note that if you use the `open` method, this value will be ignored.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Force set the open state of the popover.
    ///
    /// If this is set, the popover will be controlled by this value.
    ///
    /// NOTE: You must be used in conjunction with `on_open_change` to handle state changes.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Add a callback to be called when the open state changes.
    ///
    /// The first `&bool` parameter is the **new open state**.
    ///
    /// This is useful when using the `open` method to control the popover state.
    pub fn on_open_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    /// Set the style for the trigger element.
    pub fn trigger_style(mut self, style: StyleRefinement) -> Self {
        self.trigger_style = Some(style);
        self
    }

    /// Set whether clicking outside the popover will dismiss it, default is `true`.
    pub fn overlay_closable(mut self, closable: bool) -> Self {
        self.overlay_closable = closable;
        self
    }

    /// Set the content builder for content of the Popover.
    ///
    /// This callback will called every time on render the popover.
    /// So, you should avoid creating new elements or entities in the content closure.
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

    /// Bind the focus handle to receive focus when the popover is opened.
    /// If you not set this, a new focus handle will be created for the popover to
    ///
    /// If popover is opened, the focus will be moved to the focus handle.
    pub fn track_focus(mut self, handle: &FocusHandle) -> Self {
        self.tracked_focus_handle = Some(handle.clone());
        self
    }

    fn resolved_corner(anchor: Anchor, trigger_bounds: Bounds<Pixels>) -> Point<Pixels> {
        let offset = if anchor.is_center() {
            gpui::point(trigger_bounds.size.width.half(), px(0.))
        } else {
            Point::default()
        };

        trigger_bounds.corner(anchor.swap_vertical().into())
            + offset
            + Point {
                x: px(0.),
                y: -trigger_bounds.size.height,
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
    trigger_bounds: Bounds<Pixels>,
    open: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,

    _dismiss_subscription: Option<Subscription>,
}

impl PopoverState {
    pub fn new(default_open: bool, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            tracked_focus_handle: None,
            trigger_bounds: Bounds::default(),
            open: default_open,
            on_open_change: None,
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

    fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        self.open = open;
        if self.open {
            GlobalState::global_mut(cx).register_deferred_popover(&self.focus_handle);
        } else {
            GlobalState::global_mut(cx).unregister_deferred_popover(&self.focus_handle);
        }
    }

    fn toggle_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_open(!self.open, cx);
        if self.open {
            let state = cx.entity();
            let focus_handle = if let Some(tracked_focus_handle) = self.tracked_focus_handle.clone()
            {
                tracked_focus_handle
            } else {
                self.focus_handle.clone()
            };
            focus_handle.focus(window, cx);

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
            self._dismiss_subscription = None;
        }

        if let Some(callback) = self.on_open_change.as_ref() {
            callback(&self.open, window, cx);
        }
        cx.notify();
    }

    fn on_action_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss(window, cx);
    }
}

impl Focusable for PopoverState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopoverState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl EventEmitter<DismissEvent> for PopoverState {}

impl Popover {
    pub(crate) fn render_popover<E>(
        anchor: Anchor,
        trigger_bounds: Bounds<Pixels>,
        content: E,
        _: &mut Window,
        _: &mut App,
    ) -> Deferred
    where
        E: IntoElement + 'static,
    {
        deferred(
            anchored()
                .snap_to_window_with_margin(px(8.))
                .anchor(anchor)
                .position(Self::resolved_corner(anchor, trigger_bounds))
                .child(div().relative().child(content)),
        )
        .with_priority(1)
    }

    pub(crate) fn render_popover_content(
        anchor: Anchor,
        appearance: bool,
        _: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        v_flex()
            .id("content")
            .occlude()
            .tab_group()
            .when(appearance, |this| this.popover_style(cx).p_3())
            .map(|this| match anchor {
                Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => this.top_1(),
                Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => this.bottom_1(),
            })
    }
}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let force_open = self.open;
        let default_open = self.default_open;
        let tracked_focus_handle = self.tracked_focus_handle.clone();
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| {
            PopoverState::new(default_open, cx)
        });

        state.update(cx, |state, cx| {
            if let Some(tracked_focus_handle) = tracked_focus_handle {
                state.tracked_focus_handle = Some(tracked_focus_handle);
            }
            state.on_open_change = self.on_open_change.clone();
            if let Some(force_open) = force_open {
                state.set_open(force_open, cx);
            }
        });

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
                    cx.stop_propagation();
                    state.update(cx, |state, cx| {
                        // We force set open to false to toggle it correctly.
                        // Because if the mouse down out will toggle open first.
                        state.set_open(open, cx);
                        state.toggle_open(window, cx);
                    });
                    cx.notify(parent_view_id);
                }
            })
            .on_prepaint({
                let state = state.clone();
                move |bounds, _, cx| {
                    state.update(cx, |state, _| {
                        state.trigger_bounds = bounds;
                    })
                }
            });

        if !open {
            return el;
        }

        let popover_content =
            Self::render_popover_content(self.anchor, self.appearance, window, cx)
                .track_focus(&focus_handle)
                .key_context(CONTEXT)
                .on_action(window.listener_for(&state, PopoverState::on_action_cancel))
                .when_some(self.content, |this, content| {
                    this.child(state.update(cx, |state, cx| (content)(state, window, cx)))
                })
                .children(self.children)
                .when(self.overlay_closable, |this| {
                    this.on_mouse_down_out({
                        let state = state.clone();
                        move |_, window, cx| {
                            state.update(cx, |state, cx| {
                                state.dismiss(window, cx);
                            });
                            cx.notify(parent_view_id);
                        }
                    })
                })
                .refine_style(&self.style);

        el.child(Self::render_popover(
            self.anchor,
            trigger_bounds,
            popover_content,
            window,
            cx,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::MouseButton;

    #[test]
    fn test_popover_builder_chaining() {
        let popover = Popover::new("test")
            .anchor(Anchor::BottomCenter)
            .mouse_button(MouseButton::Right)
            .default_open(true)
            .appearance(false)
            .overlay_closable(false);

        assert_eq!(popover.anchor, Anchor::BottomCenter);
        assert_eq!(popover.mouse_button, MouseButton::Right);
        assert!(popover.default_open);
        assert!(!popover.appearance);
        assert!(!popover.overlay_closable);
    }

    #[test]
    fn test_resolved_corner_top_positions() {
        use gpui::px;

        let bounds = Bounds {
            origin: Point {
                x: px(100.),
                y: px(100.),
            },
            size: gpui::Size {
                width: px(200.),
                height: px(50.),
            },
        };

        let pos = Popover::resolved_corner(Anchor::TopLeft, bounds);
        assert_eq!(pos.x, px(100.));
        assert_eq!(pos.y, px(100.));

        let pos = Popover::resolved_corner(Anchor::TopCenter, bounds);
        assert_eq!(pos.x, px(200.));
        assert_eq!(pos.y, px(100.));

        let pos = Popover::resolved_corner(Anchor::TopRight, bounds);
        assert_eq!(pos.x, px(300.));
        assert_eq!(pos.y, px(100.));

        let pos = Popover::resolved_corner(Anchor::BottomLeft, bounds);
        assert_eq!(pos.x, px(100.));
        assert_eq!(pos.y, px(50.));

        let pos = Popover::resolved_corner(Anchor::BottomCenter, bounds);
        assert_eq!(pos.x, px(200.));
        assert_eq!(pos.y, px(50.));

        let pos = Popover::resolved_corner(Anchor::BottomRight, bounds);
        assert_eq!(pos.x, px(300.));
        assert_eq!(pos.y, px(50.));
    }
}
