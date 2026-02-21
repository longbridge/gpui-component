use super::common::{
    mirrored_lightness, mirrored_saturation, position_from_mirrored_lightness,
    position_from_mirrored_saturation, size_px as component_size_px,
};
use super::ring::{ColorRingDelegate, ColorRingEvent, ColorRingState};
use crate::stories::color_primitives_story::color_slider::color_spec::Hsv;
use gpui::{prelude::*, *};
use gpui_component::{Size as ComponentSize};
use std::cell::RefCell;
use std::f32::consts::TAU;
use std::sync::Arc;
use tiny_skia::{Pixmap, PremultipliedColorU8};

pub type ColorRingRasterState = ColorRingState;
#[allow(dead_code)] // Kept as a compatibility alias for prior event naming.
pub type ColorRingRasterEvent = ColorRingEvent;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorRingRenderer {
    Vector,
    #[default]
    Raster,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorRingRasterMode {
    Hue,
    Saturation,
    Lightness,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RingCacheKey {
    mode: u16,
    hue: u16,
    value: u16,
    saturation: u16,
    lightness: u16,
    ring_thickness: u16,
    rotation: u16,
}

pub struct RasterRingDelegate {
    mode: ColorRingRasterMode,
    hue: f32,
    hsv_value: f32,
    saturation: f32,
    lightness: f32,
    image_cache: RefCell<Option<(Size<Pixels>, RingCacheKey, Arc<Image>)>>,
}

impl RasterRingDelegate {
    pub fn hue(saturation: f32, lightness: f32) -> Self {
        Self {
            mode: ColorRingRasterMode::Hue,
            hue: 0.0,
            hsv_value: 1.0,
            saturation: saturation.clamp(0.0, 1.0),
            lightness: lightness.clamp(0.0, 1.0),
            image_cache: RefCell::new(None),
        }
    }

    pub fn saturation(hue: f32, hsv_value: f32) -> Self {
        Self {
            mode: ColorRingRasterMode::Saturation,
            hue: normalize_hue_degrees(hue),
            hsv_value: hsv_value.clamp(0.0, 1.0),
            saturation: 1.0,
            lightness: 0.5,
            image_cache: RefCell::new(None),
        }
    }

    pub fn lightness(hue: f32, saturation: f32) -> Self {
        Self {
            mode: ColorRingRasterMode::Lightness,
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

    fn cache_key(&self, circle: &ColorRingState) -> RingCacheKey {
        RingCacheKey {
            mode: match self.mode {
                ColorRingRasterMode::Hue => 0,
                ColorRingRasterMode::Saturation => 1,
                ColorRingRasterMode::Lightness => 2,
            },
            hue: (self.hue.rem_euclid(360.0) * 10.0).round() as u16,
            value: (self.hsv_value.clamp(0.0, 1.0) * 1000.0).round() as u16,
            saturation: (self.saturation.clamp(0.0, 1.0) * 1000.0).round() as u16,
            lightness: (self.lightness.clamp(0.0, 1.0) * 1000.0).round() as u16,
            ring_thickness: (circle.ring_thickness_px().clamp(1.0, 200.0) * 10.0).round() as u16,
            rotation: (circle.rotation_turns() * 3600.0).round() as u16,
        }
    }

    fn position_to_value_percent(&self, position: f32) -> f32 {
        match self.mode {
            ColorRingRasterMode::Hue => position.rem_euclid(1.0),
            ColorRingRasterMode::Saturation => mirrored_saturation(position),
            ColorRingRasterMode::Lightness => mirrored_lightness(position),
        }
    }

    fn position_from_value_percent(&self, value: f32) -> f32 {
        let value = value.clamp(0.0, 1.0);
        match self.mode {
            ColorRingRasterMode::Hue => value,
            ColorRingRasterMode::Saturation => position_from_mirrored_saturation(value),
            ColorRingRasterMode::Lightness => position_from_mirrored_lightness(value),
        }
    }

    fn color_at_position(&self, position: f32) -> Hsla {
        match self.mode {
            ColorRingRasterMode::Hue => hsla(
                position.rem_euclid(1.0),
                self.saturation.clamp(0.0, 1.0),
                self.lightness.clamp(0.0, 1.0),
                1.0,
            ),
            ColorRingRasterMode::Saturation => Hsv {
                h: self.hue.rem_euclid(360.0),
                s: mirrored_saturation(position),
                v: self.hsv_value.clamp(0.0, 1.0),
                a: 1.0,
            }
            .to_hsla_ext(),
            ColorRingRasterMode::Lightness => hsla(
                self.hue.rem_euclid(360.0) / 360.0,
                self.saturation.clamp(0.0, 1.0),
                mirrored_lightness(position),
                1.0,
            ),
        }
    }

    fn current_size(&self, circle: &ColorRingState) -> Size<Pixels> {
        if circle.bounds.size.width > px(0.0) && circle.bounds.size.height > px(0.0) {
            circle.bounds.size
        } else {
            let side = component_size_px(circle.size);
            size(px(side), px(side))
        }
    }

    fn ensure_cache(
        &self,
        circle: &ColorRingState,
        image_size: Size<Pixels>,
    ) -> Option<Arc<Image>> {
        let key = self.cache_key(circle);
        if let Some((cached_size, cached_key, image)) = self.image_cache.borrow().as_ref() {
            if *cached_size == image_size && *cached_key == key {
                return Some(image.clone());
            }
        }

        let cache_scale = Self::cache_scale(circle.size);
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
        let ring_thickness = circle.ring_thickness_px() * cache_scale;
        let inner_radius = (outer_radius - ring_thickness).max(0.0);
        let rotation_turns = circle.rotation_turns();

        for y in 0..height {
            for x in 0..width {
                let mut r_sum = 0.0_f32;
                let mut g_sum = 0.0_f32;
                let mut b_sum = 0.0_f32;
                let mut covered = 0_u8;

                for sy in [0.25_f32, 0.75_f32] {
                    for sx in [0.25_f32, 0.75_f32] {
                        let dx = x as f32 + sx - center_x;
                        let dy = y as f32 + sy - center_y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist > outer_radius || dist < inner_radius {
                            continue;
                        }

                        let theta = dy.atan2(dx);
                        let position = (0.25 + (theta / TAU) - rotation_turns).rem_euclid(1.0);
                        let rgb = self.color_at_position(position).to_rgb();
                        r_sum += rgb.r;
                        g_sum += rgb.g;
                        b_sum += rgb.b;
                        covered += 1;
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

impl ColorRingDelegate for RasterRingDelegate {
    fn style_background(
        &self,
        circle: &ColorRingState,
        container: Div,
        _window: &mut Window,
        _cx: &App,
    ) -> Div {
        let image = self.ensure_cache(circle, self.current_size(circle));
        container.when_some(image, |this, image| {
            this.child(
                img(image)
                    .size_full()
                    .absolute()
                    .top_0()
                    .left_0()
                    .rounded_full(),
            )
        })
    }

    fn get_color_at_position(&self, _circle: &ColorRingState, position: f32) -> Hsla {
        self.color_at_position(position)
    }

    fn position_to_value(&self, _circle: &ColorRingState, position: f32) -> f32 {
        self.position_to_value_percent(position)
    }

    fn value_to_position(&self, _circle: &ColorRingState, value: f32) -> f32 {
        self.position_from_value_percent(value)
    }
}

fn normalize_hue_degrees(hue: f32) -> f32 {
    hue.rem_euclid(360.0)
}
