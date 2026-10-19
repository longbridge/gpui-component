use crate::TestSupportExt as _;
use std::ops::Range;

use gpui::{
    AccessibleAction, Along, AnyElement, App, AppContext as _, Axis, Bounds, Context, Div,
    DragMoveEvent, Empty, Entity, EntityId, EventEmitter, FocusHandle, HitboxBehavior,
    InteractiveElement, IntoElement, KeyBinding, MouseButton, MouseDownEvent, Orientation,
    ParentElement, Pixels, Point, Render, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, TouchDragEvent, TouchPhase, Window,
    actions, div, prelude::FluentBuilder as _, px,
};

use crate::{a11y_disabled::A11yDisabledExt as _, element_ext::ElementExt, geometry::AxisExt};

actions!(
    slider,
    [
        Increment,
        Decrement,
        IncrementPage,
        DecrementPage,
        MoveToMinimum,
        MoveToMaximum
    ]
);

const CONTEXT: &str = "Slider";

/// The steps one Page Up or Page Down moves.
const PAGE_STEPS: f32 = 10.;

/// Binds the slider keys: Right and Up increase by a step, Left and Down
/// decrease, Page Up and Page Down move ten steps, Home and End go to the
/// minimum and maximum.
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("right", Increment, Some(CONTEXT)),
        KeyBinding::new("up", Increment, Some(CONTEXT)),
        KeyBinding::new("left", Decrement, Some(CONTEXT)),
        KeyBinding::new("down", Decrement, Some(CONTEXT)),
        KeyBinding::new("pageup", IncrementPage, Some(CONTEXT)),
        KeyBinding::new("pagedown", DecrementPage, Some(CONTEXT)),
        KeyBinding::new("home", MoveToMinimum, Some(CONTEXT)),
        KeyBinding::new("end", MoveToMaximum, Some(CONTEXT)),
    ]);
}

/// Events emitted by the [`SliderState`].
pub enum SliderEvent {
    /// Emitted continuously while the slider value is being changed by the user.
    Change(SliderValue),
    /// Emitted once when the user releases the slider after a drag or click.
    Release(SliderValue),
}

/// The value of the slider, can be a single value or a range of values.
///
/// - Can from a f32 value, which will be treated as a single value.
/// - Or from a (f32, f32) tuple, which will be treated as a range of values.
///
/// The default value is `SliderValue::Single(0.0)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SliderValue {
    Single(f32),
    Range(f32, f32),
}

impl std::fmt::Display for SliderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SliderValue::Single(value) => write!(f, "{}", value),
            SliderValue::Range(start, end) => write!(f, "{}..{}", start, end),
        }
    }
}

impl From<f32> for SliderValue {
    fn from(value: f32) -> Self {
        SliderValue::Single(value)
    }
}

impl From<(f32, f32)> for SliderValue {
    fn from(value: (f32, f32)) -> Self {
        SliderValue::Range(value.0, value.1)
    }
}

impl From<Range<f32>> for SliderValue {
    fn from(value: Range<f32>) -> Self {
        SliderValue::Range(value.start, value.end)
    }
}

impl Default for SliderValue {
    fn default() -> Self {
        SliderValue::Single(0.)
    }
}

impl SliderValue {
    /// Clamp the value to the given range.
    pub fn clamp(self, min: f32, max: f32) -> Self {
        match self {
            SliderValue::Single(value) => SliderValue::Single(value.clamp(min, max)),
            SliderValue::Range(start, end) => {
                SliderValue::Range(start.clamp(min, max), end.clamp(min, max))
            }
        }
    }

    /// Check if the value is a single value.
    #[inline]
    pub fn is_single(&self) -> bool {
        matches!(self, SliderValue::Single(_))
    }

    /// Check if the value is a range of values.
    #[inline]
    pub fn is_range(&self) -> bool {
        matches!(self, SliderValue::Range(_, _))
    }

    /// Get the start value.
    pub fn start(&self) -> f32 {
        match self {
            SliderValue::Single(value) => *value,
            SliderValue::Range(start, _) => *start,
        }
    }

    /// Get the end value.
    pub fn end(&self) -> f32 {
        match self {
            SliderValue::Single(value) => *value,
            SliderValue::Range(_, end) => *end,
        }
    }

    fn set_start(&mut self, value: f32) {
        if let SliderValue::Range(_, end) = self {
            *self = SliderValue::Range(value.min(*end), *end);
        } else {
            *self = SliderValue::Single(value);
        }
    }

    fn set_end(&mut self, value: f32) {
        if let SliderValue::Range(start, _) = self {
            *self = SliderValue::Range(*start, value.max(*start));
        } else {
            *self = SliderValue::Single(value);
        }
    }
}

/// The scale mode of the slider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SliderScale {
    /// Linear scale where values change uniformly across the slider range.
    /// This is the default mode.
    #[default]
    Linear,
    /// Logarithmic scale where the distance between values increases exponentially.
    ///
    /// This is useful for parameters that have a large range of values where smaller
    /// changes are more significant at lower values. Common examples include:
    ///
    /// - Volume controls (human hearing perception is logarithmic)
    /// - Frequency controls (musical notes follow a logarithmic scale)
    /// - Zoom levels
    /// - Any parameter where you want finer control at lower values
    ///
    /// # For example
    ///
    /// ```
    /// use gpui_base::slider::{SliderScale, SliderState};
    ///
    /// let slider = SliderState::new()
    ///     .min(1.0)    // Must be > 0 for logarithmic scale
    ///     .max(1000.0)
    ///     .scale(SliderScale::Logarithmic);
    /// ```
    ///
    /// - Moving the slider 1/3 of the way will yield ~10
    /// - Moving it 2/3 of the way will yield ~100
    /// - The full range covers 3 orders of magnitude evenly
    Logarithmic,
}

impl SliderScale {
    #[inline]
    pub fn is_linear(&self) -> bool {
        matches!(self, SliderScale::Linear)
    }

    #[inline]
    pub fn is_logarithmic(&self) -> bool {
        matches!(self, SliderScale::Logarithmic)
    }
}

/// State of the [`Slider`].
pub struct SliderState {
    min: f32,
    max: f32,
    step: f32,
    value: SliderValue,
    /// When is single value mode, only `end` is used, the start is always 0.0.
    percentage: Range<f32>,
    /// The bounds of the slider after rendered.
    bounds: Bounds<Pixels>,
    scale: SliderScale,
    /// Tracks whether the user is currently interacting with the slider so we
    /// only emit [`SliderEvent::Release`] after a real press/drag.
    dragging: bool,
    /// The touch drag this slider claimed, while one is live.
    touch_drag: Option<TouchDrag>,
    /// Whether the keyboard moves the range start thumb: the thumb the
    /// pointer moved last, the end thumb until then.
    keyboard_start: bool,
}

/// A touch drag claimed by a slider.
#[derive(Clone, Copy)]
struct TouchDrag {
    /// Identifies the gesture: every event of one drag carries the same start.
    start_position: Point<Pixels>,
    /// Whether it moves the range start thumb.
    is_start: bool,
}

impl SliderState {
    /// Create a new [`SliderState`].
    pub fn new() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            value: SliderValue::default(),
            percentage: (0.0..0.0),
            bounds: Bounds::default(),
            scale: SliderScale::default(),
            dragging: false,
            touch_drag: None,
            keyboard_start: false,
        }
    }

    /// Set the minimum value of the slider, default: 0.0
    pub fn min(mut self, min: f32) -> Self {
        if self.scale.is_logarithmic() {
            assert!(
                min > 0.0,
                "`min` must be greater than 0 for SliderScale::Logarithmic"
            );
            assert!(
                min < self.max,
                "`min` must be less than `max` for Logarithmic scale"
            );
        }
        self.min = min;
        self.update_thumb_pos();
        self
    }

    /// Set the maximum value of the slider, default: 100.0
    pub fn max(mut self, max: f32) -> Self {
        if self.scale.is_logarithmic() {
            assert!(
                max > self.min,
                "`max` must be greater than `min` for Logarithmic scale"
            );
        }
        self.max = max;
        self.update_thumb_pos();
        self
    }

    /// Set the step value of the slider, default: 1.0
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Set the scale of the slider, default: [`SliderScale::Linear`].
    pub fn scale(mut self, scale: SliderScale) -> Self {
        if scale.is_logarithmic() {
            assert!(
                self.min > 0.0,
                "`min` must be greater than 0 for Logarithmic scale"
            );
            assert!(
                self.max > self.min,
                "`max` must be greater than `min` for Logarithmic scale"
            );
        }
        self.scale = scale;
        self.update_thumb_pos();
        self
    }

    /// Set the default value of the slider, default: 0.0
    pub fn default_value(mut self, value: impl Into<SliderValue>) -> Self {
        self.value = value.into();
        self.update_thumb_pos();
        self
    }

    /// Set the value of the slider.
    pub fn set_value(
        &mut self,
        value: impl Into<SliderValue>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.value = value.into();
        self.update_thumb_pos();
        cx.notify();
    }

    /// Get the value of the slider.
    pub fn value(&self) -> SliderValue {
        self.value
    }

    /// Get the minimum value.
    pub fn min_value(&self) -> f32 {
        self.min
    }

    /// Get the maximum value.
    pub fn max_value(&self) -> f32 {
        self.max
    }

    /// Get the step value.
    pub fn step_value(&self) -> f32 {
        self.step
    }

    /// Converts a value between 0.0 and 1.0 to a value between the minimum and maximum value,
    /// depending on the chosen scale.
    fn percentage_to_value(&self, percentage: f32) -> f32 {
        match self.scale {
            SliderScale::Linear => self.min + (self.max - self.min) * percentage,
            SliderScale::Logarithmic => {
                // when percentage is 0, this simplifies to (max/min)^0 * min = 1 * min = min
                // when percentage is 1, this simplifies to (max/min)^1 * min = (max*min)/min = max
                // we clamp just to make sure we don't have issue with floating point precision
                let base = self.max / self.min;
                (base.powf(percentage) * self.min).clamp(self.min, self.max)
            }
        }
    }

    /// Converts a value between the minimum and maximum value to a value between 0.0 and 1.0,
    /// depending on the chosen scale.
    fn value_to_percentage(&self, value: f32) -> f32 {
        match self.scale {
            SliderScale::Linear => {
                let range = self.max - self.min;
                if range <= 0.0 {
                    0.0
                } else {
                    (value - self.min) / range
                }
            }
            SliderScale::Logarithmic => {
                let base = self.max / self.min;
                (value / self.min).log(base).clamp(0.0, 1.0)
            }
        }
    }

    fn update_thumb_pos(&mut self) {
        match self.value {
            SliderValue::Single(value) => {
                let percentage = self.value_to_percentage(value.clamp(self.min, self.max));
                self.percentage = 0.0..percentage;
            }
            SliderValue::Range(start, end) => {
                let clamped_start = start.clamp(self.min, self.max);
                let clamped_end = end.clamp(self.min, self.max);
                self.percentage =
                    self.value_to_percentage(clamped_start)..self.value_to_percentage(clamped_end);
            }
        }
    }

    /// Update value by mouse position
    #[doc(hidden)]
    pub fn update_value_by_position(
        &mut self,
        axis: Axis,
        position: Point<Pixels>,
        is_start: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dragging = true;
        let bounds = self.bounds;
        let step = self.step;

        let inner_pos = if axis.is_horizontal() {
            position.x - bounds.left()
        } else {
            bounds.bottom() - position.y
        };
        let total_size = bounds.size.along(axis);
        let percentage = inner_pos.clamp(px(0.), total_size) / total_size;

        let percentage = if is_start {
            percentage.clamp(0.0, self.percentage.end)
        } else {
            percentage.clamp(self.percentage.start, 1.0)
        };

        let value = self.percentage_to_value(percentage);
        let value = (value / step).round() * step;

        self.keyboard_start = is_start;
        if is_start {
            self.percentage.start = percentage;
            self.value.set_start(value);
        } else {
            self.percentage.end = percentage;
            self.value.set_end(value);
        }
        cx.emit(SliderEvent::Change(self.value));
        cx.notify();
    }

    /// Whether `position` is closer to the range start thumb than to the end thumb.
    fn is_nearer_start(&self, axis: Axis, position: Point<Pixels>) -> bool {
        let size = self.bounds.size.along(axis);
        let along = if axis.is_horizontal() {
            position.x - self.bounds.left()
        } else {
            self.bounds.bottom() - position.y
        };
        let center =
            ((self.percentage.end - self.percentage.start) / 2. + self.percentage.start) * size;
        along < center
    }

    /// Whether the keyboard moves the range start thumb: the thumb the
    /// pointer moved last, the end thumb until then. Applications use it to
    /// show which thumb the arrow keys move while the slider has focus.
    pub fn is_keyboard_on_start(&self) -> bool {
        self.keyboard_start && self.value.is_range()
    }

    /// The value of the thumb the keyboard moves.
    fn keyboard_value(&self) -> f32 {
        if self.is_keyboard_on_start() {
            self.value.start()
        } else {
            self.value.end()
        }
    }

    /// Moves the thumb the keyboard moves to `value`, snapped to the step and
    /// kept within the slider and on its side of the other thumb. A key press
    /// is a whole interaction, so it reports [`SliderEvent::Change`] and then
    /// [`SliderEvent::Release`], and nothing when the value stays.
    fn move_keyboard_thumb(&mut self, value: f32, cx: &mut Context<Self>) {
        let value = if self.step > 0. {
            (value / self.step).round() * self.step
        } else {
            value
        };
        let value = value.clamp(self.min, self.max);
        let previous = self.value;
        if self.is_keyboard_on_start() {
            self.value.set_start(value);
        } else {
            self.value.set_end(value);
        }
        if self.value == previous {
            return;
        }
        self.update_thumb_pos();
        cx.emit(SliderEvent::Change(self.value));
        cx.emit(SliderEvent::Release(self.value));
        cx.notify();
    }

    /// Moves the thumb the keyboard moves by `steps` steps.
    fn step_keyboard_thumb(&mut self, steps: f32, cx: &mut Context<Self>) {
        let value = self.keyboard_value() + steps * self.step;
        self.move_keyboard_thumb(value, cx);
    }

    /// Emit [`SliderEvent::Release`] if the user was actively interacting
    /// with the slider. Called on mouse-up both inside and outside the slider.
    #[doc(hidden)]
    pub fn handle_release(&mut self, cx: &mut Context<Self>) {
        if !self.dragging {
            return;
        }
        self.dragging = false;
        cx.emit(SliderEvent::Release(self.value));
    }
}

#[derive(Clone)]
struct DragThumb((EntityId, bool));

impl Render for DragThumb {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone)]
struct DragSlider(EntityId);

impl Render for DragSlider {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// An unstyled slider behavior root.
///
/// Applications provide the track, range, and thumb presentation as children.
#[derive(IntoElement)]
pub struct Slider {
    state: Entity<SliderState>,
    axis: Axis,
    disabled: bool,
    accessibility_label: Option<SharedString>,
    tab_index: isize,
    tab_stop: bool,
    provided_focus_handle: Option<FocusHandle>,
    base: Div,
    children: Vec<AnyElement>,
}

impl Slider {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            axis: Axis::Horizontal,
            disabled: false,
            accessibility_label: None,
            tab_index: 0,
            tab_stop: true,
            provided_focus_handle: None,
            base: div(),
            children: Vec::new(),
        }
    }

    pub fn horizontal(mut self) -> Self {
        self.axis = Axis::Horizontal;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.axis = Axis::Vertical;
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the label exposed to accessibility clients.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Sets the focus traversal index. Use this within a GPUI tab group.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Sets whether the slider participates in keyboard focus traversal.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Uses a caller-owned focus handle instead of creating keyed state.
    pub fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.provided_focus_handle = Some(focus_handle.clone());
        self
    }

    fn focus_handle(&self, window: &mut Window, cx: &mut App) -> FocusHandle {
        self.provided_focus_handle.clone().unwrap_or_else(|| {
            window
                .use_keyed_state(("slider-focus", self.state.entity_id()), cx, |_, cx| {
                    cx.focus_handle()
                })
                .read(cx)
                .clone()
        })
    }
}

impl ParentElement for Slider {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Slider {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Slider {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Slider {}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let axis = self.axis;
        let entity_id = self.state.entity_id();
        let focus_handle = self.focus_handle(window, cx);
        let state = self.state.read(cx);

        self.base
            .id(("slider", entity_id))
            .test_support()
            .role(Role::Slider)
            .when_some(self.accessibility_label, |this, label| {
                this.aria_label(label)
            })
            .aria_numeric_value(state.value().end() as f64)
            .aria_min_numeric_value(state.min_value() as f64)
            .aria_max_numeric_value(state.max_value() as f64)
            .aria_numeric_value_step(state.step_value() as f64)
            .aria_orientation(if axis.is_vertical() {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            })
            // A disabled slider takes no value, from the pointer, the keyboard
            // or assistive technology, and is no Tab stop.
            .when(!self.disabled, |this| {
                this.track_focus(
                    &focus_handle
                        .clone()
                        .tab_index(self.tab_index)
                        .tab_stop(self.tab_stop),
                )
                // The thumb stops its press from reaching the slider, so take
                // focus before it does.
                .capture_any_mouse_down(move |_, window, cx| window.focus(&focus_handle, cx))
                .key_context(CONTEXT)
                .on_action(
                    window.listener_for(&self.state, |state, _: &Increment, _, cx| {
                        state.step_keyboard_thumb(1., cx)
                    }),
                )
                .on_action(
                    window.listener_for(&self.state, |state, _: &Decrement, _, cx| {
                        state.step_keyboard_thumb(-1., cx)
                    }),
                )
                .on_action(
                    window.listener_for(&self.state, |state, _: &IncrementPage, _, cx| {
                        state.step_keyboard_thumb(PAGE_STEPS, cx)
                    }),
                )
                .on_action(
                    window.listener_for(&self.state, |state, _: &DecrementPage, _, cx| {
                        state.step_keyboard_thumb(-PAGE_STEPS, cx)
                    }),
                )
                .on_action(
                    window.listener_for(&self.state, |state, _: &MoveToMinimum, _, cx| {
                        state.move_keyboard_thumb(state.min, cx)
                    }),
                )
                .on_action(
                    window.listener_for(&self.state, |state, _: &MoveToMaximum, _, cx| {
                        state.move_keyboard_thumb(state.max, cx)
                    }),
                )
                .on_a11y_action(AccessibleAction::Increment, {
                    let state = self.state.clone();
                    move |_, _, cx| state.update(cx, |state, cx| state.step_keyboard_thumb(1., cx))
                })
                .on_a11y_action(AccessibleAction::Decrement, {
                    let state = self.state.clone();
                    move |_, _, cx| state.update(cx, |state, cx| state.step_keyboard_thumb(-1., cx))
                })
                .on_mouse_up(
                    MouseButton::Left,
                    window.listener_for(&self.state, |state, _, _, cx| state.handle_release(cx)),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    window.listener_for(&self.state, |state, _, _, cx| state.handle_release(cx)),
                )
            })
            .children(self.children)
            .a11y_disabled(self.disabled)
    }
}

/// An unstyled track that records the geometry used to map pointer positions.
#[derive(IntoElement)]
pub struct SliderTrack {
    state: Entity<SliderState>,
    axis: Axis,
    disabled: bool,
    base: Div,
    children: Vec<AnyElement>,
}

impl SliderTrack {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            axis: Axis::Horizontal,
            disabled: false,
            base: div(),
            children: Vec::new(),
        }
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for SliderTrack {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for SliderTrack {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SliderTrack {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for SliderTrack {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let axis = self.axis;
        let entity_id = self.state.entity_id();
        let state = self.state.read(cx);
        let is_range = state.value().is_range();
        let percentage = state.percentage();
        // Touch: GPUI routes a finger drag to `TouchDragEvent` (claimed with
        // `prevent_default`) or to scrolling; `on_drag` below is mouse-only. Claim
        // drags that start on the track, as the scrollbar thumb does.
        let touch_layer = (!self.disabled).then(|| {
            let slider_state = self.state.clone();
            gpui::canvas(
                |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                move |_, hitbox, window, _| {
                    let slider_state = slider_state.clone();
                    window.on_mouse_event(move |event: &TouchDragEvent, phase, window, cx| {
                        if !phase.bubble() {
                            return;
                        }
                        slider_state.update(cx, |state, cx| {
                            if event.phase == TouchPhase::Started {
                                // A drag whose end never arrived (the slider was
                                // disabled or not painted) ends when another begins.
                                if state.touch_drag.take().is_some() {
                                    state.handle_release(cx);
                                }
                                // The hitbox, not the bounds, so a layer covering
                                // the track keeps its own drags.
                                if window.default_prevented() || !hitbox.is_hovered(window) {
                                    return;
                                }
                                window.prevent_default();
                                let is_start =
                                    is_range && state.is_nearer_start(axis, event.start_position);
                                state.touch_drag = Some(TouchDrag {
                                    start_position: event.start_position,
                                    is_start,
                                });
                            }
                            let Some(drag) = state.touch_drag else {
                                return;
                            };
                            if drag.start_position != event.start_position {
                                return;
                            }
                            cx.stop_propagation();
                            match event.phase {
                                TouchPhase::Started | TouchPhase::Moved => {
                                    state.update_value_by_position(
                                        axis,
                                        event.position,
                                        drag.is_start,
                                        window,
                                        cx,
                                    );
                                }
                                TouchPhase::Ended | TouchPhase::Cancelled => {
                                    state.touch_drag = None;
                                    state.handle_release(cx);
                                }
                            }
                        });
                    });
                },
            )
            .absolute()
            .size_full()
        });
        self.base
            .id("slider-bar-container")
            .test_support()
            .children(self.children)
            .children(touch_layer)
            .when(!self.disabled, |this| {
                this.on_mouse_down(
                    MouseButton::Left,
                    window.listener_for(
                        &self.state,
                        move |state, event: &MouseDownEvent, window, cx| {
                            let is_start = if is_range {
                                let size = state.bounds().size.along(axis);
                                let position = if axis.is_horizontal() {
                                    event.position.x - state.bounds().left()
                                } else {
                                    state.bounds().bottom() - event.position.y
                                };
                                let center = ((percentage.end - percentage.start) / 2.
                                    + percentage.start)
                                    * size;
                                position < center
                            } else {
                                false
                            };
                            state.update_value_by_position(
                                axis,
                                event.position,
                                is_start,
                                window,
                                cx,
                            );
                        },
                    ),
                )
                .when(!is_range, |this| {
                    this.on_drag(DragSlider(entity_id), |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    })
                    .on_drag_move(window.listener_for(
                        &self.state,
                        move |state, event: &DragMoveEvent<DragSlider>, window, cx| {
                            let DragSlider(id) = event.drag(cx);
                            if *id == entity_id {
                                state.update_value_by_position(
                                    axis,
                                    event.event.position,
                                    false,
                                    window,
                                    cx,
                                );
                            }
                        },
                    ))
                })
            })
    }
}

/// An unstyled slider indicator that records the value-mapping bounds.
#[derive(IntoElement)]
pub struct SliderIndicator {
    state: Entity<SliderState>,
    base: Div,
    children: Vec<AnyElement>,
}

impl SliderIndicator {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            base: div(),
            children: Vec::new(),
        }
    }
}

impl ParentElement for SliderIndicator {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for SliderIndicator {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SliderIndicator {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for SliderIndicator {}

impl RenderOnce for SliderIndicator {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .id("slider-bar")
            .children(self.children)
            .on_prepaint({
                let state = self.state;
                // Only when they moved: with retained views, an update while
                // the window draws renders every view that reads the state
                // again.
                move |bounds, _, cx| {
                    if state.read(cx).bounds != bounds {
                        state.update(cx, |state, _| state.set_bounds(bounds));
                    }
                }
            })
    }
}

/// An unstyled draggable slider thumb.
#[derive(IntoElement)]
pub struct SliderThumb {
    state: Entity<SliderState>,
    axis: Axis,
    start: bool,
    disabled: bool,
    base: Div,
    children: Vec<AnyElement>,
}

impl SliderThumb {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            axis: Axis::Horizontal,
            start: false,
            disabled: false,
            base: div(),
            children: Vec::new(),
        }
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn start(mut self, start: bool) -> Self {
        self.start = start;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for SliderThumb {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for SliderThumb {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SliderThumb {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for SliderThumb {}

impl RenderOnce for SliderThumb {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        let entity_id = self.state.entity_id();
        let axis = self.axis;
        let start = self.start;
        self.base
            .id(("slider-thumb", start as u32))
            .test_support()
            .children(self.children)
            .when(!self.disabled, |this| {
                this.on_mouse_down(
                    MouseButton::Left,
                    window.listener_for(&self.state, move |state, _, _, cx| {
                        state.keyboard_start = start;
                        cx.stop_propagation();
                    }),
                )
                .on_drag(DragThumb((entity_id, start)), |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(window.listener_for(
                    &self.state,
                    move |state, event: &DragMoveEvent<DragThumb>, window, cx| {
                        let DragThumb((id, start)) = event.drag(cx);
                        if *id == entity_id {
                            state.update_value_by_position(
                                axis,
                                event.event.position,
                                *start,
                                window,
                                cx,
                            );
                        }
                    },
                ))
            })
    }
}

impl EventEmitter<SliderEvent> for SliderState {}

impl SliderState {
    #[doc(hidden)]
    pub fn percentage(&self) -> Range<f32> {
        self.percentage.clone()
    }

    #[doc(hidden)]
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    #[doc(hidden)]
    pub fn set_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = bounds;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_value_conversions_and_clamping_are_preserved() {
        assert_eq!(SliderValue::from(5.), SliderValue::Single(5.));
        assert_eq!(SliderValue::from((2., 8.)), SliderValue::Range(2., 8.));
        assert_eq!(SliderValue::from(2.0..8.0), SliderValue::Range(2., 8.));
        assert_eq!(
            SliderValue::Range(-1., 12.).clamp(0., 10.),
            SliderValue::Range(0., 10.)
        );
    }

    #[test]
    fn legacy_linear_state_keeps_percentage_and_range_ordering() {
        let state = SliderState::new()
            .min(0.)
            .max(200.)
            .default_value((50., 150.));
        assert_eq!(state.value(), SliderValue::Range(50., 150.));
        assert_eq!(state.percentage(), 0.25..0.75);
    }

    #[test]
    fn legacy_logarithmic_state_keeps_mapping() {
        let state = SliderState::new()
            .min(1.)
            .max(1000.)
            .scale(SliderScale::Logarithmic)
            .default_value(10.);
        let percentage = state.percentage().end;
        assert!((percentage - (1. / 3.)).abs() < 0.0001);
    }

    #[test]
    #[should_panic(expected = "`min` must be greater than 0")]
    fn legacy_logarithmic_validation_is_preserved() {
        let _ = SliderState::new().scale(SliderScale::Logarithmic);
    }

    /// A focused slider moved with the keyboard.
    mod keyboard {
        use std::{cell::RefCell, rc::Rc};

        use gpui::{
            AppContext as _, Context, Entity, FocusHandle, IntoElement, Modifiers,
            ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext, Window,
            point, px,
        };

        use super::super::{
            Slider, SliderEvent, SliderIndicator, SliderState, SliderTrack, SliderValue, init,
        };

        /// A slider 100px wide, so a pixel is a step from 0 to 100.
        struct Harness {
            state: Entity<SliderState>,
            focus: FocusHandle,
            disabled: bool,
        }

        impl Render for Harness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                Slider::new(&self.state)
                    .track_focus(&self.focus)
                    .disabled(self.disabled)
                    .w(px(100.))
                    .h(px(20.))
                    .child(
                        SliderTrack::new(&self.state)
                            .disabled(self.disabled)
                            .size_full()
                            .child(SliderIndicator::new(&self.state).size_full()),
                    )
            }
        }

        type Events = Rc<RefCell<Vec<(&'static str, SliderValue)>>>;

        fn open(
            cx: &mut TestAppContext,
            value: impl Into<SliderValue>,
            disabled: bool,
        ) -> (
            Entity<SliderState>,
            FocusHandle,
            &mut VisualTestContext,
            Events,
        ) {
            cx.update(init);
            let state = cx.new(|_| SliderState::new().default_value(value));
            let events = Events::default();
            cx.update(|cx| {
                let events = events.clone();
                cx.subscribe(&state, move |_, event: &SliderEvent, _| {
                    events.borrow_mut().push(match event {
                        SliderEvent::Change(value) => ("change", *value),
                        SliderEvent::Release(value) => ("release", *value),
                    })
                })
                .detach();
            });
            let focus = cx.update(|cx| cx.focus_handle());
            let (_, cx) = cx.add_window_view({
                let state = state.clone();
                let focus = focus.clone();
                move |_, _| Harness {
                    state,
                    focus,
                    disabled,
                }
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            (state, focus, cx, events)
        }

        /// The events since the last call, as `(name, value)` pairs.
        fn take(events: &Events) -> Vec<(&'static str, SliderValue)> {
            std::mem::take(&mut *events.borrow_mut())
        }

        /// Each key press is one whole change: `change` then `release`.
        fn pressed(value: impl Into<SliderValue>) -> Vec<(&'static str, SliderValue)> {
            let value = value.into();
            vec![("change", value), ("release", value)]
        }

        #[gpui::test]
        fn keys_step_page_and_jump_to_the_ends(cx: &mut TestAppContext) {
            let (state, focus, cx, events) = open(cx, 50., false);
            cx.update(|window, cx| window.focus(&focus, cx));

            for (key, value) in [
                ("right", 51.),
                ("up", 52.),
                ("left", 51.),
                ("down", 50.),
                ("pageup", 60.),
                ("pagedown", 50.),
                ("home", 0.),
                ("end", 100.),
            ] {
                cx.simulate_keystrokes(key);
                assert_eq!(take(&events), pressed(value), "{key}");
            }
            // At the maximum, a further increase changes nothing.
            cx.simulate_keystrokes("right pageup end");
            assert!(take(&events).is_empty());
            assert_eq!(state.read_with(cx, |state, _| state.percentage()), 0.0..1.0);
        }

        #[gpui::test]
        fn a_click_focuses_the_slider(cx: &mut TestAppContext) {
            let (state, focus, cx, events) = open(cx, 0., false);
            cx.simulate_click(point(px(30.), px(10.)), Modifiers::default());
            assert_eq!(take(&events), pressed(30.));
            cx.update(|window, _| assert!(focus.is_focused(window)));
            cx.simulate_keystrokes("right");
            assert_eq!(take(&events), pressed(31.));
            assert_eq!(
                state.read_with(cx, |state, _| state.value()),
                SliderValue::Single(31.)
            );
        }

        #[gpui::test]
        fn keys_move_the_range_thumb_the_pointer_moved_last(cx: &mut TestAppContext) {
            let (_, focus, cx, events) = open(cx, (20., 80.), false);
            cx.update(|window, cx| window.focus(&focus, cx));
            // The end thumb until the pointer moves the start one.
            cx.simulate_keystrokes("right");
            assert_eq!(take(&events), pressed((20., 81.)));

            cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
            assert_eq!(take(&events), pressed((10., 81.)));
            cx.simulate_keystrokes("left");
            assert_eq!(take(&events), pressed((9., 81.)));
            // The start thumb stops at the end one.
            cx.simulate_keystrokes("end");
            assert_eq!(take(&events), pressed((81., 81.)));
        }

        #[gpui::test]
        fn a_disabled_slider_takes_no_keys(cx: &mut TestAppContext) {
            let (state, focus, cx, events) = open(cx, 50., true);
            cx.simulate_click(point(px(30.), px(10.)), Modifiers::default());
            cx.update(|window, _| assert!(!focus.is_focused(window)));
            cx.update(|window, cx| window.focus(&focus, cx));
            cx.simulate_keystrokes("right home");
            assert!(take(&events).is_empty());
            assert_eq!(
                state.read_with(cx, |state, _| state.value()),
                SliderValue::Single(50.)
            );
        }
    }

    /// A slider's bar beside a view that reads its state, under retained views.
    #[cfg(feature = "gpui-fast")]
    mod retained {
        use std::{cell::Cell, rc::Rc};

        use gpui::{
            AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
            TestAppContext, Window, div, px,
        };

        use super::super::{SliderIndicator, SliderState};

        /// A view that shows a slider's value, counting its renders.
        struct Readout {
            state: Entity<SliderState>,
            renders: Rc<Cell<usize>>,
        }

        impl Render for Readout {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                self.renders.set(self.renders.get() + 1);
                div().child(self.state.read(cx).value().to_string())
            }
        }

        /// A slider's bar in a view of its own.
        struct Bar {
            state: Entity<SliderState>,
        }

        impl Render for Bar {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
                    .w(px(120.))
                    .h(px(24.))
                    .child(SliderIndicator::new(&self.state).size_full())
            }
        }

        /// The bar and the readout side by side.
        struct Pair {
            bar: Entity<Bar>,
            readout: Entity<Readout>,
        }

        impl Render for Pair {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
                    .flex()
                    .child(self.bar.clone())
                    .child(self.readout.clone())
            }
        }

        /// Drawing a slider's bar again where it was changes nothing in the
        /// slider's state: a view beside it that reads the state is not built
        /// again for it.
        #[gpui::test]
        fn drawing_a_slider_again_leaves_its_readers_retained(cx: &mut TestAppContext) {
            let renders = Rc::new(Cell::new(0));
            let (view, cx) = cx.add_window_view({
                let renders = renders.clone();
                move |_, cx| {
                    let state = cx.new(|_| SliderState::new());
                    Pair {
                        bar: cx.new(|_| Bar {
                            state: state.clone(),
                        }),
                        readout: cx.new(|_| Readout { state, renders }),
                    }
                }
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let rendered = renders.get();
            for _ in 0..3 {
                view.update(cx, |pair, cx| pair.bar.update(cx, |_, cx| cx.notify()));
                cx.update(|window, cx| window.draw(cx).clear(cx));
            }
            cx.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(renders.get(), rendered, "the readout was built again");
        }
    }
}
