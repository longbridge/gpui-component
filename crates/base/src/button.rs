use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, FocusHandle, InteractiveElement, Interactivity,
    IntoElement, MouseButton, ParentElement, Refineable as _, RenderOnce, Role, SharedString,
    Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _,
};
use smallvec::SmallVec;

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// An unstyled button that owns interaction, focus, keyboard, and accessibility behavior.
///
/// Layout and visual states are intentionally supplied by the application through
/// GPUI's [`Styled`] API.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    disabled: bool,
    children: SmallVec<[AnyElement; 2]>,
    on_click: Option<ClickHandler>,
    accessibility_label: Option<SharedString>,
    role: Role,
    toggled: Option<bool>,
    focus_enabled: bool,
    provided_focus_handle: Option<FocusHandle>,
    tab_index: isize,
    tab_stop: bool,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()),
            id,
            style: StyleRefinement::default(),
            disabled: false,
            children: SmallVec::new(),
            on_click: None,
            accessibility_label: None,
            role: Role::Button,
            toggled: None,
            focus_enabled: true,
            provided_focus_handle: None,
            tab_index: 0,
            tab_stop: true,
        }
    }

    /// Builds button behavior around an existing stateful div.
    ///
    /// This is used by styled adapters that must preserve their existing
    /// element identity, style ordering, and specialized pointer handlers.
    pub fn from_stateful(id: impl Into<ElementId>, base: Stateful<Div>) -> Self {
        let mut this = Self::new(id);
        this.base = base;
        this
    }

    /// Sets whether the button ignores pointer and keyboard activation.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the label exposed to accessibility clients.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Overrides the accessibility role. The default is [`Role::Button`].
    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    /// Exposes an optional toggle-button state to accessibility clients.
    pub fn toggled(mut self, toggled: Option<bool>) -> Self {
        self.toggled = toggled;
        self
    }

    /// Controls whether this element tracks a focus handle.
    pub fn focus_enabled(mut self, enabled: bool) -> Self {
        self.focus_enabled = enabled;
        self
    }

    /// Uses a caller-owned focus handle instead of creating keyed state.
    pub fn with_focus_handle(mut self, focus_handle: &FocusHandle) -> Self {
        self.provided_focus_handle = Some(focus_handle.clone());
        self
    }

    /// Sets the activation handler for pointer, Enter, and Space input.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Sets the focus traversal index. The default is `0`.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Sets whether the button participates in keyboard focus traversal.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    fn focus_handle(&self, window: &mut Window, cx: &mut App) -> FocusHandle {
        self.provided_focus_handle.clone().unwrap_or_else(|| {
            window
                .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
                .read(cx)
                .clone()
        })
    }

    /// Applies behavior to the underlying stateful div without introducing a
    /// wrapper element.
    pub fn into_stateful(self, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        let focus_handle = self.focus_handle(window, cx);
        let disabled = self.disabled;
        let style = self.style;
        let on_click = self.on_click;

        self.base
            .role(self.role)
            .when_some(self.accessibility_label, |this, label| {
                this.aria_label(label)
            })
            .when_some(self.toggled, |this, toggled| {
                this.aria_toggled(if toggled {
                    gpui::accesskit::Toggled::True
                } else {
                    gpui::accesskit::Toggled::False
                })
            })
            .when(self.focus_enabled && !disabled, |this| {
                this.track_focus(
                    &focus_handle
                        .tab_index(self.tab_index)
                        .tab_stop(self.tab_stop),
                )
            })
            .when(disabled, |this| {
                this.on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
            })
            .when_some(
                (!disabled).then_some(on_click).flatten(),
                |this, on_click| {
                    this.on_click(move |event, window, cx| {
                        on_click(event, window, cx);
                    })
                },
            )
            .children(self.children)
            .map(|mut this| {
                this.style().refine(&style);
                this
            })
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Button {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Button {}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.into_stateful(window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use gpui::{
        ClickEvent, Context, Element as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render,
        TestAppContext, VisualTestContext, accesskit, canvas, point, px,
    };

    struct ButtonHarness {
        disabled: bool,
        button_clicks: Rc<Cell<usize>>,
        parent_clicks: Rc<Cell<usize>>,
        keyboard_events: Rc<Cell<usize>>,
    }

    impl Render for ButtonHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let button_clicks = self.button_clicks.clone();
            let keyboard_events = self.keyboard_events.clone();
            let parent_clicks = self.parent_clicks.clone();

            div()
                .id("button-parent")
                .tab_group()
                .size(px(100.))
                .on_click(move |_, _, _| parent_clicks.set(parent_clicks.get() + 1))
                .child(
                    Button::new("button")
                        .disabled(self.disabled)
                        .size_full()
                        .on_click(move |event, _, _| {
                            button_clicks.set(button_clicks.get() + 1);
                            if matches!(event, ClickEvent::Keyboard(_)) {
                                keyboard_events.set(keyboard_events.get() + 1);
                            }
                        }),
                )
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        disabled: bool,
    ) -> (
        &mut VisualTestContext,
        Rc<Cell<usize>>,
        Rc<Cell<usize>>,
        Rc<Cell<usize>>,
    ) {
        let button_clicks = Rc::new(Cell::new(0));
        let parent_clicks = Rc::new(Cell::new(0));
        let keyboard_events = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let button_clicks = button_clicks.clone();
            let parent_clicks = parent_clicks.clone();
            let keyboard_events = keyboard_events.clone();
            move |_, _| ButtonHarness {
                disabled,
                button_clicks,
                parent_clicks,
                keyboard_events,
            }
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        (cx, button_clicks, parent_clicks, keyboard_events)
    }

    #[gpui::test]
    fn pointer_activation_fires_once(cx: &mut TestAppContext) {
        let (cx, button_clicks, _, keyboard_events) = harness(cx, false);

        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());

        assert_eq!(button_clicks.get(), 1);
        assert_eq!(keyboard_events.get(), 0);
    }

    #[gpui::test]
    fn enter_and_space_use_one_native_keyboard_click_each(cx: &mut TestAppContext) {
        let (cx, button_clicks, _, keyboard_events) = harness(cx, false);
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        button_clicks.set(0);
        cx.update(|window, cx| {
            assert!(window.focused(cx).is_some());
            window.draw(cx).clear(cx);
        });

        for key in ["enter", "space"] {
            let keystroke = Keystroke::parse(key).unwrap();
            cx.simulate_event(KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            });
            cx.simulate_event(KeyUpEvent { keystroke });
        }

        assert_eq!(button_clicks.get(), 2);
        assert_eq!(keyboard_events.get(), 2);
    }

    #[gpui::test]
    fn disabled_button_is_inert_and_blocks_parent_activation(cx: &mut TestAppContext) {
        let (cx, button_clicks, parent_clicks, _) = harness(cx, true);

        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        cx.update(|window, cx| window.focus_next(cx));
        cx.simulate_keystrokes("enter space");

        assert_eq!(button_clicks.get(), 0);
        assert_eq!(parent_clicks.get(), 0);
    }

    #[test]
    fn state_styling_methods_are_available_to_applications() {
        let _ = Button::new("states")
            .hover(|style| style.opacity(0.9))
            .active(|style| style.opacity(0.8))
            .focus_visible(|style| style.opacity(0.7));
    }

    #[gpui::test]
    fn accessibility_role_label_and_disabled_action_surface(cx: &mut TestAppContext) {
        type Captured = Arc<Mutex<Option<(accesskit::Node, accesskit::Node)>>>;

        struct A11yProbe {
            captured: Captured,
        }

        impl Render for A11yProbe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let captured = self.captured.clone();
                canvas(
                    move |_, window, cx| {
                        let mut info = |button: Button| {
                            let mut node = accesskit::Node::new(Role::Button);
                            button
                                .render(window, cx)
                                .into_element()
                                .write_a11y_info(&mut node);
                            node
                        };
                        let enabled = info(
                            Button::new("enabled")
                                .accessibility_label("Save")
                                .on_click(|_, _, _| {}),
                        );
                        let disabled = info(
                            Button::new("disabled")
                                .disabled(true)
                                .accessibility_label("Save")
                                .on_click(|_, _, _| {}),
                        );
                        *captured.lock().unwrap() = Some((enabled, disabled));
                    },
                    |_, _, _, _| {},
                )
            }
        }

        let captured: Captured = Arc::new(Mutex::new(None));
        let result = captured.clone();
        let (_, cx) = cx.add_window_view(move |_, _| A11yProbe { captured });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        let (enabled, disabled) = result.lock().unwrap().take().unwrap();
        assert_eq!(enabled.role(), Role::Button);
        assert_eq!(enabled.label(), Some("Save"));
        assert!(enabled.supports_action(accesskit::Action::Click));

        assert_eq!(disabled.role(), Role::Button);
        assert!(!disabled.supports_action(accesskit::Action::Click));

        // GPUI's current StatefulInteractiveElement interface has no
        // aria-disabled setter even though AccessKit can represent it. This
        // assertion records that upstream gap instead of claiming support.
        assert!(!disabled.is_disabled());
    }
}
