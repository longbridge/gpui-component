#![allow(dead_code)]

use super::common::{
    arc_geometry, effective_position as resolve_effective_position, pointer_hits_arc_target,
    pointer_to_arc_position, position_to_turn, size_px as component_size_px, start_turns,
    sweep_turns, thumb_top_left, value_from_position, value_percent,
};
use crate::stories::color_primitives_story::color_slider::color_thumb::{ColorThumb, ThumbShape};
use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, ElementExt, PixelsExt as _, Sizable, Size, StyledExt as _};

pub mod sizing {
    pub const ARC_THICKNESS_XSMALL: f32 = 7.0;
    pub const ARC_THICKNESS_SMALL: f32 = 14.0;
    pub const ARC_THICKNESS_MEDIUM: f32 = 20.0;
    pub const ARC_THICKNESS_LARGE: f32 = 28.0;

    pub const THUMB_SIZE_SMALL: f32 = 12.0;
    pub const THUMB_SIZE_MEDIUM: f32 = 16.0;
    pub const THUMB_SIZE_LARGE: f32 = 20.0;
}

pub trait ColorArcDelegate: 'static {
    fn style_background(
        &self,
        arc: &ColorArcState,
        container: Div,
        window: &mut Window,
        cx: &App,
    ) -> Div;

    fn get_color_at_position(&self, arc: &ColorArcState, position: f32) -> Hsla;

    fn position_to_value(&self, _arc: &ColorArcState, position: f32) -> f32 {
        position.clamp(0.0, 1.0)
    }

    fn value_to_position(&self, _arc: &ColorArcState, value: f32) -> f32 {
        value.clamp(0.0, 1.0)
    }

    fn prewarm_raster_cache(
        &self,
        _arc: &ColorArcState,
        _image_size: gpui::Size<Pixels>,
        _border_color: Hsla,
    ) {
    }

    fn renderer(&self) -> super::raster::ColorArcRenderer {
        super::raster::ColorArcRenderer::Vector
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColorArcEvent {
    Change(f32),
    Release(f32),
}

pub struct ColorArcState {
    pub id: SharedString,
    pub value: f32,
    pub range: std::ops::Range<f32>,
    pub step: Option<f32>,
    pub reversed: bool,
    pub size: Size,
    pub arc_thickness: Option<f32>,
    pub arc_thickness_size: Option<Size>,
    pub thumb_size: Option<f32>,
    pub bounds: Bounds<Pixels>,
    pub delegate: Box<dyn ColorArcDelegate>,
    pub style: StyleRefinement,
    pub disabled: bool,
    pub start_degrees: f32,
    pub sweep_degrees: f32,
    interaction_position: Option<f32>,
    drag_interaction_enabled: bool,
    pub focus_handle: FocusHandle,
}

impl ColorArcState {
    fn clamp_to_range(&self, value: f32) -> f32 {
        value.clamp(
            self.range.start.min(self.range.end),
            self.range.end.max(self.range.start),
        )
    }

    pub fn new<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        delegate: Box<dyn ColorArcDelegate>,
        cx: &mut Context<V>,
    ) -> Self {
        Self {
            id: id.into(),
            value,
            range: 0.0..1.0,
            step: None,
            reversed: false,
            size: Size::Medium,
            arc_thickness: None,
            arc_thickness_size: None,
            thumb_size: None,
            bounds: Bounds::default(),
            delegate,
            style: StyleRefinement::default(),
            disabled: false,
            start_degrees: 0.0,
            sweep_degrees: 180.0,
            interaction_position: None,
            drag_interaction_enabled: false,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn hue<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        delegate: super::delegates::HueArcDelegate,
        cx: &mut Context<V>,
    ) -> Self {
        // Raster-first default for performance; use `*_with_renderer(..., Vector, ...)` to opt in.
        Self::hue_raster(id, value, delegate.saturation, delegate.lightness, cx)
    }

    pub fn saturation<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        delegate: super::delegates::SaturationArcDelegate,
        cx: &mut Context<V>,
    ) -> Self {
        // Raster-first default for performance; use `*_with_renderer(..., Vector, ...)` to opt in.
        Self::saturation_raster(id, value, delegate.hue, delegate.hsv_value, cx)
    }

    pub fn lightness<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        delegate: super::delegates::LightnessArcDelegate,
        cx: &mut Context<V>,
    ) -> Self {
        // Raster-first default for performance; use `*_with_renderer(..., Vector, ...)` to opt in.
        Self::lightness_raster(id, value, delegate.hue, delegate.saturation, cx)
    }

    pub fn min(mut self, min: f32) -> Self {
        self.range.start = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.range.end = max;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step.abs());
        self
    }

    pub fn reversed(mut self, reversed: bool) -> Self {
        self.reversed = reversed;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn arc_thickness(mut self, thickness: f32) -> Self {
        self.arc_thickness = Some(thickness.max(1.0));
        self
    }

    pub fn arc_thickness_size(mut self, size: impl Into<Size>) -> Self {
        self.arc_thickness_size = Some(size.into());
        self
    }

    pub fn thumb_size(mut self, size: f32) -> Self {
        self.thumb_size = Some(size.max(4.0));
        self
    }

    pub fn start_degrees(mut self, degrees: f32) -> Self {
        self.start_degrees = degrees;
        self
    }

    pub fn sweep_degrees(mut self, degrees: f32) -> Self {
        self.sweep_degrees = degrees.max(0.0);
        self
    }

    pub fn set_value(&mut self, value: f32, cx: &mut Context<Self>) {
        let clamped_value = self.clamp_to_range(value);
        if self.value != clamped_value {
            self.value = clamped_value;
            self.interaction_position = None;
            cx.notify();
        }
    }

    pub fn set_delegate(&mut self, delegate: Box<dyn ColorArcDelegate>, cx: &mut Context<Self>) {
        self.delegate = delegate;
        cx.notify();
    }

    #[allow(dead_code)] // Optional prewarm hook to avoid first-interaction raster hitching.
    pub fn prewarm_raster_cache_square_in_place(&mut self, size_px: f32, cx: &mut Context<Self>) {
        let side = px(size_px.max(1.0));
        let image_size = size(side, side);
        self.delegate
            .prewarm_raster_cache(self, image_size, cx.theme().border);
    }

    pub fn set_start_degrees(&mut self, degrees: f32, cx: &mut Context<Self>) {
        if (self.start_degrees - degrees).abs() > f32::EPSILON {
            self.start_degrees = degrees;
            cx.notify();
        }
    }

    pub fn set_sweep_degrees(&mut self, degrees: f32, cx: &mut Context<Self>) {
        let degrees = degrees.max(0.0);
        if (self.sweep_degrees - degrees).abs() > f32::EPSILON {
            self.sweep_degrees = degrees;
            cx.notify();
        }
    }

    pub fn start_turns(&self) -> f32 {
        start_turns(self.start_degrees)
    }

    pub fn sweep_turns(&self) -> f32 {
        sweep_turns(self.sweep_degrees)
    }

    pub fn arc_thickness_px(&self) -> f32 {
        if let Some(thickness) = self.arc_thickness {
            return thickness;
        }

        let thickness_size = self.arc_thickness_size.unwrap_or(self.size);
        match thickness_size {
            Size::XSmall => sizing::ARC_THICKNESS_XSMALL,
            Size::Small => sizing::ARC_THICKNESS_SMALL,
            Size::Medium => sizing::ARC_THICKNESS_MEDIUM,
            Size::Large => sizing::ARC_THICKNESS_LARGE,
            Size::Size(px) => (px.as_f32() * 0.1).max(8.0),
        }
    }

    pub fn thumb_size_px(&self) -> f32 {
        self.thumb_size.unwrap_or(match self.size {
            Size::XSmall | Size::Small => sizing::THUMB_SIZE_SMALL,
            Size::Medium => sizing::THUMB_SIZE_MEDIUM,
            Size::Large => sizing::THUMB_SIZE_LARGE,
            Size::Size(px) => (px.as_f32() * 0.08).max(10.0),
        })
    }
}

impl ColorArcState {
    fn value_to_percent(&self) -> f32 {
        value_percent(self.value, self.range.clone())
    }

    fn update_from_mouse(
        &mut self,
        pointer: Point<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(position) = pointer_to_arc_position(
            self.bounds,
            pointer,
            self.start_turns(),
            self.sweep_turns(),
            self.reversed,
        ) else {
            return;
        };

        self.value = value_from_position(position, self.range.clone(), self.step, |position| {
            self.delegate.position_to_value(self, position)
        });
        self.interaction_position = Some(position);

        cx.emit(ColorArcEvent::Change(self.value));
        cx.notify();
    }

    fn emit_release(&self, cx: &mut Context<Self>) {
        cx.emit(ColorArcEvent::Release(self.value));
    }

    fn effective_position(&self) -> f32 {
        let value_percent = self.value_to_percent();
        resolve_effective_position(
            value_percent,
            self.interaction_position,
            |value_percent| {
                let mut position = self.delegate.value_to_position(self, value_percent);
                if self.reversed {
                    position = 1.0 - position;
                }
                position.clamp(0.0, 1.0)
            },
            |position| self.delegate.position_to_value(self, position),
        )
    }

    fn position_to_turn(&self, position: f32) -> f32 {
        let logical_position = if self.reversed {
            1.0 - position
        } else {
            position
        };
        position_to_turn(logical_position, self.start_turns(), self.sweep_turns())
    }

    fn accepts_pointer_at(&self, pointer: Point<Pixels>) -> bool {
        let Some(geometry) = arc_geometry(self.bounds, self.arc_thickness_px()) else {
            return false;
        };

        let thumb_turn = self.position_to_turn(self.effective_position());
        pointer_hits_arc_target(
            pointer,
            geometry,
            self.start_turns(),
            self.sweep_turns(),
            thumb_turn,
            self.thumb_size_px(),
        )
    }

    fn keyboard_step(&self, key: &str, modifiers: Modifiers) -> Option<f32> {
        let base_step = self
            .step
            .unwrap_or((self.range.end - self.range.start).abs() / 100.0);
        let multiplier = if modifiers.shift {
            10.0
        } else if modifiers.alt {
            0.1
        } else {
            1.0
        };
        let step = base_step * multiplier;

        let mut delta_sign = match key {
            "left" | "up" => -1.0,
            "right" | "down" => 1.0,
            _ => return None,
        };

        if self.reversed {
            delta_sign = -delta_sign;
        }

        Some(self.value + delta_sign * step)
    }
}

impl Styled for ColorArcState {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for ColorArcState {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl EventEmitter<ColorArcEvent> for ColorArcState {}

impl Render for ColorArcState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ColorArc::new(&cx.entity())
    }
}

#[derive(IntoElement)]
pub struct ColorArc {
    state: Entity<ColorArcState>,
}

struct ArcRenderLayout {
    size_px: f32,
    thumb_size: f32,
    value_position: f32,
    thumb_left: f32,
    thumb_top: f32,
}

impl ColorArc {
    pub fn new(state: &Entity<ColorArcState>) -> Self {
        Self {
            state: state.clone(),
        }
    }

    fn render_layout(state: &ColorArcState) -> ArcRenderLayout {
        let size_px = component_size_px(state.size);
        let thumb_size = state.thumb_size_px();
        let value_position = state.effective_position();
        let thumb_turn = state.position_to_turn(value_position);

        let current_size = if state.bounds.size.width > px(0.0) {
            state.bounds.size
        } else {
            size(px(size_px), px(size_px))
        };

        let local_bounds = Bounds {
            origin: point(px(0.0), px(0.0)),
            size: current_size,
        };
        let (thumb_left, thumb_top) = arc_geometry(local_bounds, state.arc_thickness_px())
            .map(|geometry| thumb_top_left(geometry, thumb_turn, thumb_size))
            .unwrap_or((0.0, 0.0));

        ArcRenderLayout {
            size_px,
            thumb_size,
            value_position,
            thumb_left,
            thumb_top,
        }
    }

    fn render_background(state: &ColorArcState, window: &mut Window, cx: &App) -> Div {
        state.delegate.style_background(
            state,
            div().absolute().inset_0().overflow_hidden(),
            window,
            cx,
        )
    }

    fn render_thumb(layout: &ArcRenderLayout, color: Hsla) -> Div {
        div()
            .absolute()
            .left(px(layout.thumb_left))
            .top(px(layout.thumb_top))
            .child(
                ColorThumb::new(px(layout.thumb_size))
                    .shape(ThumbShape::Circle)
                    .active(false)
                    .color(color),
            )
    }

    fn render_disabled_overlay(cx: &App) -> Div {
        div()
            .absolute()
            .inset_0()
            .bg(cx.theme().background.opacity(0.45))
    }

    fn handle_keyboard(
        state: &mut ColorArcState,
        event: &KeyDownEvent,
        cx: &mut Context<ColorArcState>,
    ) {
        match event.keystroke.key.as_str() {
            "home" => {
                state.set_value(state.range.start, cx);
                cx.emit(ColorArcEvent::Change(state.range.start));
                cx.emit(ColorArcEvent::Release(state.range.start));
                cx.stop_propagation();
            }
            "end" => {
                state.set_value(state.range.end, cx);
                cx.emit(ColorArcEvent::Change(state.range.end));
                cx.emit(ColorArcEvent::Release(state.range.end));
                cx.stop_propagation();
            }
            "left" | "up" | "right" | "down" => {
                let Some(next_value) =
                    state.keyboard_step(event.keystroke.key.as_str(), event.keystroke.modifiers)
                else {
                    return;
                };

                let clamped_value = next_value.clamp(
                    state.range.start.min(state.range.end),
                    state.range.end.max(state.range.start),
                );
                state.value = clamped_value;

                let value_pct = value_percent(clamped_value, state.range.clone());
                let mut pos = state.delegate.value_to_position(state, value_pct);
                if state.reversed {
                    pos = 1.0 - pos;
                }
                state.interaction_position = Some(pos.clamp(0.0, 1.0));

                cx.emit(ColorArcEvent::Change(clamped_value));
                cx.emit(ColorArcEvent::Release(clamped_value));
                cx.notify();
                cx.stop_propagation();
            }
            _ => {}
        }
    }

    fn with_keyboard_handlers(
        root: Stateful<Div>,
        state_entity: Entity<ColorArcState>,
    ) -> Stateful<Div> {
        root.on_key_down(move |event: &KeyDownEvent, _window: &mut Window, cx: &mut App| {
            state_entity.update(cx, |state, cx| {
                Self::handle_keyboard(state, event, cx);
            });
        })
    }

    fn with_prepaint_handler(
        root: Stateful<Div>,
        state_entity: Entity<ColorArcState>,
    ) -> Stateful<Div> {
        root.on_prepaint(move |bounds: Bounds<Pixels>, _: &mut Window, cx: &mut App| {
            state_entity.update(cx, |state, _| {
                state.bounds = bounds;
            })
        })
    }

    fn interaction_canvas(state_entity: Entity<ColorArcState>) -> impl IntoElement {
        canvas(
            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |_, _, window, _| {
                window.on_mouse_event({
                    let state_entity = state_entity.clone();
                    move |ev: &MouseMoveEvent, phase, window, cx| {
                        if !phase.bubble() {
                            return;
                        }
                        let _ = state_entity.update(cx, |state, cx| {
                            if !state.drag_interaction_enabled {
                                return;
                            }
                            state.update_from_mouse(ev.position, window, cx);
                        });
                    }
                });

                window.on_mouse_event({
                    let state_entity = state_entity.clone();
                    move |_: &MouseUpEvent, phase, _, cx| {
                        if !phase.bubble() {
                            return;
                        }
                        let _ = state_entity.update(cx, |state, cx| {
                            if state.drag_interaction_enabled {
                                state.drag_interaction_enabled = false;
                                state.emit_release(cx);
                                cx.notify();
                            }
                        });
                    }
                });
            },
        )
        .absolute()
        .inset_0()
    }

    fn with_pointer_handlers(
        root: Stateful<Div>,
        state_entity: Entity<ColorArcState>,
        window: &mut Window,
    ) -> Stateful<Div> {
        root.child(Self::interaction_canvas(state_entity.clone()))
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(
                    &state_entity,
                    |state: &mut ColorArcState,
                     ev: &MouseDownEvent,
                     window: &mut Window,
                     cx: &mut Context<ColorArcState>| {
                        if !state.accepts_pointer_at(ev.position) {
                            return;
                        }
                        state.drag_interaction_enabled = true;
                        state.focus_handle.focus(window, cx);
                        cx.stop_propagation();
                        state.update_from_mouse(ev.position, window, cx);
                    },
                ),
            )
            .on_mouse_up(
                MouseButton::Left,
                window.listener_for(
                    &state_entity,
                    |state: &mut ColorArcState,
                     _: &MouseUpEvent,
                     _: &mut Window,
                     cx: &mut Context<ColorArcState>| {
                        if state.drag_interaction_enabled {
                            state.drag_interaction_enabled = false;
                            state.emit_release(cx);
                            cx.notify();
                        }
                    },
                ),
            )
    }
}

impl RenderOnce for ColorArc {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_entity = self.state.clone();
        let state = state_entity.read(cx);
        let layout = Self::render_layout(&state);
        let thumb_color = state
            .delegate
            .get_color_at_position(&state, layout.value_position);
        let background = Self::render_background(&state, window, cx);

        let root = div()
            .id(state.id.clone())
            .size(px(layout.size_px))
            .relative()
            .refine_style(&state.style)
            .child(background)
            .when(!state.disabled, |this| this.child(Self::render_thumb(&layout, thumb_color)))
            .when(state.disabled, |this| this.child(Self::render_disabled_overlay(cx)))
            .track_focus(&state.focus_handle)
            ;
        let root = Self::with_keyboard_handlers(root, state_entity.clone());
        let root = Self::with_prepaint_handler(root, state_entity.clone());

        if state.disabled {
            root
        } else {
            Self::with_pointer_handlers(root, state_entity, window)
        }
    }
}
