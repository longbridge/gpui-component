use super::super::domain::FieldDomain2D;
use super::super::model::{ColorFieldModel2D, ColorFieldModelKind};
use crate::stories::color_primitives_story::color_spec::Hsv;
use gpui::{black, white, *};
use std::sync::Arc;
use tiny_skia::{Pixmap, PremultipliedColorU8};

const RASTER_SUBSAMPLE_OFFSETS: [f32; 2] = [0.25, 0.75];

pub(super) fn model_kind_code(kind: ColorFieldModelKind) -> u16 {
    match kind {
        ColorFieldModelKind::Generic => 0,
        ColorFieldModelKind::SvAtHue => 1,
        ColorFieldModelKind::HsAtValue => 2,
        ColorFieldModelKind::HvAtSaturation => 3,
        ColorFieldModelKind::HueSaturationLightness => 4,
        ColorFieldModelKind::HueSaturationWheel => 5,
    }
}

pub(super) fn quantized_background_hsv(model_kind: u16, hsv: Hsv) -> (u16, u16, u16, u16) {
    let hue = (hsv.h.clamp(0.0, 360.0) * 10.0).round() as u16;
    let saturation = (hsv.s.clamp(0.0, 1.0) * 1000.0).round() as u16;
    let value = (hsv.v.clamp(0.0, 1.0) * 1000.0).round() as u16;
    let alpha = (hsv.a.clamp(0.0, 1.0) * 1000.0).round() as u16;

    match model_kind {
        // SV-at-hue background depends only on hue.
        1 => (hue, 0, 0, 0),
        // HS-at-value background depends only on value.
        2 => (0, 0, value, 0),
        // HV-at-saturation background depends only on saturation.
        3 => (0, saturation, 0, 0),
        // Photoshop HSL background is static for the model.
        4 => (0, 0, 0, 0),
        // Hue/Saturation wheel background depends only on fixed value.
        5 => (0, 0, value, 0),
        // Generic model fallback: conservatively include full HSVA.
        _ => (hue, saturation, value, alpha),
    }
}

pub(super) fn domain_outline_hash(domain: &dyn FieldDomain2D) -> u64 {
    let mut hash = 1469598103934665603u64;
    for (x, y) in domain.outline_points() {
        let xi = (x.clamp(0.0, 1.0) * 8192.0).round() as i32 as u32 as u64;
        let yi = (y.clamp(0.0, 1.0) * 8192.0).round() as i32 as u32 as u64;

        hash ^= xi;
        hash = hash.wrapping_mul(1099511628211u64);
        hash ^= yi;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}

fn raster_scale_for_size(size: Size<Pixels>) -> f32 {
    let max_side = size.width.max(size.height).as_f32();
    if max_side <= 180.0 {
        3.0
    } else if max_side <= 300.0 {
        2.0
    } else {
        1.5
    }
}

pub(super) fn rasterize_domain_image(
    size: Size<Pixels>,
    domain: &dyn FieldDomain2D,
    model: &dyn ColorFieldModel2D,
    hsv: Hsv,
    _samples_per_axis: usize,
) -> Option<Arc<Image>> {
    if domain.is_circle() && model.kind() == ColorFieldModelKind::HueSaturationWheel {
        return rasterize_hue_saturation_wheel_image(size, model, hsv);
    }

    let scale = raster_scale_for_size(size);
    let width = (size.width.as_f32() * scale).round() as u32;
    let height = (size.height.as_f32() * scale).round() as u32;
    if width == 0 || height == 0 {
        return None;
    }

    let mut pixmap = Pixmap::new(width, height)?;
    let pixels = pixmap.pixels_mut();
    let hsl_photoshop_bar_start_u = hsl_photoshop_bar_start_u(size, model.kind(), domain.is_rect());

    for y in 0..height {
        for x in 0..width {
            let mut r_sum = 0.0_f32;
            let mut g_sum = 0.0_f32;
            let mut b_sum = 0.0_f32;
            let mut a_sum = 0.0_f32;
            let mut covered = 0_u8;

            for sy in RASTER_SUBSAMPLE_OFFSETS {
                for sx in RASTER_SUBSAMPLE_OFFSETS {
                    let uv = (
                        ((x as f32 + sx) / width as f32).clamp(0.0, 1.0),
                        ((y as f32 + sy) / height as f32).clamp(0.0, 1.0),
                    );
                    if !domain.contains_uv(uv) {
                        continue;
                    }

                    let hsla = if let Some(bar_start_u) = hsl_photoshop_bar_start_u {
                        hsl_photoshop_raster_color_at_uv(model, hsv, uv, bar_start_u)
                    } else {
                        model.color_at_uv(&hsv, uv)
                    };
                    let rgb = hsla.to_rgb();
                    let alpha = hsla.a.clamp(0.0, 1.0);
                    r_sum += rgb.r * alpha;
                    g_sum += rgb.g * alpha;
                    b_sum += rgb.b * alpha;
                    a_sum += alpha;
                    covered += 1;
                }
            }

            if covered == 0 {
                continue;
            }

            let inv = 1.0 / covered as f32;
            let alpha = (a_sum * inv).clamp(0.0, 1.0);
            let r_u8 = ((r_sum * inv).clamp(0.0, 1.0) * 255.0).round() as u8;
            let g_u8 = ((g_sum * inv).clamp(0.0, 1.0) * 255.0).round() as u8;
            let b_u8 = ((b_sum * inv).clamp(0.0, 1.0) * 255.0).round() as u8;
            let a_u8 = (alpha * 255.0).round() as u8;

            if let Some(pixel) = PremultipliedColorU8::from_rgba(r_u8, g_u8, b_u8, a_u8) {
                pixels[(y * width + x) as usize] = pixel;
            }
        }
    }

    let png_data = pixmap.encode_png().ok()?;
    Some(Arc::new(Image::from_bytes(ImageFormat::Png, png_data)))
}

fn hsl_photoshop_bar_start_u(
    size: Size<Pixels>,
    kind: ColorFieldModelKind,
    is_rect_domain: bool,
) -> Option<f32> {
    if kind != ColorFieldModelKind::HueSaturationLightness || !is_rect_domain {
        return None;
    }

    let width = size.width.as_f32().max(0.0);
    if width <= 0.0 {
        return None;
    }

    let bar_width = width.min(6.0);
    Some((1.0_f32 - (bar_width / width)).clamp(0.0, 1.0))
}

fn hsl_photoshop_raster_color_at_uv(
    model: &dyn ColorFieldModel2D,
    hsv: Hsv,
    uv: (f32, f32),
    bar_start_u: f32,
) -> Hsla {
    if uv.0 >= bar_start_u {
        if uv.1 < 0.5 {
            white()
        } else {
            black()
        }
    } else {
        model.color_at_uv(&hsv, uv)
    }
}

fn rasterize_hue_saturation_wheel_image(
    size: Size<Pixels>,
    model: &dyn ColorFieldModel2D,
    hsv: Hsv,
) -> Option<Arc<Image>> {
    let scale = raster_scale_for_size(size);
    let width = (size.width.as_f32() * scale).round() as u32;
    let height = (size.height.as_f32() * scale).round() as u32;
    if width == 0 || height == 0 {
        return None;
    }

    let mut pixmap = Pixmap::new(width, height)?;
    let pixels = pixmap.pixels_mut();
    let center_x = width as f32 * 0.5;
    let center_y = height as f32 * 0.5;
    let max_radius_sq = (width.min(height) as f32 * 0.5).powi(2);

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 + 0.5 - center_x;
            let dy = y as f32 + 0.5 - center_y;
            if dx * dx + dy * dy > max_radius_sq {
                continue;
            }

            let uv = (
                ((x as f32 + 0.5) / width as f32).clamp(0.0, 1.0),
                ((y as f32 + 0.5) / height as f32).clamp(0.0, 1.0),
            );
            let hsla = model.color_at_uv(&hsv, uv);
            let rgb = hsla.to_rgb();
            let alpha = hsla.a.clamp(0.0, 1.0);
            let r_u8 = (rgb.r.clamp(0.0, 1.0) * alpha * 255.0).round() as u8;
            let g_u8 = (rgb.g.clamp(0.0, 1.0) * alpha * 255.0).round() as u8;
            let b_u8 = (rgb.b.clamp(0.0, 1.0) * alpha * 255.0).round() as u8;
            let a_u8 = (alpha * 255.0).round() as u8;

            if let Some(pixel) = PremultipliedColorU8::from_rgba(r_u8, g_u8, b_u8, a_u8) {
                pixels[(y * width + x) as usize] = pixel;
            }
        }
    }

    let png_data = pixmap.encode_png().ok()?;
    Some(Arc::new(Image::from_bytes(ImageFormat::Png, png_data)))
}
