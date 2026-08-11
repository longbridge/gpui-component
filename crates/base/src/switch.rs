use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, FocusHandle, InteractiveElement, Interactivity,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, Refineable as _, RenderOnce, Role,
    SharedString, Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Toggled, Window,
    div, prelude::FluentBuilder as _,
};
use smallvec::SmallVec;

type ToggleHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;
type PointerDownToggleHandler = Rc<dyn Fn(bool, &MouseDownEvent, &mut Window, &mut App)>;

/// An unstyled binary control that owns switch interaction and semantics.
///
/// The checked value is controlled by the application. Activation reports the
/// next value through [`Switch::on_toggle`]; the application must render that
/// value back through [`Switch::checked`]. Children and all visual states remain
/// application-owned.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    checked: bool,
    disabled: bool,
    children: SmallVec<[AnyElement; 2]>,
    on_toggle: Option<ToggleHandler>,
    on_pointer_down_toggle: Option<PointerDownToggleHandler>,
    accessibility_label: Option<SharedString>,
    tab_index: isize,
    tab_stop: bool,
    focusable: bool,
    block_pointer_when_disabled: bool,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()),
            id,
            style: StyleRefinement::default(),
            checked: false,
            disabled: false,
            children: SmallVec::new(),
            on_toggle: None,
            on_pointer_down_toggle: None,
            accessibility_label: None,
            tab_index: 0,
            tab_stop: true,
            focusable: true,
            block_pointer_when_disabled: true,
        }
    }

    /// Sets the application-controlled checked value.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Sets whether pointer and keyboard activation are ignored.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Handles activation with the next checked value and its input event.
    pub fn on_toggle(
        mut self,
        handler: impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self.on_pointer_down_toggle = None;
        self
    }

    /// Handles pointer activation on left mouse down instead of click.
    ///
    /// This is useful for controls whose established interaction contract fires
    /// before mouse-up. Calling this replaces an existing [`Self::on_toggle`].
    pub fn on_pointer_down_toggle(
        mut self,
        handler: impl Fn(bool, &MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pointer_down_toggle = Some(Rc::new(handler));
        self.on_toggle = None;
        self
    }

    /// Sets the name exposed to accessibility clients.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Sets the focus traversal index. Use this within a GPUI tab group.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Sets whether the switch participates in keyboard focus traversal.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Sets whether this switch installs a focus handle at all.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// Sets whether disabled pointer presses stop propagation to ancestors.
    pub fn block_pointer_when_disabled(mut self, block: bool) -> Self {
        self.block_pointer_when_disabled = block;
        self
    }

    fn focus_handle(&self, window: &mut Window, cx: &mut App) -> FocusHandle {
        window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone()
    }
}

impl Styled for Switch {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Switch {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Switch {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Switch {}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.focus_handle(window, cx);
        let checked = self.checked;
        let disabled = self.disabled;
        let style = self.style;
        let on_pointer_down_toggle = self.on_pointer_down_toggle;

        self.base
            .role(Role::Switch)
            .aria_toggled(if checked {
                Toggled::True
            } else {
                Toggled::False
            })
            .when_some(self.accessibility_label, |this, label| {
                this.aria_label(label)
            })
            .when(!disabled && self.focusable, |this| {
                this.track_focus(
                    &focus_handle
                        .tab_index(self.tab_index)
                        .tab_stop(self.tab_stop),
                )
            })
            .when(disabled && self.block_pointer_when_disabled, |this| {
                this.on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
            })
            .when_some(
                (!disabled).then_some(on_pointer_down_toggle).flatten(),
                |this, on_pointer_down_toggle| {
                    this.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        on_pointer_down_toggle(!checked, event, window, cx);
                    })
                },
            )
            .when_some(
                (!disabled).then_some(self.on_toggle).flatten(),
                |this, on_toggle| {
                    this.on_click(move |event, window, cx| {
                        on_toggle(!checked, event, window, cx);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use gpui::{
        Context, Element as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render,
        TestAppContext, VisualTestContext, accesskit, canvas, point, px,
    };

    struct SwitchHarness {
        checked: bool,
        disabled: bool,
        toggles: Rc<Cell<usize>>,
        keyboard_events: Rc<Cell<usize>>,
        last_value: Rc<Cell<bool>>,
        parent_clicks: Rc<Cell<usize>>,
    }

    impl Render for SwitchHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let toggles = self.toggles.clone();
            let keyboard_events = self.keyboard_events.clone();
            let last_value = self.last_value.clone();
            let parent_clicks = self.parent_clicks.clone();

            div()
                .id("switch-parent")
                .tab_group()
                .size(px(100.))
                .on_click(move |_, _, _| parent_clicks.set(parent_clicks.get() + 1))
                .child(
                    Switch::new("switch")
                        .checked(self.checked)
                        .disabled(self.disabled)
                        .size_full()
                        .on_toggle(move |value, event, _, _| {
                            toggles.set(toggles.get() + 1);
                            last_value.set(value);
                            if matches!(event, ClickEvent::Keyboard(_)) {
                                keyboard_events.set(keyboard_events.get() + 1);
                            }
                        }),
                )
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        checked: bool,
        disabled: bool,
    ) -> (
        &mut VisualTestContext,
        Rc<Cell<usize>>,
        Rc<Cell<usize>>,
        Rc<Cell<bool>>,
        Rc<Cell<usize>>,
    ) {
        let toggles = Rc::new(Cell::new(0));
        let keyboard_events = Rc::new(Cell::new(0));
        let last_value = Rc::new(Cell::new(checked));
        let parent_clicks = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let toggles = toggles.clone();
            let keyboard_events = keyboard_events.clone();
            let last_value = last_value.clone();
            let parent_clicks = parent_clicks.clone();
            move |_, _| SwitchHarness {
                checked,
                disabled,
                toggles,
                keyboard_events,
                last_value,
                parent_clicks,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, toggles, keyboard_events, last_value, parent_clicks)
    }

    fn activate_key(cx: &mut VisualTestContext, key: &str) {
        let keystroke = Keystroke::parse(key).unwrap();
        cx.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke });
    }

    #[gpui::test]
    fn pointer_reports_the_next_value_once(cx: &mut TestAppContext) {
        let (cx, toggles, keyboard_events, last_value, _) = harness(cx, false, false);

        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());

        assert_eq!(toggles.get(), 1);
        assert_eq!(keyboard_events.get(), 0);
        assert!(last_value.get());
    }

    #[gpui::test]
    fn enter_and_space_report_one_native_keyboard_activation_each(cx: &mut TestAppContext) {
        let (cx, toggles, keyboard_events, last_value, _) = harness(cx, false, false);
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        toggles.set(0);
        cx.update(|window, cx| {
            assert!(window.focused(cx).is_some());
            window.draw(cx).clear(cx);
        });

        activate_key(cx, "enter");
        activate_key(cx, "space");

        assert_eq!(toggles.get(), 2);
        assert_eq!(keyboard_events.get(), 2);
        assert!(last_value.get());
    }

    #[gpui::test]
    fn disabled_switch_is_inert_and_blocks_parent_activation(cx: &mut TestAppContext) {
        let (cx, toggles, _, _, parent_clicks) = harness(cx, false, true);

        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        activate_key(cx, "enter");
        activate_key(cx, "space");

        assert_eq!(toggles.get(), 0);
        assert_eq!(parent_clicks.get(), 0);
    }

    #[gpui::test]
    fn compatibility_options_leave_disabled_pointer_events_to_ancestors(cx: &mut TestAppContext) {
        struct CompatibilityHarness {
            parent_mouse_downs: Rc<Cell<usize>>,
        }

        impl Render for CompatibilityHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let parent_mouse_downs = self.parent_mouse_downs.clone();
                div()
                    .id("parent")
                    .size(px(100.))
                    .on_mouse_down(MouseButton::Left, move |_, _, _| {
                        parent_mouse_downs.set(parent_mouse_downs.get() + 1)
                    })
                    .child(
                        Switch::new("switch")
                            .disabled(true)
                            .focusable(false)
                            .block_pointer_when_disabled(false)
                            .size_full(),
                    )
            }
        }

        let parent_mouse_downs = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let parent_mouse_downs = parent_mouse_downs.clone();
            move |_, _| CompatibilityHarness { parent_mouse_downs }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());

        assert_eq!(parent_mouse_downs.get(), 1);
        cx.update(|window, cx| assert!(window.focused(cx).is_none()));
    }

    #[test]
    fn application_owned_state_styles_are_available() {
        let _ = Switch::new("states")
            .hover(|style| style.opacity(0.9))
            .active(|style| style.opacity(0.8))
            .focus_visible(|style| style.opacity(0.7));
    }

    #[gpui::test]
    fn accessibility_exposes_switch_role_label_and_toggled_state(cx: &mut TestAppContext) {
        type Captured = Arc<Mutex<Option<(accesskit::Node, accesskit::Node)>>>;

        struct A11yProbe {
            captured: Captured,
        }

        impl Render for A11yProbe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let captured = self.captured.clone();
                canvas(
                    move |_, window, cx| {
                        let mut info = |switch: Switch| {
                            let mut node = accesskit::Node::new(Role::Switch);
                            switch
                                .render(window, cx)
                                .into_element()
                                .write_a11y_info(&mut node);
                            node
                        };
                        let enabled = info(
                            Switch::new("enabled")
                                .checked(true)
                                .accessibility_label("Airplane mode")
                                .on_toggle(|_, _, _, _| {}),
                        );
                        let disabled = info(
                            Switch::new("disabled")
                                .checked(false)
                                .disabled(true)
                                .accessibility_label("Airplane mode")
                                .on_toggle(|_, _, _, _| {}),
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
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let (enabled, disabled) = result.lock().unwrap().take().unwrap();

        assert_eq!(enabled.role(), Role::Switch);
        assert_eq!(enabled.label(), Some("Airplane mode"));
        assert_eq!(enabled.toggled(), Some(Toggled::True));
        assert!(enabled.supports_action(accesskit::Action::Click));

        assert_eq!(disabled.role(), Role::Switch);
        assert_eq!(disabled.toggled(), Some(Toggled::False));
        assert!(!disabled.supports_action(accesskit::Action::Click));
        // GPUI currently has no aria-disabled setter. Keep the limitation
        // explicit instead of claiming an AccessKit disabled state.
        assert!(!disabled.is_disabled());
    }
}
