use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, FocusHandle, InteractiveElement, Interactivity,
    IntoElement, MouseButton, ParentElement, Refineable as _, RenderOnce, Role, SharedString,
    Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Toggled, Window, div,
    prelude::FluentBuilder as _,
};
use smallvec::SmallVec;

type ChangeHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;

/// An unstyled radio control that owns activation, focus, and accessibility behavior.
///
/// The application owns its indicator, label, layout, colors, and state styling.
/// Selection is controlled through [`Radio::checked`]; activating an unchecked
/// radio requests `true` through [`Radio::on_change`].
#[derive(IntoElement)]
pub struct Radio {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    checked: bool,
    disabled: bool,
    children: SmallVec<[AnyElement; 2]>,
    on_change: Option<ChangeHandler>,
    accessibility_label: Option<SharedString>,
    position_in_set: Option<usize>,
    size_of_set: Option<usize>,
    tab_index: isize,
    tab_stop: bool,
    toggle_on_activate: bool,
    prevent_pointer_focus: bool,
    stop_disabled_pointer_propagation: bool,
    selected_accessibility_state: bool,
}

impl Radio {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()),
            id,
            style: StyleRefinement::default(),
            checked: false,
            disabled: false,
            children: SmallVec::new(),
            on_change: None,
            accessibility_label: None,
            position_in_set: None,
            size_of_set: None,
            tab_index: 0,
            tab_stop: true,
            toggle_on_activate: false,
            prevent_pointer_focus: false,
            stop_disabled_pointer_propagation: true,
            selected_accessibility_state: false,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Sets the one-based position exposed to accessibility clients.
    pub fn position_in_set(mut self, position: usize) -> Self {
        self.position_in_set = Some(position);
        self
    }

    pub fn size_of_set(mut self, size: usize) -> Self {
        self.size_of_set = Some(size);
        self
    }

    /// Handles a requested selection change.
    ///
    /// The callback receives `true`. Activating an already checked radio is a
    /// no-op because a radio cannot deselect itself.
    pub fn on_change(
        mut self,
        handler: impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Requests the inverse checked state on activation, including when the
    /// radio is already checked. This supports legacy independently-managed
    /// radios; radio groups normally keep selection-only behavior.
    pub fn toggle_on_activate(mut self, toggle: bool) -> Self {
        self.toggle_on_activate = toggle;
        self
    }

    /// Prevents pointer-down from moving focus, for compatibility with hosts
    /// that manage radio focus separately.
    pub fn prevent_pointer_focus(mut self, prevent: bool) -> Self {
        self.prevent_pointer_focus = prevent;
        self
    }

    /// Controls whether a disabled radio consumes pointer-down propagation.
    pub fn stop_disabled_pointer_propagation(mut self, stop: bool) -> Self {
        self.stop_disabled_pointer_propagation = stop;
        self
    }

    /// Reports the checked state with `aria-selected` instead of the default
    /// radio `aria-toggled` state for legacy accessibility compatibility.
    pub fn selected_accessibility_state(mut self, selected: bool) -> Self {
        self.selected_accessibility_state = selected;
        self
    }

    fn focus_handle(&self, window: &mut Window, cx: &mut App) -> FocusHandle {
        window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone()
    }
}

impl Styled for Radio {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Radio {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Radio {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Radio {}

impl RenderOnce for Radio {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.focus_handle(window, cx);
        let disabled = self.disabled;
        let checked = self.checked;
        let style = self.style;
        let on_change = self.on_change;
        let toggle_on_activate = self.toggle_on_activate;

        self.base
            .role(Role::RadioButton)
            .when(self.selected_accessibility_state, |this| {
                this.aria_selected(checked)
            })
            .when(!self.selected_accessibility_state, |this| {
                this.aria_toggled(if checked {
                    Toggled::True
                } else {
                    Toggled::False
                })
            })
            .when_some(self.accessibility_label, |this, label| {
                this.aria_label(label)
            })
            .when_some(self.position_in_set, |this, position| {
                this.aria_position_in_set(position)
            })
            .when_some(self.size_of_set, |this, size| this.aria_size_of_set(size))
            .when(!disabled, |this| {
                this.track_focus(
                    &focus_handle
                        .tab_index(self.tab_index)
                        .tab_stop(self.tab_stop),
                )
            })
            .when(self.prevent_pointer_focus, |this| {
                this.on_mouse_down(MouseButton::Left, |_, window, _| {
                    window.prevent_default();
                })
            })
            .when(disabled && self.stop_disabled_pointer_propagation, |this| {
                this.on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
            })
            .when_some(
                (!disabled && (!checked || toggle_on_activate))
                    .then_some(on_change)
                    .flatten(),
                |this, on_change| {
                    this.on_click(move |event, window, cx| {
                        on_change(!checked, event, window, cx);
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

/// Controlled selection state shared by application-owned radio groups.
///
/// This type deliberately does not prescribe layout. Arrow-key roving focus
/// requires a group element that owns ordered item focus handles and is not yet
/// provided by this primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadioGroupState<T> {
    selected: Option<T>,
    disabled: bool,
}

impl<T> Default for RadioGroupState<T> {
    fn default() -> Self {
        Self {
            selected: None,
            disabled: false,
        }
    }
}

impl<T: PartialEq> RadioGroupState<T> {
    pub fn new(selected: Option<T>) -> Self {
        Self {
            selected,
            disabled: false,
        }
    }

    pub fn selected(&self) -> Option<&T> {
        self.selected.as_ref()
    }

    pub fn is_selected(&self, value: &T) -> bool {
        self.selected.as_ref() == Some(value)
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// Selects a value, returning whether the controlled state changed.
    pub fn select(&mut self, value: T) -> bool {
        if self.disabled || self.is_selected(&value) {
            return false;
        }
        self.selected = Some(value);
        true
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

    struct RadioHarness {
        checked: bool,
        disabled: bool,
        changes: Rc<Cell<usize>>,
        keyboard_changes: Rc<Cell<usize>>,
    }

    impl Render for RadioHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let changes = self.changes.clone();
            let keyboard_changes = self.keyboard_changes.clone();
            Radio::new("radio")
                .checked(self.checked)
                .disabled(self.disabled)
                .size(px(100.))
                .on_change(move |checked, event, _, _| {
                    assert!(checked);
                    changes.set(changes.get() + 1);
                    if matches!(event, ClickEvent::Keyboard(_)) {
                        keyboard_changes.set(keyboard_changes.get() + 1);
                    }
                })
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        checked: bool,
        disabled: bool,
    ) -> (&mut VisualTestContext, Rc<Cell<usize>>, Rc<Cell<usize>>) {
        let changes = Rc::new(Cell::new(0));
        let keyboard_changes = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let changes = changes.clone();
            let keyboard_changes = keyboard_changes.clone();
            move |_, _| RadioHarness {
                checked,
                disabled,
                changes,
                keyboard_changes,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, changes, keyboard_changes)
    }

    #[gpui::test]
    fn pointer_and_keyboard_activation_fire_once(cx: &mut TestAppContext) {
        let (cx, changes, keyboard_changes) = harness(cx, false, false);
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        assert_eq!(changes.get(), 1);

        changes.set(0);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        for key in ["enter", "space"] {
            let keystroke = Keystroke::parse(key).unwrap();
            cx.simulate_event(KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            });
            cx.simulate_event(KeyUpEvent { keystroke });
        }
        assert_eq!(changes.get(), 2);
        assert_eq!(keyboard_changes.get(), 2);
    }

    #[gpui::test]
    fn checked_and_disabled_radios_are_inert(cx: &mut TestAppContext) {
        for (checked, disabled) in [(true, false), (false, true)] {
            let (cx, changes, _) = harness(cx, checked, disabled);
            cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
            cx.simulate_keystrokes("enter space");
            assert_eq!(changes.get(), 0);
        }
    }

    #[gpui::test]
    fn accessibility_exposes_role_state_set_metadata_and_action(cx: &mut TestAppContext) {
        type Captured = Arc<Mutex<Option<accesskit::Node>>>;
        struct Probe(Captured);
        impl Render for Probe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let captured = self.0.clone();
                canvas(
                    move |_, window, cx| {
                        let mut node = accesskit::Node::new(Role::RadioButton);
                        Radio::new("probe")
                            .checked(true)
                            .accessibility_label("Choice")
                            .position_in_set(2)
                            .size_of_set(3)
                            .render(window, cx)
                            .into_element()
                            .write_a11y_info(&mut node);
                        *captured.lock().unwrap() = Some(node);
                    },
                    |_, _, _, _| {},
                )
            }
        }
        let captured: Captured = Arc::new(Mutex::new(None));
        let result = captured.clone();
        let (_, cx) = cx.add_window_view(move |_, _| Probe(captured));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let node = result.lock().unwrap().take().unwrap();
        assert_eq!(node.role(), Role::RadioButton);
        assert_eq!(node.label(), Some("Choice"));
        assert_eq!(node.toggled(), Some(Toggled::True));
        assert_eq!(node.position_in_set(), Some(2));
        assert_eq!(node.size_of_set(), Some(3));
        assert!(!node.supports_action(accesskit::Action::Click));
    }

    #[test]
    fn group_state_is_controlled_and_respects_disabled() {
        let mut group = RadioGroupState::new(Some("one"));
        assert!(group.is_selected(&"one"));
        assert!(group.select("two"));
        assert_eq!(group.selected(), Some(&"two"));
        assert!(!group.select("two"));
        group.set_disabled(true);
        assert!(!group.select("three"));
        assert_eq!(group.selected(), Some(&"two"));
    }
}
