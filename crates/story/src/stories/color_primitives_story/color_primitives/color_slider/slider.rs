pub use super::color_thumb::ThumbShape;
use super::color_thumb::{ColorThumb, ThumbAxis, TrackEndcaps, bar_main_axis_size};
use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, ElementExt, Sizable, Size, StyledExt as _};

pub mod sizing {
    pub const TRACK_THICKNESS_XSMALL: f32 = 4.0;
    pub const TRACK_THICKNESS_SMALL: f32 = 14.0;
    pub const TRACK_THICKNESS_MEDIUM: f32 = 24.0;
    pub const TRACK_THICKNESS_LARGE: f32 = 34.0;

    pub const THUMB_SIZE_XSMALL: f32 = 10.0;
    pub const THUMB_SIZE_SMALL: f32 = 12.0;
    pub const THUMB_SIZE_MEDIUM: f32 = 20.0;
    pub const THUMB_SIZE_LARGE: f32 = 30.0;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ColorInterpolation {
    #[default]
    Rgb,
    Hsl,
    Lab,
}

pub trait ColorSliderDelegate: 'static {
    fn style_background(
        &self,
        slider: &ColorSliderState,
        container: Div,
        window: &mut Window,
        cx: &App,
    ) -> Div;

    /// Get the color at a specific position (0.0 to 1.0) on the slider.
    fn get_color_at_position(&self, slider: &ColorSliderState, position: f32) -> Hsla;

    fn interpolation_method(&self, slider: &ColorSliderState) -> ColorInterpolation {
        slider.interpolation
    }
}

fn normalized_value_percent(value: f32, start: f32, end: f32) -> f32 {
    let span = end - start;
    if span.abs() <= f32::EPSILON {
        return 0.0;
    }

    ((value - start) / span).clamp(0.0, 1.0)
}

#[derive(Clone)]
struct ColorSliderDrag(EntityId);

impl Render for ColorSliderDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ThumbPosition {
    /// Thumb stays within slider bounds (default behavior)
    #[default]
    InsideSlider,
    /// Thumb centerline aligns with slider ends (half thumb extends past ends)
    EdgeToEdge,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ThumbSize {
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColorSliderEvent {
    Change(f32),
    Release(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ThumbConfig {
    pub position: ThumbPosition,
    pub size: ThumbSize,
    pub shape: ThumbShape,
}

pub struct SliderDimensions {
    pub axis: Axis,
    pub size: Size,
    pub bounds: Bounds<Pixels>,
}

impl Default for SliderDimensions {
    fn default() -> Self {
        Self {
            axis: Axis::Horizontal,
            size: Size::Medium,
            bounds: Bounds::default(),
        }
    }
}

/// State for [`ColorSlider`].
pub struct ColorSliderState {
    pub id: SharedString,
    pub value: f32,
    pub range: std::ops::Range<f32>,
    pub step: Option<f32>,
    pub reversed: bool,
    pub thumb: ThumbConfig,
    pub dimensions: SliderDimensions,
    pub delegate: Box<dyn ColorSliderDelegate>,
    pub style: StyleRefinement,
    pub interpolation: ColorInterpolation,
    pub disabled: bool,
    pub focus_handle: FocusHandle,
    interaction_active: bool,
}

impl ColorSliderState {
    pub fn new<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        delegate: Box<dyn ColorSliderDelegate>,
        cx: &mut Context<V>,
    ) -> Self {
        Self {
            id: id.into(),
            value,
            range: 0.0..1.0,
            step: None,
            reversed: false,
            thumb: ThumbConfig::default(),
            dimensions: SliderDimensions::default(),
            delegate,
            style: StyleRefinement::default(),
            interpolation: ColorInterpolation::default(),
            disabled: false,
            focus_handle: cx.focus_handle(),
            interaction_active: false,
        }
    }

    pub fn hue<V: 'static>(id: impl Into<SharedString>, value: f32, cx: &mut Context<V>) -> Self {
        Self::new(id, value, Box::new(super::delegates::HueDelegate), cx).max(360.0)
    }

    pub fn channel<S: super::color_spec::ColorSpecification, V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        delegate: super::delegates::ChannelDelegate<S>,
        cx: &mut Context<V>,
    ) -> Self {
        Self::new(id, value, Box::new(delegate), cx)
    }

    pub fn alpha<S: super::color_spec::ColorSpecification, V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        delegate: super::delegates::AlphaDelegate<S>,
        cx: &mut Context<V>,
    ) -> Self {
        Self::new(id, value, Box::new(delegate), cx)
    }

    pub fn gradient<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        colors: Vec<Hsla>,
        cx: &mut Context<V>,
    ) -> Self {
        Self::new(
            id,
            value,
            Box::new(super::delegates::GradientDelegate { colors }),
            cx,
        )
    }

    pub fn horizontal(mut self) -> Self {
        self.dimensions.axis = Axis::Horizontal;
        self
    }

    #[allow(dead_code)] // Kept for API parity with Slider orientation builders.
    pub fn vertical(mut self) -> Self {
        self.dimensions.axis = Axis::Vertical;
        self
    }

    pub fn min(mut self, min: f32) -> Self {
        self.range.start = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.range.end = max;
        self
    }

    pub fn set_range(&mut self, min: f32, max: f32, cx: &mut Context<Self>) {
        let range_changed = self.range.start != min || self.range.end != max;
        if range_changed {
            self.range.start = min;
            self.range.end = max;
        }

        let clamped_value = self.clamp_to_range(self.value);
        let value_changed = self.value != clamped_value;
        if value_changed {
            self.value = clamped_value;
        }

        if range_changed || value_changed {
            cx.notify();
        }
    }

    pub fn reversed(mut self, reversed: bool) -> Self {
        self.reversed = reversed;
        self
    }

    #[allow(dead_code)] // Runtime setter kept for API parity with builder-based configuration.
    pub fn set_reversed(&mut self, reversed: bool, cx: &mut Context<Self>) {
        if self.reversed != reversed {
            self.reversed = reversed;
            cx.notify();
        }
    }

    #[allow(dead_code)] // Runtime setter kept for parity with other interactive controls.
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        if self.disabled != disabled {
            self.disabled = disabled;
            cx.notify();
        }
    }

    pub fn edge_to_edge(mut self) -> Self {
        self.thumb.position = ThumbPosition::EdgeToEdge;
        self
    }

    pub fn thumb_xsmall(mut self) -> Self {
        self.thumb.size = ThumbSize::XSmall;
        self
    }

    pub fn thumb_small(mut self) -> Self {
        self.thumb.size = ThumbSize::Small;
        self
    }

    pub fn thumb_medium(mut self) -> Self {
        self.thumb.size = ThumbSize::Medium;
        self
    }

    pub fn thumb_large(mut self) -> Self {
        self.thumb.size = ThumbSize::Large;
        self
    }

    pub fn thumb_square(mut self) -> Self {
        self.thumb.shape = ThumbShape::Square;
        self
    }

    pub fn interpolation(mut self, interpolation: ColorInterpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    #[allow(dead_code)] // Runtime setter retained for dynamic interpolation switching.
    pub fn set_interpolation(&mut self, interpolation: ColorInterpolation, cx: &mut Context<Self>) {
        if self.interpolation != interpolation {
            self.interpolation = interpolation;
            cx.notify();
        }
    }

    pub fn set_value(&mut self, value: f32, cx: &mut Context<Self>) {
        let clamped_value = self.clamp_to_range(value);
        if self.value != clamped_value {
            self.value = clamped_value;
            cx.notify();
        }
    }

    pub fn set_size(&mut self, size: Size, cx: &mut Context<Self>) {
        let synced_thumb = Self::synced_thumb_size(&size);
        let needs_thumb_update = synced_thumb.is_some_and(|thumb| self.thumb.size != thumb);

        if self.dimensions.size != size || needs_thumb_update {
            self.dimensions.size = size;
            if let Some(thumb) = synced_thumb {
                self.thumb.size = thumb;
            }
            cx.notify();
        }
    }

    #[allow(dead_code)] // Runtime setter retained for external thumb style controls.
    pub fn set_thumb_size(&mut self, size: ThumbSize, cx: &mut Context<Self>) {
        if self.thumb.size != size {
            self.thumb.size = size;
            cx.notify();
        }
    }

    #[allow(dead_code)] // Runtime setter retained for external thumb style controls.
    pub fn set_thumb_position(&mut self, position: ThumbPosition, cx: &mut Context<Self>) {
        if self.thumb.position != position {
            self.thumb.position = position;
            cx.notify();
        }
    }

    #[allow(dead_code)] // Runtime setter retained for external thumb style controls.
    pub fn set_thumb_shape(&mut self, shape: ThumbShape, cx: &mut Context<Self>) {
        if self.thumb.shape != shape {
            self.thumb.shape = shape;
            cx.notify();
        }
    }

    pub fn set_corner_radius(&mut self, radius: AbsoluteLength, cx: &mut Context<Self>) {
        self.style.corner_radii.top_left = Some(radius);
        self.style.corner_radii.top_right = Some(radius);
        self.style.corner_radii.bottom_left = Some(radius);
        self.style.corner_radii.bottom_right = Some(radius);
        cx.notify();
    }

    #[allow(dead_code)] // Runtime counterpart to `set_corner_radius`, kept for API completeness.
    pub fn clear_corner_radius(&mut self, cx: &mut Context<Self>) {
        let had_custom = self.style.corner_radii.top_left.is_some()
            || self.style.corner_radii.top_right.is_some()
            || self.style.corner_radii.bottom_left.is_some()
            || self.style.corner_radii.bottom_right.is_some();

        if had_custom {
            self.style.corner_radii.top_left = None;
            self.style.corner_radii.top_right = None;
            self.style.corner_radii.bottom_left = None;
            self.style.corner_radii.bottom_right = None;
            cx.notify();
        }
    }

    #[allow(dead_code)] // Runtime setter kept for orientation parity with builder API.
    pub fn set_axis(&mut self, axis: Axis, cx: &mut Context<Self>) {
        if self.dimensions.axis != axis {
            self.dimensions.axis = axis;
            cx.notify();
        }
    }

    pub fn set_delegate(&mut self, delegate: Box<dyn ColorSliderDelegate>, cx: &mut Context<Self>) {
        self.delegate = delegate;
        cx.notify();
    }

    pub fn track_thickness(&self) -> f32 {
        match self.dimensions.size {
            Size::XSmall => sizing::TRACK_THICKNESS_XSMALL,
            Size::Small => sizing::TRACK_THICKNESS_SMALL,
            Size::Medium => sizing::TRACK_THICKNESS_MEDIUM,
            Size::Large => sizing::TRACK_THICKNESS_LARGE,
            Size::Size(base_px) => base_px.as_f32(),
        }
    }

    pub fn thumb_size(&self) -> f32 {
        match self.thumb.size {
            ThumbSize::XSmall => sizing::THUMB_SIZE_XSMALL,
            ThumbSize::Small => sizing::THUMB_SIZE_SMALL,
            ThumbSize::Medium => sizing::THUMB_SIZE_MEDIUM,
            ThumbSize::Large => sizing::THUMB_SIZE_LARGE,
        }
    }

    pub fn track_inset(&self) -> f32 {
        match self.effective_thumb_position() {
            ThumbPosition::InsideSlider => 0.0,
            ThumbPosition::EdgeToEdge => self.thumb_main_axis_size() / 2.0,
        }
    }
}

impl ColorSliderState {
    fn synced_thumb_size(size: &Size) -> Option<ThumbSize> {
        match size {
            Size::XSmall => Some(ThumbSize::XSmall),
            Size::Small => Some(ThumbSize::Small),
            Size::Medium => Some(ThumbSize::Medium),
            Size::Large => Some(ThumbSize::Large),
            Size::Size(_) => None,
        }
    }

    fn clamp_to_range(&self, value: f32) -> f32 {
        value.clamp(
            self.range.start.min(self.range.end),
            self.range.end.max(self.range.start),
        )
    }

    fn effective_thumb_position(&self) -> ThumbPosition {
        let hint = self.thumb.shape.layout_hint();
        if hint.supported_positions.contains(&self.thumb.position) {
            self.thumb.position
        } else {
            hint.preferred_position
        }
    }

    fn thumb_main_axis_size(&self) -> f32 {
        match self.thumb.shape {
            ThumbShape::Bar => bar_main_axis_size(px(self.thumb_size())).as_f32(),
            ThumbShape::Circle | ThumbShape::Square => self.thumb_size(),
        }
    }

    fn update_from_mouse(
        &mut self,
        position: Point<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let size = if self.dimensions.axis == Axis::Vertical {
            self.dimensions.bounds.size.height
        } else {
            self.dimensions.bounds.size.width
        };

        if size <= px(0.0) {
            return;
        }

        let inset = px(self.track_inset());
        let track_length = size - inset * 2.0;

        if track_length <= px(0.0) {
            return;
        }

        let local_pos = if self.dimensions.axis == Axis::Vertical {
            position.y - self.dimensions.bounds.origin.y
        } else {
            position.x - self.dimensions.bounds.origin.x
        };

        let mut percentage = ((local_pos - inset) / track_length).clamp(0.0, 1.0);

        if self.reversed {
            percentage = 1.0 - percentage;
        }

        let mut value = self.range.start + (self.range.end - self.range.start) * percentage;

        if let Some(step) = self.step {
            let step = step.abs();
            if step > f32::EPSILON {
                // Snap relative to the configured range start, not absolute zero.
                value = self.range.start + ((value - self.range.start) / step).round() * step;
            }
        }

        self.value = value.clamp(
            self.range.start.min(self.range.end),
            self.range.end.max(self.range.start),
        );
        cx.emit(ColorSliderEvent::Change(self.value));
        cx.notify();
    }

    fn emit_release(&self, cx: &mut Context<Self>) {
        cx.emit(ColorSliderEvent::Release(self.value));
    }

    fn begin_interaction(&mut self) {
        self.interaction_active = true;
    }

    fn end_interaction(&mut self, cx: &mut Context<Self>) {
        if !self.interaction_active {
            return;
        }
        self.interaction_active = false;
        self.emit_release(cx);
    }
}

impl Styled for ColorSliderState {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for ColorSliderState {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        let size = size.into();
        self.dimensions.size = size;
        if let Some(thumb) = Self::synced_thumb_size(&self.dimensions.size) {
            self.thumb.size = thumb;
        }
        self
    }
}

impl EventEmitter<ColorSliderEvent> for ColorSliderState {}

impl Render for ColorSliderState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ColorSlider::new(&cx.entity())
    }
}

#[derive(IntoElement)]
pub struct ColorSlider {
    state: Entity<ColorSliderState>,
}

#[derive(Clone, Copy)]
struct SliderLayout {
    is_vertical: bool,
    track_thickness: f32,
    thumb_size: f32,
    track_inset: f32,
    track_hitsize: f32,
    thumb_pos_pct: f32,
    thumb_main_adjust: Pixels,
}

impl ColorSlider {
    pub fn new(state: &Entity<ColorSliderState>) -> Self {
        Self {
            state: state.clone(),
        }
    }

    fn compute_layout(state: &ColorSliderState) -> SliderLayout {
        let track_thickness = state.track_thickness();
        let thumb_size = state.thumb_size();
        let track_inset = state.track_inset();
        let is_vertical = state.dimensions.axis == Axis::Vertical;
        let track_hitsize = track_thickness.max(thumb_size);

        let thumb_pos_pct =
            normalized_value_percent(state.value, state.range.start, state.range.end);
        let thumb_pos_pct = if state.reversed {
            1.0 - thumb_pos_pct
        } else {
            thumb_pos_pct
        };

        let thumb_main_adjust = px(state.thumb_main_axis_size() * thumb_pos_pct);

        SliderLayout {
            is_vertical,
            track_thickness,
            thumb_size,
            track_inset,
            track_hitsize,
            thumb_pos_pct,
            thumb_main_adjust,
        }
    }

    fn resolve_track_radius(state: &ColorSliderState, rem_size: Pixels) -> Corners<Pixels> {
        let corner_radii = state.style.corner_radii.clone();
        let uses_custom_corner_radii = corner_radii.top_left.is_some()
            || corner_radii.top_right.is_some()
            || corner_radii.bottom_left.is_some()
            || corner_radii.bottom_right.is_some();
        let default_radius = if uses_custom_corner_radii {
            px(999.)
        } else {
            match state.thumb.shape.layout_hint().preferred_track_endcaps {
                TrackEndcaps::Rounded => px(999.),
            }
        };

        Corners {
            top_left: corner_radii
                .top_left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
            top_right: corner_radii
                .top_right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
            bottom_left: corner_radii
                .bottom_left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
            bottom_right: corner_radii
                .bottom_right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
        }
    }

    fn track_frame(layout: SliderLayout, radius: Corners<Pixels>, border_color: Hsla) -> Div {
        div()
            .absolute()
            .when(layout.is_vertical, |this| {
                this.w(px(layout.track_thickness))
                    .left(px((layout.track_hitsize - layout.track_thickness) / 2.0))
                    .top(px(layout.track_inset))
                    .bottom(px(layout.track_inset))
            })
            .when(!layout.is_vertical, |this| {
                this.h(px(layout.track_thickness))
                    .top(px((layout.track_hitsize - layout.track_thickness) / 2.0))
                    .left(px(layout.track_inset))
                    .right(px(layout.track_inset))
            })
            .corner_radii(radius)
            .overflow_hidden()
            .border_1()
            .border_color(border_color)
    }

    fn disabled_overlay(layout: SliderLayout, radius: Corners<Pixels>, color: Hsla) -> Div {
        div()
            .absolute()
            .when(layout.is_vertical, |this| {
                this.w(px(layout.track_thickness))
                    .left(px((layout.track_hitsize - layout.track_thickness) / 2.0))
                    .top(px(layout.track_inset))
                    .bottom(px(layout.track_inset))
            })
            .when(!layout.is_vertical, |this| {
                this.h(px(layout.track_thickness))
                    .top(px((layout.track_hitsize - layout.track_thickness) / 2.0))
                    .left(px(layout.track_inset))
                    .right(px(layout.track_inset))
            })
            .corner_radii(radius)
            .bg(color)
    }

    fn thumb_fill_color(state: &ColorSliderState, layout: SliderLayout) -> Option<Hsla> {
        if state.thumb.shape == ThumbShape::Bar {
            return Some(
                state
                    .delegate
                    .get_color_at_position(state, layout.thumb_pos_pct),
            );
        }

        let thumb_extends_beyond_track = layout.thumb_size > layout.track_thickness;
        match state.effective_thumb_position() {
            ThumbPosition::InsideSlider if !thumb_extends_beyond_track => None,
            _ => Some(
                state
                    .delegate
                    .get_color_at_position(state, layout.thumb_pos_pct),
            ),
        }
    }

    fn thumb_element(
        state: &ColorSliderState,
        layout: SliderLayout,
        fill_color: Option<Hsla>,
    ) -> Div {
        let thumb_axis = if layout.is_vertical {
            ThumbAxis::Vertical
        } else {
            ThumbAxis::Horizontal
        };
        let thumb_cross_offset_outer = px((layout.track_hitsize - layout.thumb_size) / 2.0);

        div()
            .absolute()
            .when(layout.is_vertical, |this| {
                this.left(thumb_cross_offset_outer)
                    .top(relative(layout.thumb_pos_pct))
                    .mt(-layout.thumb_main_adjust)
            })
            .when(!layout.is_vertical, |this| {
                this.top(thumb_cross_offset_outer)
                    .left(relative(layout.thumb_pos_pct))
                    .ml(-layout.thumb_main_adjust)
            })
            .child(
                ColorThumb::new(px(layout.thumb_size))
                    .shape(state.thumb.shape)
                    .axis(thumb_axis)
                    .active(false)
                    .when_some(fill_color, |this, color| this.color(color)),
            )
    }

    fn value_from_key(state: &ColorSliderState, event: &KeyDownEvent) -> Option<f32> {
        let base_step = state
            .step
            .unwrap_or((state.range.end - state.range.start).abs() / 100.0);
        let multiplier = if event.keystroke.modifiers.shift {
            10.0
        } else if event.keystroke.modifiers.alt {
            0.1
        } else {
            1.0
        };
        let step = base_step * multiplier;
        let is_horizontal = state.dimensions.axis == Axis::Horizontal;
        let reversed = state.reversed;

        match event.keystroke.key.as_str() {
            "left" if is_horizontal => {
                if reversed {
                    Some(state.value + step)
                } else {
                    Some(state.value - step)
                }
            }
            "right" if is_horizontal => {
                if reversed {
                    Some(state.value - step)
                } else {
                    Some(state.value + step)
                }
            }
            "up" if !is_horizontal => {
                if reversed {
                    Some(state.value + step)
                } else {
                    Some(state.value - step)
                }
            }
            "down" if !is_horizontal => {
                if reversed {
                    Some(state.value - step)
                } else {
                    Some(state.value + step)
                }
            }
            "home" => Some(state.range.start),
            "end" => Some(state.range.end),
            _ => None,
        }
    }

    fn key_handler(
        state_entity: Entity<ColorSliderState>,
    ) -> impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static {
        move |event: &KeyDownEvent, _window: &mut Window, cx: &mut App| {
            state_entity.update(cx, |state, cx| {
                if let Some(value) = Self::value_from_key(state, event) {
                    let clamped_value = state.clamp_to_range(value);
                    state.set_value(clamped_value, cx);
                    cx.emit(ColorSliderEvent::Change(clamped_value));
                    cx.emit(ColorSliderEvent::Release(clamped_value));
                    cx.stop_propagation();
                }
            });
        }
    }

    fn prepaint_handler(
        state_entity: Entity<ColorSliderState>,
    ) -> impl Fn(Bounds<Pixels>, &mut Window, &mut App) + 'static {
        move |bounds: Bounds<Pixels>, _: &mut Window, cx: &mut App| {
            state_entity.update(cx, |state, _| {
                if state.dimensions.bounds != bounds {
                    state.dimensions.bounds = bounds;
                }
            })
        }
    }

    fn attach_pointer_interactions(
        root: Stateful<Div>,
        state_entity: Entity<ColorSliderState>,
        drag_id: EntityId,
        window: &mut Window,
    ) -> Stateful<Div> {
        root.child(
            canvas(
                |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                {
                    let state_entity = state_entity.clone();
                    move |_, _, window, _cx| {
                        window.on_mouse_event({
                            let state_entity = state_entity.clone();
                            move |_: &MouseUpEvent, phase, _, cx| {
                                if !phase.bubble() {
                                    return;
                                }
                                let _ = state_entity.update(cx, |state, cx| {
                                    state.end_interaction(cx);
                                });
                            }
                        });
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .on_mouse_down(
            MouseButton::Left,
            window.listener_for(
                &state_entity,
                |state: &mut ColorSliderState,
                 ev: &MouseDownEvent,
                 window: &mut Window,
                 cx: &mut Context<ColorSliderState>| {
                    state.focus_handle.focus(window, cx);
                    state.begin_interaction();
                    state.update_from_mouse(ev.position, window, cx);
                },
            ),
        )
        .on_mouse_up(
            MouseButton::Left,
            window.listener_for(
                &state_entity,
                |state: &mut ColorSliderState,
                 _: &MouseUpEvent,
                 _: &mut Window,
                 cx: &mut Context<ColorSliderState>| {
                    state.end_interaction(cx);
                },
            ),
        )
        .on_drag(ColorSliderDrag(drag_id), |drag, _, _, cx| {
            cx.stop_propagation();
            cx.new(|_| drag.clone())
        })
        .on_drag_move(window.listener_for(
            &state_entity,
            move |state: &mut ColorSliderState,
                  ev: &DragMoveEvent<ColorSliderDrag>,
                  window: &mut Window,
                  cx: &mut Context<ColorSliderState>| {
                if ev.drag(cx).0 != drag_id {
                    return;
                }
                state.update_from_mouse(ev.event.position, window, cx);
            },
        ))
    }
}

impl RenderOnce for ColorSlider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_entity = self.state.clone();
        let entity_id = state_entity.entity_id();
        let state = state_entity.read(cx);
        let control_id = state.id.clone();

        let track_border_color = cx.theme().border;
        let disabled_overlay_color = cx.theme().background.opacity(0.45);

        let layout = Self::compute_layout(&state);
        let radius = Self::resolve_track_radius(&state, window.rem_size());
        let container = Self::track_frame(layout, radius, track_border_color);
        let background_element = state
            .delegate
            .style_background(&state, container, window, cx);
        let fill_color = Self::thumb_fill_color(&state, layout);
        let thumb = Self::thumb_element(&state, layout, fill_color);

        let style = state.style.clone();
        let disabled = state.disabled;

        let mut root = div()
            .id(control_id)
            .when(layout.is_vertical, |this| {
                this.w(px(layout.track_hitsize)).h_full()
            })
            .when(!layout.is_vertical, |this| {
                this.h(px(layout.track_hitsize)).w_full()
            })
            .relative()
            .refine_style(&style)
            .child(background_element)
            .when(disabled, |this| {
                this.child(Self::disabled_overlay(
                    layout,
                    radius,
                    disabled_overlay_color,
                ))
            })
            .when(!disabled, |this| this.child(thumb))
            .track_focus(&state.focus_handle)
            .on_key_down(Self::key_handler(state_entity.clone()))
            .on_prepaint(Self::prepaint_handler(state_entity.clone()));

        if !disabled {
            root = Self::attach_pointer_interactions(root, state_entity.clone(), entity_id, window);
        }

        root
    }
}

#[cfg(test)]
mod tests {
    use super::normalized_value_percent;

    fn approx_eq(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-6, "expected {a} ~= {b}");
    }

    #[test]
    fn normalized_value_percent_handles_regular_and_degenerate_ranges() {
        approx_eq(normalized_value_percent(25.0, 0.0, 100.0), 0.25);
        approx_eq(normalized_value_percent(-10.0, 0.0, 100.0), 0.0);
        approx_eq(normalized_value_percent(150.0, 0.0, 100.0), 1.0);
        approx_eq(normalized_value_percent(10.0, 5.0, 5.0), 0.0);
    }

    #[test]
    fn normalized_value_percent_supports_descending_ranges() {
        approx_eq(normalized_value_percent(1.0, 1.0, 0.0), 0.0);
        approx_eq(normalized_value_percent(0.5, 1.0, 0.0), 0.5);
        approx_eq(normalized_value_percent(0.0, 1.0, 0.0), 1.0);
    }
}
