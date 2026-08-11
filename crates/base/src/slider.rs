use std::rc::Rc;

use gpui::{
    AccessibleAction, AnyElement, App, Div, ElementId, FocusHandle, InteractiveElement,
    Interactivity, IntoElement, KeyDownEvent, Orientation, ParentElement, Refineable as _,
    RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, div, prelude::FluentBuilder as _,
};
use smallvec::SmallVec;

use crate::StateStyle;

type ChangeHandler = Rc<dyn Fn(f32, &mut Window, &mut App)>;

/// An unstyled, controlled single-value slider.
///
/// This primitive owns range normalization, stepping, focus, keyboard, and
/// accessibility behavior. The application owns the track/thumb geometry and
/// all visual presentation.
#[derive(IntoElement)]
pub struct Slider {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    semantic_styles: SliderStyles,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    disabled: bool,
    orientation: Orientation,
    children: SmallVec<[AnyElement; 2]>,
    on_change: Option<ChangeHandler>,
    accessibility_label: Option<SharedString>,
    tab_index: isize,
    tab_stop: bool,
}

/// Semantic root styles supported by [`Slider`].
#[derive(Default)]
pub struct SliderStyles {
    disabled: StyleRefinement,
}

impl SliderStyles {
    pub fn disabled(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.disabled
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }
}

impl Slider {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()),
            id,
            style: StyleRefinement::default(),
            semantic_styles: SliderStyles::default(),
            value: 0.,
            min: 0.,
            max: 100.,
            step: 1.,
            disabled: false,
            orientation: Orientation::Horizontal,
            children: SmallVec::new(),
            on_change: None,
            accessibility_label: None,
            tab_index: 0,
            tab_stop: true,
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    /// Sets a positive finite step. Invalid steps fall back to `1.0` when used.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Configures application-owned styles for the slider's semantic states.
    pub fn styles(mut self, build: impl FnOnce(SliderStyles) -> SliderStyles) -> Self {
        self.semantic_styles = build(self.semantic_styles);
        self
    }

    fn resolved_style(&self) -> StyleRefinement {
        let mut style = self.style.clone();
        if self.disabled {
            style.refine(&self.semantic_styles.disabled);
        }
        style
    }

    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
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

    /// Resolves an application-measured pointer fraction into a stepped value.
    ///
    /// Registry-owned presentation can calculate the fraction from its own
    /// track geometry and feed the result into its controlled state.
    pub fn value_for_fraction(min: f32, max: f32, step: f32, fraction: f32) -> f32 {
        normalize(min + (max - min) * fraction.clamp(0., 1.), min, max, step)
    }

    fn focus_handle(&self, window: &mut Window, cx: &mut App) -> FocusHandle {
        window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone()
    }
}

impl Styled for Slider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Slider {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Slider {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Slider {}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.focus_handle(window, cx);
        let min = self.min.min(self.max);
        let max = self.min.max(self.max);
        let step = valid_step(self.step);
        let value = normalize(self.value, min, max, step);
        let disabled = self.disabled;
        let style = self.resolved_style();
        let on_change = self.on_change;

        self.base
            .role(Role::Slider)
            .aria_numeric_value(value as f64)
            .aria_min_numeric_value(min as f64)
            .aria_max_numeric_value(max as f64)
            .aria_orientation(self.orientation)
            .when_some(self.accessibility_label, |this, label| {
                this.aria_label(label)
            })
            .when(!disabled, |this| {
                this.track_focus(
                    &focus_handle
                        .tab_index(self.tab_index)
                        .tab_stop(self.tab_stop),
                )
            })
            .when_some(
                (!disabled).then_some(on_change).flatten(),
                |this, on_change| {
                    let increment = on_change.clone();
                    let decrement = on_change.clone();
                    let keyboard = on_change.clone();
                    this.on_a11y_action(AccessibleAction::Increment, move |_, window, cx| {
                        increment(normalize(value + step, min, max, step), window, cx);
                    })
                    .on_a11y_action(AccessibleAction::Decrement, move |_, window, cx| {
                        decrement(normalize(value - step, min, max, step), window, cx);
                    })
                    .on_key_down(move |event: &KeyDownEvent, window, cx| {
                        let requested = match event.keystroke.key.as_str() {
                            "left" | "down" => Some(normalize(value - step, min, max, step)),
                            "right" | "up" => Some(normalize(value + step, min, max, step)),
                            "home" => Some(min),
                            "end" => Some(max),
                            _ => None,
                        };
                        if let Some(requested) = requested {
                            cx.stop_propagation();
                            keyboard(requested, window, cx);
                        }
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

fn valid_step(step: f32) -> f32 {
    if step.is_finite() && step > 0. {
        step
    } else {
        1.
    }
}

fn normalize(value: f32, min: f32, max: f32, step: f32) -> f32 {
    if !value.is_finite() || min == max {
        return min;
    }
    let step = valid_step(step);
    let steps = ((value.clamp(min, max) - min) / step).round();
    (min + steps * step).clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use gpui::{
        Context, Element as _, KeyDownEvent, Keystroke, Render, TestAppContext, VisualTestContext,
        accesskit, canvas, px,
    };

    struct Harness {
        disabled: bool,
        changes: Rc<RefCell<Vec<f32>>>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let changes = self.changes.clone();
            Slider::new("slider")
                .min(10.)
                .max(20.)
                .step(2.)
                .value(14.)
                .disabled(self.disabled)
                .size(px(100.))
                .on_change(move |value, _, _| changes.borrow_mut().push(value))
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        disabled: bool,
    ) -> (&mut VisualTestContext, Rc<RefCell<Vec<f32>>>) {
        let changes = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let changes = changes.clone();
            move |_, _| Harness { disabled, changes }
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.focus_next(cx);
        });
        (cx, changes)
    }

    #[test]
    fn fraction_conversion_clamps_and_steps_from_minimum() {
        assert_eq!(Slider::value_for_fraction(10., 20., 2., 0.49), 14.);
        assert_eq!(Slider::value_for_fraction(10., 20., 2., -1.), 10.);
        assert_eq!(Slider::value_for_fraction(10., 20., 2., 2.), 20.);
    }

    #[test]
    fn semantic_state_styles_are_available_to_applications() {
        let _ = Slider::new("states").styles(|styles| styles.disabled(|style| style.opacity(0.5)));
    }

    #[test]
    fn disabled_semantic_style_refines_instance_style_only_when_active() {
        let enabled = Slider::new("enabled")
            .opacity(0.9)
            .styles(|styles| styles.disabled(|style| style.opacity(0.5)));
        assert_eq!(enabled.resolved_style().opacity, Some(0.9));

        let disabled = Slider::new("disabled")
            .styles(|styles| styles.disabled(|style| style.opacity(0.5)))
            .opacity(0.9)
            .disabled(true);
        assert_eq!(disabled.resolved_style().opacity, Some(0.5));
    }

    #[gpui::test]
    fn keyboard_requests_stepped_values(cx: &mut TestAppContext) {
        let (cx, changes) = harness(cx, false);
        for key in ["right", "left", "home", "end"] {
            cx.simulate_event(KeyDownEvent {
                keystroke: Keystroke::parse(key).unwrap(),
                is_held: false,
                prefer_character_input: false,
            });
        }
        assert_eq!(changes.borrow().as_slice(), &[16., 12., 10., 20.]);
    }

    #[gpui::test]
    fn disabled_slider_is_keyboard_inert(cx: &mut TestAppContext) {
        let (cx, changes) = harness(cx, true);
        cx.simulate_keystrokes("right left home end");
        assert!(changes.borrow().is_empty());
    }

    #[gpui::test]
    fn accessibility_exposes_range_orientation_and_actions(cx: &mut TestAppContext) {
        type Captured = Arc<Mutex<Option<(accesskit::Node, accesskit::Node)>>>;
        struct Probe(Captured);
        impl Render for Probe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let captured = self.0.clone();
                canvas(
                    move |_, window, cx| {
                        let mut info = |slider: Slider| {
                            let mut node = accesskit::Node::new(Role::Slider);
                            slider
                                .render(window, cx)
                                .into_element()
                                .write_a11y_info(&mut node);
                            node
                        };
                        let enabled = info(
                            Slider::new("enabled")
                                .min(10.)
                                .max(20.)
                                .value(14.)
                                .orientation(Orientation::Vertical)
                                .accessibility_label("Volume")
                                .on_change(|_, _, _| {}),
                        );
                        let disabled = info(
                            Slider::new("disabled")
                                .disabled(true)
                                .on_change(|_, _, _| {}),
                        );
                        *captured.lock().unwrap() = Some((enabled, disabled));
                    },
                    |_, _, _, _| {},
                )
            }
        }
        let captured: Captured = Arc::new(Mutex::new(None));
        let result = captured.clone();
        let (_, cx) = cx.add_window_view(move |_, _| Probe(captured));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let (enabled, disabled) = result.lock().unwrap().take().unwrap();
        assert_eq!(enabled.role(), Role::Slider);
        assert_eq!(enabled.label(), Some("Volume"));
        assert_eq!(enabled.numeric_value(), Some(14.));
        assert_eq!(enabled.min_numeric_value(), Some(10.));
        assert_eq!(enabled.max_numeric_value(), Some(20.));
        assert_eq!(enabled.orientation(), Some(Orientation::Vertical));
        assert!(enabled.supports_action(accesskit::Action::Increment));
        assert!(enabled.supports_action(accesskit::Action::Decrement));
        assert!(!disabled.supports_action(accesskit::Action::Increment));
        assert!(!disabled.supports_action(accesskit::Action::Decrement));
    }
}
