use super::arc::{ColorArcDelegate, ColorArcEvent, ColorArcState};
use super::common::{arc_contains_turn, size_px as component_size_px, turn_to_position};
use crate::stories::color_primitives_story::color_slider::color_spec::Hsv;
use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, PixelsExt as _, Size as ComponentSize};
use std::cell::RefCell;
use std::f32::consts::TAU;
use std::sync::Arc;
use tiny_skia::{Pixmap, PremultipliedColorU8};

pub type ColorArcRasterState = ColorArcState;
#[allow(dead_code)] // Kept as a compatibility alias for prior event naming.
pub type ColorArcRasterEvent = ColorArcEvent;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorArcRenderer {
    Vector,
    #[default]
    Raster,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)] // Raster hue mode is exposed for API symmetry with vector delegates.
pub enum ColorArcRasterMode {
    Hue,
    Saturation,
    Lightness,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ArcCacheKey {
    mode: u16,
    hue: u16,
    value: u16,
    saturation: u16,
    lightness: u16,
    arc_thickness: u16,
    start_turns: u16,
    sweep_turns: u16,
    reversed: u8,
    border_r: u8,
    border_g: u8,
    border_b: u8,
    border_a: u8,
}

pub struct RasterArcDelegate {
    mode: ColorArcRasterMode,
    hue: f32,
    hsv_value: f32,
    saturation: f32,
    lightness: f32,
    image_cache: RefCell<Option<(Size<Pixels>, ArcCacheKey, Arc<Image>)>>,
}

impl RasterArcDelegate {
    #[allow(dead_code)] // Public constructor kept for hue arc raster parity.
    pub fn hue(saturation: f32, lightness: f32) -> Self {
        Self {
            mode: ColorArcRasterMode::Hue,
            hue: 0.0,
            hsv_value: 1.0,
            saturation: saturation.clamp(0.0, 1.0),
            lightness: lightness.clamp(0.0, 1.0),
            image_cache: RefCell::new(None),
        }
    }

    pub fn saturation(hue: f32, hsv_value: f32) -> Self {
        Self {
            mode: ColorArcRasterMode::Saturation,
            hue: normalize_hue_degrees(hue),
            hsv_value: hsv_value.clamp(0.0, 1.0),
            saturation: 1.0,
            lightness: 0.5,
            image_cache: RefCell::new(None),
        }
    }

    pub fn lightness(hue: f32, saturation: f32) -> Self {
        Self {
            mode: ColorArcRasterMode::Lightness,
            hue: normalize_hue_degrees(hue),
            hsv_value: 1.0,
            saturation: saturation.clamp(0.0, 1.0),
            lightness: 1.0,
            image_cache: RefCell::new(None),
        }
    }

    fn cache_scale(size: ComponentSize) -> f32 {
        match size {
            ComponentSize::XSmall | ComponentSize::Small => 4.0,
            ComponentSize::Medium => 3.0,
            ComponentSize::Large => 2.0,
            ComponentSize::Size(px) => {
                let size = px.as_f32();
                if size <= 180.0 {
                    4.0
                } else if size <= 260.0 {
                    3.0
                } else {
                    2.0
                }
            }
        }
    }

    fn cache_key(&self, arc: &ColorArcState, border_color: Hsla) -> ArcCacheKey {
        let border_rgb = border_color.to_rgb();
        ArcCacheKey {
            mode: match self.mode {
                ColorArcRasterMode::Hue => 0,
                ColorArcRasterMode::Saturation => 1,
                ColorArcRasterMode::Lightness => 2,
            },
            hue: (self.hue.rem_euclid(360.0) * 10.0).round() as u16,
            value: (self.hsv_value.clamp(0.0, 1.0) * 1000.0).round() as u16,
            saturation: (self.saturation.clamp(0.0, 1.0) * 1000.0).round() as u16,
            lightness: (self.lightness.clamp(0.0, 1.0) * 1000.0).round() as u16,
            arc_thickness: (arc.arc_thickness_px().clamp(1.0, 200.0) * 10.0).round() as u16,
            start_turns: (arc.start_turns() * 3600.0).round() as u16,
            sweep_turns: (arc.sweep_turns().clamp(0.0, 1.0) * 3600.0).round() as u16,
            reversed: if arc.reversed { 1 } else { 0 },
            border_r: (border_rgb.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            border_g: (border_rgb.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            border_b: (border_rgb.b.clamp(0.0, 1.0) * 255.0).round() as u8,
            border_a: (border_color.a.clamp(0.0, 1.0) * 255.0).round() as u8,
        }
    }

    fn color_at_logical_position(&self, position: f32) -> Hsla {
        let logical = position.clamp(0.0, 1.0);
        match self.mode {
            ColorArcRasterMode::Hue => hsla(
                logical,
                self.saturation.clamp(0.0, 1.0),
                self.lightness.clamp(0.0, 1.0),
                1.0,
            ),
            ColorArcRasterMode::Saturation => Hsv {
                h: self.hue.rem_euclid(360.0),
                s: logical,
                v: self.hsv_value.clamp(0.0, 1.0),
                a: 1.0,
            }
            .to_hsla_ext(),
            ColorArcRasterMode::Lightness => hsla(
                self.hue.rem_euclid(360.0) / 360.0,
                self.saturation.clamp(0.0, 1.0),
                logical,
                1.0,
            ),
        }
    }

    fn current_size(&self, arc: &ColorArcState) -> Size<Pixels> {
        if arc.bounds.size.width > px(0.0) && arc.bounds.size.height > px(0.0) {
            arc.bounds.size
        } else {
            let side = component_size_px(arc.size);
            size(px(side), px(side))
        }
    }

    fn ensure_cache(
        &self,
        arc: &ColorArcState,
        image_size: Size<Pixels>,
        border_color: Hsla,
    ) -> Option<Arc<Image>> {
        let key = self.cache_key(arc, border_color);
        if let Some((cached_size, cached_key, image)) = self.image_cache.borrow().as_ref() {
            if *cached_size == image_size && *cached_key == key {
                return Some(image.clone());
            }
        }

        let sweep_turns = arc.sweep_turns().clamp(0.0, 1.0);
        if sweep_turns <= f32::EPSILON {
            return None;
        }

        let cache_scale = Self::cache_scale(arc.size);
        let width = (image_size.width.as_f32() * cache_scale).round() as u32;
        let height = (image_size.height.as_f32() * cache_scale).round() as u32;
        if width == 0 || height == 0 {
            return None;
        }

        let mut pixmap = Pixmap::new(width, height)?;
        let pixels = pixmap.pixels_mut();

        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let outer_radius = width.min(height) as f32 / 2.0;
        let arc_thickness = arc.arc_thickness_px() * cache_scale;
        if arc_thickness <= 0.0 || outer_radius <= 0.0 {
            return None;
        }

        let border_width = 1.6 * cache_scale;
        let fill_thickness = (arc_thickness - border_width * 2.0).max(0.0);

        let outer_border_radius = outer_radius;
        let inner_border_radius = (outer_border_radius - arc_thickness).max(0.0);

        let outer_fill_radius = (outer_radius - border_width).max(0.0);
        let inner_fill_radius = (outer_fill_radius - fill_thickness).max(0.0);

        let start_turn = arc.start_turns();
        let end_turn = (start_turn + sweep_turns).rem_euclid(1.0);
        let has_caps = sweep_turns < 0.9999;

        let border_cap_radius = arc_thickness * 0.5;
        let border_track_radius = (outer_border_radius - border_cap_radius).max(0.0);
        let border_start_center = cap_center(start_turn, border_track_radius, center_x, center_y);
        let border_end_center = cap_center(end_turn, border_track_radius, center_x, center_y);

        let fill_cap_radius = fill_thickness * 0.5;
        let fill_track_radius = (outer_fill_radius - fill_cap_radius).max(0.0);
        let fill_start_center = cap_center(start_turn, fill_track_radius, center_x, center_y);
        let fill_end_center = cap_center(end_turn, fill_track_radius, center_x, center_y);

        let border_rgb = border_color.to_rgb();
        let border_r = border_rgb.r.clamp(0.0, 1.0);
        let border_g = border_rgb.g.clamp(0.0, 1.0);
        let border_b = border_rgb.b.clamp(0.0, 1.0);

        for y in 0..height {
            for x in 0..width {
                let mut r_sum = 0.0_f32;
                let mut g_sum = 0.0_f32;
                let mut b_sum = 0.0_f32;
                let mut covered = 0_u8;

                for sy in [0.25_f32, 0.75_f32] {
                    for sx in [0.25_f32, 0.75_f32] {
                        let sample_x = x as f32 + sx;
                        let sample_y = y as f32 + sy;
                        let dx = sample_x - center_x;
                        let dy = sample_y - center_y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        let theta = dy.atan2(dx);
                        let turn = (0.25 + (theta / TAU)).rem_euclid(1.0);
                        let on_arc = arc_contains_turn(turn, start_turn, sweep_turns);

                        let mut sample_rgb: Option<(f32, f32, f32)> = None;

                        if fill_thickness > 0.0 {
                            let on_fill_band =
                                on_arc && dist <= outer_fill_radius && dist >= inner_fill_radius;

                            let start_fill_dist =
                                point_distance(sample_x, sample_y, fill_start_center);
                            let end_fill_dist = point_distance(sample_x, sample_y, fill_end_center);
                            let on_start_fill_cap = has_caps && start_fill_dist <= fill_cap_radius;
                            let on_end_fill_cap = has_caps && end_fill_dist <= fill_cap_radius;
                            let on_fill_cap = on_start_fill_cap || on_end_fill_cap;

                            if on_fill_band || on_fill_cap {
                                let logical_position = if on_fill_band {
                                    let position = turn_to_position(turn, start_turn, sweep_turns);
                                    logical_position(arc.reversed, position)
                                } else if on_start_fill_cap && !on_end_fill_cap {
                                    logical_position(arc.reversed, 0.0)
                                } else if on_end_fill_cap && !on_start_fill_cap {
                                    logical_position(arc.reversed, 1.0)
                                } else if start_fill_dist <= end_fill_dist {
                                    logical_position(arc.reversed, 0.0)
                                } else {
                                    logical_position(arc.reversed, 1.0)
                                };

                                let fill_rgb =
                                    self.color_at_logical_position(logical_position).to_rgb();
                                sample_rgb = Some((fill_rgb.r, fill_rgb.g, fill_rgb.b));
                            }
                        }

                        if sample_rgb.is_none() {
                            let on_border_band = on_arc
                                && dist <= outer_border_radius
                                && dist >= inner_border_radius;
                            let on_border_cap = if has_caps {
                                let start_border_dist =
                                    point_distance(sample_x, sample_y, border_start_center);
                                let end_border_dist =
                                    point_distance(sample_x, sample_y, border_end_center);
                                start_border_dist <= border_cap_radius
                                    || end_border_dist <= border_cap_radius
                            } else {
                                false
                            };

                            if on_border_band || on_border_cap {
                                sample_rgb = Some((border_r, border_g, border_b));
                            }
                        }

                        if let Some((r, g, b)) = sample_rgb {
                            r_sum += r;
                            g_sum += g;
                            b_sum += b;
                            covered += 1;
                        }
                    }
                }

                if covered == 0 {
                    continue;
                }

                let inv = 1.0 / covered as f32;
                let r_u8 = (r_sum * inv * 255.0).round() as u8;
                let g_u8 = (g_sum * inv * 255.0).round() as u8;
                let b_u8 = (b_sum * inv * 255.0).round() as u8;
                let a_u8 = ((covered as f32 / 4.0) * 255.0).round() as u8;

                if let Some(pixel) = PremultipliedColorU8::from_rgba(r_u8, g_u8, b_u8, a_u8) {
                    pixels[(y * width + x) as usize] = pixel;
                }
            }
        }

        let png_data = pixmap.encode_png().ok()?;
        let image = Arc::new(Image::from_bytes(ImageFormat::Png, png_data));
        *self.image_cache.borrow_mut() = Some((image_size, key, image.clone()));
        Some(image)
    }
}

impl ColorArcDelegate for RasterArcDelegate {
    fn style_background(
        &self,
        arc: &ColorArcState,
        container: Div,
        _window: &mut Window,
        cx: &App,
    ) -> Div {
        let image = self.ensure_cache(arc, self.current_size(arc), cx.theme().border);
        container.when_some(image, |this, image| {
            this.child(img(image).size_full().absolute().top_0().left_0())
        })
    }

    fn get_color_at_position(&self, arc: &ColorArcState, position: f32) -> Hsla {
        self.color_at_logical_position(logical_position(arc.reversed, position))
    }

    fn prewarm_raster_cache(
        &self,
        arc: &ColorArcState,
        image_size: Size<Pixels>,
        border_color: Hsla,
    ) {
        let _ = self.ensure_cache(arc, image_size, border_color);
    }

    fn renderer(&self) -> ColorArcRenderer {
        ColorArcRenderer::Raster
    }
}

fn logical_position(reversed: bool, position: f32) -> f32 {
    if reversed {
        1.0 - position.clamp(0.0, 1.0)
    } else {
        position.clamp(0.0, 1.0)
    }
}

fn cap_center(turn: f32, track_radius: f32, center_x: f32, center_y: f32) -> (f32, f32) {
    let theta = (turn.rem_euclid(1.0) - 0.25) * TAU;
    (
        center_x + track_radius * theta.cos(),
        center_y + track_radius * theta.sin(),
    )
}

fn point_distance(x: f32, y: f32, center: (f32, f32)) -> f32 {
    let dx = x - center.0;
    let dy = y - center.1;
    (dx * dx + dy * dy).sqrt()
}

fn normalize_hue_degrees(hue: f32) -> f32 {
    hue.rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stories::color_primitives_story::color_arc::delegates::{
        HueArcDelegate, LightnessArcDelegate, SaturationArcDelegate,
    };

    #[::core::prelude::v1::test]
    fn renderer_default_is_raster() {
        assert_eq!(ColorArcRenderer::default(), ColorArcRenderer::Raster);
    }

    #[::core::prelude::v1::test]
    fn raster_delegate_reports_raster_renderer() {
        let hue = RasterArcDelegate::hue(0.8, 0.4);
        let sat = RasterArcDelegate::saturation(180.0, 1.0);
        let light = RasterArcDelegate::lightness(180.0, 0.8);

        assert_eq!(hue.renderer(), ColorArcRenderer::Raster);
        assert_eq!(sat.renderer(), ColorArcRenderer::Raster);
        assert_eq!(light.renderer(), ColorArcRenderer::Raster);
    }

    #[::core::prelude::v1::test]
    fn vector_delegates_report_vector_renderer() {
        let hue = HueArcDelegate {
            saturation: 0.8,
            lightness: 0.4,
        };
        let sat = SaturationArcDelegate {
            hue: 180.0,
            hsv_value: 1.0,
        };
        let light = LightnessArcDelegate {
            hue: 180.0,
            saturation: 0.8,
        };

        assert_eq!(hue.renderer(), ColorArcRenderer::Vector);
        assert_eq!(sat.renderer(), ColorArcRenderer::Vector);
        assert_eq!(light.renderer(), ColorArcRenderer::Vector);
    }
}
