use super::paint_raster::{domain_outline_hash, model_kind_code, quantized_background_hsv, rasterize_domain_image};
use super::super::domain::{CircleDomain, FieldDomain2D, RectDomain, TriangleDomain};
use super::super::model::{
    ColorFieldModel2D, HsAtValueModel, HueSaturationLightnessModel, HueSaturationWheelModel,
    HvAtSaturationModel, SvAtHueModel,
};
use crate::stories::color_primitives_story::color_spec::Hsv;
use crate::stories::color_primitives_story::mouse_behavior::{
    apply_hover_cursor, apply_window_cursor, reset_window_cursor_if_claimed,
    resolve_shared_mouse_preset, MouseCursorDecision, SharedMousePreset, SharedMousePresetContext,
};
use gpui::{prelude::*, *};
use std::sync::Arc;

const EDGE_TO_EDGE_THUMB_LIMIT_INSET_PX: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[allow(dead_code)]
pub enum FieldThumbPosition {
    #[default]
    InsideField,
    EdgeToEdgeClipped,
    EdgeToEdge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum ColorFieldRenderer {
    Vector,
    #[default]
    RasterImage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum ColorFieldMousePreset {
    #[default]
    Default,
    Crosshair,
    Passthrough,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct ColorFieldMouseContext {
    pub pointer: Point<Pixels>,
    pub hovered: bool,
    pub contains_pointer: bool,
    pub dragging: bool,
    pub external_drag_active: bool,
}

#[derive(Clone)]
pub enum ColorFieldMouseBehavior {
    Preset(ColorFieldMousePreset),
    Custom(Arc<dyn Fn(ColorFieldMouseContext) -> MouseCursorDecision + Send + Sync>),
}

impl Default for ColorFieldMouseBehavior {
    fn default() -> Self {
        Self::Preset(ColorFieldMousePreset::Default)
    }
}

impl ColorFieldMousePreset {
    fn resolve(self, ctx: ColorFieldMouseContext) -> MouseCursorDecision {
        let (shared_preset, active_cursor_style) = match self {
            ColorFieldMousePreset::Default => (SharedMousePreset::Default, CursorStyle::PointingHand),
            ColorFieldMousePreset::Crosshair => {
                (SharedMousePreset::Crosshair, CursorStyle::Crosshair)
            }
            ColorFieldMousePreset::Passthrough => {
                (SharedMousePreset::Passthrough, CursorStyle::Arrow)
            }
        };

        resolve_shared_mouse_preset(
            shared_preset,
            SharedMousePresetContext {
                hovered: ctx.hovered,
                contains_pointer: ctx.contains_pointer,
                dragging: ctx.dragging,
                external_drag_active: ctx.external_drag_active,
            },
            active_cursor_style,
        )
    }
}

impl ColorFieldMouseBehavior {
    fn resolve(&self, ctx: ColorFieldMouseContext) -> MouseCursorDecision {
        match self {
            ColorFieldMouseBehavior::Preset(preset) => preset.resolve(ctx),
            ColorFieldMouseBehavior::Custom(resolver) => resolver(ctx),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FieldImageCacheKey {
    size: Size<Pixels>,
    domain_hash: u64,
    model_kind: u16,
    model_cache_key: u64,
    hue: u16,
    saturation: u16,
    value: u16,
    alpha: u16,
    samples: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColorFieldEvent {
    Change(Hsv),
    Release(Hsv),
}

pub struct ColorFieldState {
    pub id: SharedString,
    pub hsv: Hsv,
    pub thumb_size: f32,
    pub bounds: Bounds<Pixels>,
    pub domain: Arc<dyn FieldDomain2D>,
    pub model: Arc<dyn ColorFieldModel2D>,
    pub style: StyleRefinement,
    pub show_border: bool,
    pub thumb_position: FieldThumbPosition,
    pub renderer: ColorFieldRenderer,
    pub samples_per_axis: usize,
    pub mouse_behavior: ColorFieldMouseBehavior,
    interaction_active: bool,
    window_cursor_claimed: bool,
    hover_inside_domain: bool,
    image_cache: Option<(FieldImageCacheKey, Arc<Image>)>,
}

impl ColorFieldState {
    fn circle_inside_max_radius_uv(&self, width_px: f32, height_px: f32) -> f32 {
        let half_thumb_uv = if width_px > 0.0 && height_px > 0.0 {
            (self.thumb_size / width_px.min(height_px)).clamp(0.0, 1.0) * 0.5
        } else {
            0.0
        };
        (0.5 - half_thumb_uv).max(0.0)
    }

    pub fn new(
        id: impl Into<SharedString>,
        hsv: Hsv,
        domain: Arc<dyn FieldDomain2D>,
        model: Arc<dyn ColorFieldModel2D>,
    ) -> Self {
        Self {
            id: id.into(),
            hsv: Hsv {
                h: hsv.h.clamp(0.0, 360.0),
                s: hsv.s.clamp(0.0, 1.0),
                v: hsv.v.clamp(0.0, 1.0),
                a: hsv.a.clamp(0.0, 1.0),
            },
            thumb_size: 16.0,
            bounds: Bounds::default(),
            domain,
            model,
            style: StyleRefinement::default(),
            show_border: true,
            thumb_position: FieldThumbPosition::EdgeToEdge,
            renderer: ColorFieldRenderer::RasterImage,
            samples_per_axis: 180,
            mouse_behavior: ColorFieldMouseBehavior::default(),
            interaction_active: false,
            window_cursor_claimed: false,
            hover_inside_domain: false,
            image_cache: None,
        }
    }

    pub fn saturation_value_rect(id: impl Into<SharedString>, hsv: Hsv, thumb_size: f32) -> Self {
        Self::new(id, hsv, Arc::new(RectDomain), Arc::new(SvAtHueModel)).thumb_size(thumb_size)
    }

    pub fn saturation_value_triangle(
        id: impl Into<SharedString>,
        hsv: Hsv,
        thumb_size: f32,
    ) -> Self {
        Self::new(
            id,
            hsv,
            Arc::new(TriangleDomain::up()),
            Arc::new(SvAtHueModel),
        )
        .thumb_size(thumb_size)
    }

    pub fn hue_saturation_wheel(id: impl Into<SharedString>, hsv: Hsv, thumb_size: f32) -> Self {
        Self::new(
            id,
            hsv,
            Arc::new(CircleDomain),
            Arc::new(HueSaturationWheelModel),
        )
        .thumb_size(thumb_size)
    }

    pub fn saturation_value(id: impl Into<SharedString>, hsv: Hsv, thumb_size: f32) -> Self {
        Self::new(id, hsv, Arc::new(RectDomain), Arc::new(SvAtHueModel)).thumb_size(thumb_size)
    }

    pub fn hue_saturation(id: impl Into<SharedString>, hsv: Hsv, thumb_size: f32) -> Self {
        Self::new(id, hsv, Arc::new(RectDomain), Arc::new(HsAtValueModel)).thumb_size(thumb_size)
    }

    pub fn hue_value(id: impl Into<SharedString>, hsv: Hsv, thumb_size: f32) -> Self {
        Self::new(id, hsv, Arc::new(RectDomain), Arc::new(HvAtSaturationModel))
            .thumb_size(thumb_size)
    }

    #[allow(dead_code)]
    pub fn hue_saturation_lightness(
        id: impl Into<SharedString>,
        hsv: Hsv,
        thumb_size: f32,
    ) -> Self {
        Self::new(
            id,
            hsv,
            Arc::new(RectDomain),
            Arc::new(HueSaturationLightnessModel),
        )
        .thumb_size(thumb_size)
    }

    // Alias used by composition stories that present this plane as HSV(H,S at fixed V).
    #[allow(dead_code)]
    pub fn hue_saturation_value(
        id: impl Into<SharedString>,
        hsv: Hsv,
        thumb_size: f32,
    ) -> Self {
        Self::hue_saturation_lightness(id, hsv, thumb_size)
    }

    pub fn thumb_size(mut self, thumb_size: f32) -> Self {
        self.thumb_size = thumb_size.max(8.0);
        self
    }

    #[allow(dead_code)]
    pub fn no_border(mut self) -> Self {
        self.show_border = false;
        self
    }

    #[allow(dead_code)]
    pub fn edge_to_edge(mut self) -> Self {
        self.thumb_position = FieldThumbPosition::EdgeToEdge;
        self
    }

    #[allow(dead_code)]
    pub fn edge_to_edge_clipped(mut self) -> Self {
        self.thumb_position = FieldThumbPosition::EdgeToEdgeClipped;
        self
    }

    #[allow(dead_code)]
    pub fn inside_field(mut self) -> Self {
        self.thumb_position = FieldThumbPosition::InsideField;
        self
    }

    #[allow(dead_code)]
    pub fn vector(mut self) -> Self {
        self.renderer = ColorFieldRenderer::Vector;
        self
    }

    #[allow(dead_code)]
    pub fn raster_image(mut self) -> Self {
        self.renderer = ColorFieldRenderer::RasterImage;
        self
    }

    #[allow(dead_code)]
    pub fn raster_image_prewarmed_square(self, size_px: f32) -> Self {
        self.raster_image_prewarmed(size_px, size_px)
    }

    #[allow(dead_code)]
    pub fn raster_image_prewarmed(mut self, width_px: f32, height_px: f32) -> Self {
        self.renderer = ColorFieldRenderer::RasterImage;
        self.prewarm_raster_cache_size_in_place(width_px, height_px);
        self
    }

    #[allow(dead_code)]
    pub fn with_domain(mut self, domain: Arc<dyn FieldDomain2D>) -> Self {
        self.domain = domain;
        self
    }

    #[allow(dead_code)]
    pub fn with_model(mut self, model: Arc<dyn ColorFieldModel2D>) -> Self {
        self.model = model;
        self
    }

    #[allow(dead_code)]
    pub fn samples_per_axis(mut self, samples: usize) -> Self {
        self.samples_per_axis = samples.max(16);
        self
    }

    #[allow(dead_code)]
    pub fn mouse_preset(mut self, preset: ColorFieldMousePreset) -> Self {
        self.mouse_behavior = ColorFieldMouseBehavior::Preset(preset);
        self
    }

    #[allow(dead_code)]
    pub fn mouse_behavior(mut self, behavior: ColorFieldMouseBehavior) -> Self {
        self.mouse_behavior = behavior;
        self
    }

    #[allow(dead_code)]
    pub fn mouse_behavior_custom<F>(mut self, resolver: F) -> Self
    where
        F: Fn(ColorFieldMouseContext) -> MouseCursorDecision + Send + Sync + 'static,
    {
        self.mouse_behavior = ColorFieldMouseBehavior::Custom(Arc::new(resolver));
        self
    }

    pub fn set_hsv(&mut self, hsv: Hsv, cx: &mut Context<Self>) {
        let clamped = Hsv {
            h: hsv.h.clamp(0.0, 360.0),
            s: hsv.s.clamp(0.0, 1.0),
            v: hsv.v.clamp(0.0, 1.0),
            a: hsv.a.clamp(0.0, 1.0),
        };
        if self.hsv != clamped {
            self.hsv = clamped;
            // Programmatic updates (slider clicks, reset) should repaint immediately
            // without waiting for a follow-up input event to refresh raster cache.
            if self.renderer == ColorFieldRenderer::RasterImage
                && self.bounds.size.width > px(0.0)
                && self.bounds.size.height > px(0.0)
            {
                let _ = self.ensure_raster_image_cache(self.bounds.size);
            }
            cx.notify();
        }
    }

    pub fn set_hsv_components(
        &mut self,
        hue: f32,
        saturation: f32,
        value: f32,
        cx: &mut Context<Self>,
    ) {
        self.set_hsv(
            Hsv {
                h: hue,
                s: saturation,
                v: value,
                a: self.hsv.a,
            },
            cx,
        );
    }

    #[allow(dead_code)]
    pub fn set_domain(&mut self, domain: Arc<dyn FieldDomain2D>, cx: &mut Context<Self>) {
        self.domain = domain;
        self.image_cache = None;
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn set_model(&mut self, model: Arc<dyn ColorFieldModel2D>, cx: &mut Context<Self>) {
        self.model = model;
        self.image_cache = None;
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn set_renderer(&mut self, renderer: ColorFieldRenderer, cx: &mut Context<Self>) {
        if self.renderer != renderer {
            self.renderer = renderer;
            self.image_cache = None;
            cx.notify();
        }
    }

    #[allow(dead_code)]
    pub fn prewarm_raster_cache_square_in_place(&mut self, size_px: f32) {
        self.prewarm_raster_cache_size_in_place(size_px, size_px);
    }

    #[allow(dead_code)]
    pub fn prewarm_raster_cache_size_in_place(&mut self, width_px: f32, height_px: f32) {
        if self.renderer != ColorFieldRenderer::RasterImage {
            return;
        }
        let width = px(width_px.max(1.0));
        let height = px(height_px.max(1.0));
        let _ = self.ensure_raster_image_cache(size(width, height));
    }

    #[allow(dead_code)]
    pub fn set_corner_radius(&mut self, radius: AbsoluteLength, cx: &mut Context<Self>) {
        self.style.corner_radii.top_left = Some(radius);
        self.style.corner_radii.top_right = Some(radius);
        self.style.corner_radii.bottom_left = Some(radius);
        self.style.corner_radii.bottom_right = Some(radius);
        cx.notify();
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn set_show_border(&mut self, show_border: bool, cx: &mut Context<Self>) {
        if self.show_border != show_border {
            self.show_border = show_border;
            cx.notify();
        }
    }

    #[allow(dead_code)]
    pub fn set_thumb_position(
        &mut self,
        thumb_position: FieldThumbPosition,
        cx: &mut Context<Self>,
    ) {
        if self.thumb_position != thumb_position {
            self.thumb_position = thumb_position;
            cx.notify();
        }
    }

    #[allow(dead_code)]
    pub fn set_mouse_behavior(
        &mut self,
        behavior: ColorFieldMouseBehavior,
        cx: &mut Context<Self>,
    ) {
        self.mouse_behavior = behavior;
        cx.notify();
    }

    pub(super) fn resolved_corner_radii(&self, window: &Window) -> Corners<Pixels> {
        let rem_size = window.rem_size();
        let corner_radii = self.style.corner_radii.clone();
        let default_radius = px(4.0);

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

    fn point_to_unit_uv(&self, position: Point<Pixels>) -> Option<(f32, f32)> {
        if self.bounds.size.width <= px(0.0) || self.bounds.size.height <= px(0.0) {
            return None;
        }

        let local_x = (position.x - self.bounds.origin.x).as_f32();
        let local_y = (position.y - self.bounds.origin.y).as_f32();
        let width = self.bounds.size.width.as_f32().max(1.0);
        let height = self.bounds.size.height.as_f32().max(1.0);

        Some((local_x / width, local_y / height))
    }

    pub(super) fn update_from_mouse(
        &mut self,
        position: Point<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(unit_uv) = self.point_to_unit_uv(position) else {
            return;
        };

        let width = self.bounds.size.width.as_f32().max(1.0);
        let height = self.bounds.size.height.as_f32().max(1.0);

        let mut uv = (unit_uv.0.clamp(0.0, 1.0), unit_uv.1.clamp(0.0, 1.0));
        if self.domain.is_circle() && self.thumb_position == FieldThumbPosition::InsideField {
            let max_radius_uv = self.circle_inside_max_radius_uv(width, height);
            if max_radius_uv <= f32::EPSILON {
                uv = (0.5, 0.5);
            } else {
                let dx = uv.0 - 0.5;
                let dy = uv.1 - 0.5;
                let radius = (dx * dx + dy * dy).sqrt();
                let clamped = if radius > max_radius_uv {
                    let scale = max_radius_uv / radius;
                    (0.5 + dx * scale, 0.5 + dy * scale)
                } else {
                    uv
                };
                let expand = 0.5 / max_radius_uv;
                uv = (
                    (0.5 + (clamped.0 - 0.5) * expand).clamp(0.0, 1.0),
                    (0.5 + (clamped.1 - 0.5) * expand).clamp(0.0, 1.0),
                );
            }
        }
        let uv = self.domain.clamp_uv(uv);

        let mut next = self.hsv;
        self.model.apply_uv(&mut next, uv);
        next.h = next.h.clamp(0.0, 360.0);
        next.s = next.s.clamp(0.0, 1.0);
        next.v = next.v.clamp(0.0, 1.0);

        if self.hsv != next {
            self.hsv = next;
            cx.emit(ColorFieldEvent::Change(self.hsv));
            cx.notify();
        }
    }

    fn accepts_pointer_at(&self, position: Point<Pixels>) -> bool {
        let Some(uv) = self.point_to_unit_uv(position) else {
            return false;
        };

        if !(0.0..=1.0).contains(&uv.0) || !(0.0..=1.0).contains(&uv.1) {
            return false;
        }

        self.domain.contains_uv(uv)
    }

    #[allow(dead_code)]
    pub fn contains_pointer(&self, position: Point<Pixels>) -> bool {
        self.accepts_pointer_at(position)
    }

    fn resolve_mouse_cursor(
        &self,
        pointer: Point<Pixels>,
        hovered: bool,
        external_drag_active: bool,
    ) -> MouseCursorDecision {
        let contains_pointer = self.accepts_pointer_at(pointer);
        self.mouse_behavior.resolve(ColorFieldMouseContext {
            pointer,
            hovered,
            contains_pointer,
            dragging: self.interaction_active,
            external_drag_active,
        })
    }

    fn apply_idle_cursor_handoff(
        &mut self,
        window: &mut Window,
        hitbox: &Hitbox,
        decision: MouseCursorDecision,
        hovered: bool,
    ) {
        if hovered {
            // Avoid a one-frame Arrow flash on release:
            // if we still hover the field, drop the window-cursor
            // claim and let hitbox hover cursor take over directly.
            self.window_cursor_claimed = false;
            apply_hover_cursor(window, hitbox, decision);
        } else {
            reset_window_cursor_if_claimed(window, &mut self.window_cursor_claimed);
        }
    }

    pub(super) fn update_cursor_state(
        &mut self,
        pointer: Point<Pixels>,
        hovered: bool,
        external_drag_active: bool,
        window: &mut Window,
        hitbox: &Hitbox,
    ) {
        let decision = self.resolve_mouse_cursor(pointer, hovered, external_drag_active);
        if self.interaction_active {
            apply_window_cursor(window, decision, &mut self.window_cursor_claimed);
        } else {
            self.apply_idle_cursor_handoff(window, hitbox, decision, hovered);
        }
    }

    pub(super) fn handle_active_move(
        &mut self,
        pointer: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.interaction_active {
            return;
        }

        self.update_from_mouse(pointer, window, cx);
    }

    pub(super) fn handle_pointer_release(
        &mut self,
        pointer: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.interaction_active {
            self.update_from_mouse(pointer, window, cx);
        }
        let was_active = self.interaction_active;
        self.end_interaction(cx);
        if was_active {
            cx.notify();
            window.refresh();
        }
    }

    pub(super) fn begin_interaction(&mut self) {
        self.interaction_active = true;
    }

    fn end_interaction(&mut self, cx: &mut Context<Self>) {
        if !self.interaction_active {
            return;
        }
        self.interaction_active = false;
        cx.emit(ColorFieldEvent::Release(self.hsv));
    }

    pub(super) fn is_interaction_active(&self) -> bool {
        self.interaction_active
    }

    pub(super) fn update_hover_inside_domain(
        &mut self,
        pointer: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let inside = self.accepts_pointer_at(pointer);
        if inside != self.hover_inside_domain {
            self.hover_inside_domain = inside;
            cx.notify();
        }
    }

    pub(super) fn refresh_bounds_and_render_cache(
        &mut self,
        bounds: Bounds<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let bounds_changed = self.bounds != bounds;
        if self.bounds != bounds {
            self.bounds = bounds;
        }

        if bounds_changed && self.renderer == ColorFieldRenderer::Vector {
            cx.notify();
        }

        if self.renderer == ColorFieldRenderer::RasterImage {
            let had_image = self.image_cache.is_some();
            let refreshed = self.ensure_raster_image_cache(bounds.size);
            if refreshed || !had_image {
                cx.notify();
            }
        }
    }

    pub(super) fn thumb_uv(
        &self,
        domain: &dyn FieldDomain2D,
        model: &dyn ColorFieldModel2D,
    ) -> (f32, f32) {
        let is_circle = domain.is_circle();
        let raw_thumb_uv = domain.clamp_uv(model.uv_from_hsv(&self.hsv));

        match self.thumb_position {
            FieldThumbPosition::EdgeToEdge | FieldThumbPosition::EdgeToEdgeClipped => {
                if is_circle {
                    raw_thumb_uv
                } else if self.bounds.size.width > px(0.0) && self.bounds.size.height > px(0.0) {
                    let inset_x =
                        (EDGE_TO_EDGE_THUMB_LIMIT_INSET_PX / self.bounds.size.width.as_f32())
                            .clamp(0.0, 0.5);
                    let inset_y =
                        (EDGE_TO_EDGE_THUMB_LIMIT_INSET_PX / self.bounds.size.height.as_f32())
                            .clamp(0.0, 0.5);
                    (
                        raw_thumb_uv.0.clamp(inset_x, 1.0 - inset_x),
                        raw_thumb_uv.1.clamp(inset_y, 1.0 - inset_y),
                    )
                } else {
                    (raw_thumb_uv.0.clamp(0.0, 1.0), raw_thumb_uv.1.clamp(0.0, 1.0))
                }
            }
            FieldThumbPosition::InsideField => {
                if self.bounds.size.width > px(0.0) && self.bounds.size.height > px(0.0) {
                    if is_circle {
                        let max_radius_uv = self.circle_inside_max_radius_uv(
                            self.bounds.size.width.as_f32(),
                            self.bounds.size.height.as_f32(),
                        );
                        let scale = if max_radius_uv <= f32::EPSILON {
                            0.0
                        } else {
                            (max_radius_uv / 0.5).clamp(0.0, 1.0)
                        };
                        (
                            (0.5 + (raw_thumb_uv.0 - 0.5) * scale).clamp(0.0, 1.0),
                            (0.5 + (raw_thumb_uv.1 - 0.5) * scale).clamp(0.0, 1.0),
                        )
                    } else {
                        let half_x =
                            (self.thumb_size / 2.0 / self.bounds.size.width.as_f32()).clamp(0.0, 0.5);
                        let half_y =
                            (self.thumb_size / 2.0 / self.bounds.size.height.as_f32()).clamp(0.0, 0.5);
                        (
                            (raw_thumb_uv.0 * (1.0 - 2.0 * half_x) + half_x).clamp(0.0, 1.0),
                            (raw_thumb_uv.1 * (1.0 - 2.0 * half_y) + half_y).clamp(0.0, 1.0),
                        )
                    }
                } else {
                    raw_thumb_uv
                }
            }
        }
    }

    pub(super) fn cached_image(&self) -> Option<Arc<Image>> {
        self.image_cache.as_ref().map(|(_, image)| image.clone())
    }

    fn cache_key_for_size(&self, size: Size<Pixels>) -> FieldImageCacheKey {
        let model_kind = model_kind_code(self.model.kind());
        let (hue, saturation, value, alpha) = quantized_background_hsv(model_kind, self.hsv);

        FieldImageCacheKey {
            size,
            domain_hash: domain_outline_hash(self.domain.as_ref()),
            model_kind,
            model_cache_key: self.model.cache_key_part(),
            hue,
            saturation,
            value,
            alpha,
            samples: self.samples_per_axis.clamp(16, u16::MAX as usize) as u16,
        }
    }

    fn ensure_raster_image_cache(&mut self, size: Size<Pixels>) -> bool {
        if self.renderer != ColorFieldRenderer::RasterImage {
            return false;
        }

        let key = self.cache_key_for_size(size);
        if let Some((cached_key, _)) = &self.image_cache {
            if *cached_key == key {
                return false;
            }
        }

        let Some(image) = rasterize_domain_image(
            size,
            self.domain.as_ref(),
            self.model.as_ref(),
            self.hsv,
            self.samples_per_axis,
        ) else {
            return false;
        };

        self.image_cache = Some((key, image));
        true
    }
}

impl Styled for ColorFieldState {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl EventEmitter<ColorFieldEvent> for ColorFieldState {}
