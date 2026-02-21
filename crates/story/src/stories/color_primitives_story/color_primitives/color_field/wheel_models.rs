#![allow(dead_code)]

use crate::stories::color_primitives_story::color_spec::Hsv;
use super::model::{ColorFieldModel2D, ColorFieldModelKind};
use gpui::{hsla, Hsla};
use std::f32::consts::TAU;

const WHITE_MIX_HUE_WHEEL_CACHE_KEY: u64 = 0x1001;
const HSL_WHEEL_CACHE_KEY: u64 = 0x1002;
const GAMMA_HSV_WHEEL_CACHE_KEY: u64 = 0x1003;
const OKLCH_WHEEL_CACHE_KEY: u64 = 0x1004;

macro_rules! impl_wheel_model {
    ($model:ty, $cache_key:expr, |$hsv:ident, $uv:ident| $color_expr:expr) => {
        impl ColorFieldModel2D for $model {
            fn apply_uv(&self, hsv: &mut Hsv, uv: (f32, f32)) {
                apply_wheel_uv(hsv, uv);
            }

            fn uv_from_hsv(&self, hsv: &Hsv) -> (f32, f32) {
                wheel_uv_from_hs(hsv.h, hsv.s)
            }

            fn color_at_uv(&self, $hsv: &Hsv, $uv: (f32, f32)) -> Hsla {
                $color_expr
            }

            fn cache_key_part(&self) -> u64 {
                $cache_key
            }

            fn kind(&self) -> ColorFieldModelKind {
                ColorFieldModelKind::HueSaturationWheel
            }

            fn thumb_color(&self, hsv: &Hsv) -> Hsla {
                self.color_at_uv(hsv, self.uv_from_hsv(hsv))
            }
        }
    };
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WhiteMixHueWheelModel;

impl_wheel_model!(
    WhiteMixHueWheelModel,
    WHITE_MIX_HUE_WHEEL_CACHE_KEY,
    |_hsv, uv| {
        let (hue, saturation) = wheel_hs_from_uv(uv);
        let hue_rgb = hsla(hue / 360.0, 1.0, 0.5, 1.0).to_rgb();
        let sat = saturation.clamp(0.0, 1.0);

        rgba_to_hsla(
            1.0 * (1.0 - sat) + hue_rgb.r * sat,
            1.0 * (1.0 - sat) + hue_rgb.g * sat,
            1.0 * (1.0 - sat) + hue_rgb.b * sat,
            1.0,
        )
    }
);

#[derive(Clone, Copy, Debug, Default)]
pub struct HslWheelModel;

impl_wheel_model!(
    HslWheelModel,
    HSL_WHEEL_CACHE_KEY,
    |hsv, uv| {
        let (hue, saturation) = wheel_hs_from_uv(uv);
        hsla(
            hue / 360.0,
            saturation,
            hsv.v.clamp(0.0, 1.0),
            hsv.a.clamp(0.0, 1.0),
        )
    }
);

#[derive(Clone, Copy, Debug, Default)]
pub struct GammaCorrectedHsvWheelModel;

impl_wheel_model!(
    GammaCorrectedHsvWheelModel,
    GAMMA_HSV_WHEEL_CACHE_KEY,
    |_hsv, uv| {
        let (hue, saturation) = wheel_hs_from_uv(uv);
        let hue_rgb = Hsv {
            h: hue,
            s: 1.0,
            v: 1.0,
            a: 1.0,
        }
        .to_hsla_ext()
        .to_rgb();
        let sat = saturation.clamp(0.0, 1.0);

        let white_linear = 1.0;
        let r_linear = white_linear * (1.0 - sat) + srgb_to_linear(hue_rgb.r) * sat;
        let g_linear = white_linear * (1.0 - sat) + srgb_to_linear(hue_rgb.g) * sat;
        let b_linear = white_linear * (1.0 - sat) + srgb_to_linear(hue_rgb.b) * sat;

        rgba_to_hsla(
            linear_to_srgb(r_linear),
            linear_to_srgb(g_linear),
            linear_to_srgb(b_linear),
            1.0,
        )
    }
);

#[derive(Clone, Copy, Debug, Default)]
pub struct OklchWheelModel;

impl_wheel_model!(
    OklchWheelModel,
    OKLCH_WHEEL_CACHE_KEY,
    |hsv, uv| {
        let (hue, saturation) = wheel_hs_from_uv(uv);
        let sat = saturation.clamp(0.0, 1.0);
        let rim_lightness = (0.35 + hsv.v.clamp(0.0, 1.0) * 0.5).clamp(0.0, 1.0);
        let l = 1.0 - sat * (1.0 - rim_lightness);
        let c = sat * 0.33;
        let (r, g, b) = oklch_to_srgb_gamut_mapped(l, c, hue);
        rgba_to_hsla(r, g, b, hsv.a.clamp(0.0, 1.0))
    }
);

fn apply_wheel_uv(hsv: &mut Hsv, uv: (f32, f32)) {
    let (hue, saturation) = wheel_hs_from_uv(uv);
    hsv.h = hue;
    hsv.s = saturation;
}

fn wheel_hs_from_uv(uv: (f32, f32)) -> (f32, f32) {
    let dx = uv.0 - 0.5;
    let dy = 0.5 - uv.1;
    let hue = dy.atan2(dx).rem_euclid(TAU).to_degrees();
    let saturation = ((dx * dx + dy * dy).sqrt() / 0.5).clamp(0.0, 1.0);
    (hue, saturation)
}

fn wheel_uv_from_hs(hue_degrees: f32, saturation: f32) -> (f32, f32) {
    let angle = hue_degrees.rem_euclid(360.0).to_radians();
    let radius = saturation.clamp(0.0, 1.0) * 0.5;
    (
        (0.5 + radius * angle.cos()).clamp(0.0, 1.0),
        (0.5 - radius * angle.sin()).clamp(0.0, 1.0),
    )
}

fn rgba_to_hsla(r: f32, g: f32, b: f32, a: f32) -> Hsla {
    let r = r.clamp(0.0, 1.0);
    let g = g.clamp(0.0, 1.0);
    let b = b.clamp(0.0, 1.0);
    let a = a.clamp(0.0, 1.0);

    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let delta = max - min;
    let lightness = (max + min) * 0.5;

    if delta <= f32::EPSILON {
        return hsla(0.0, 0.0, lightness, a);
    }

    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs()).max(f32::EPSILON);
    let hue_sector = if (max - r).abs() <= f32::EPSILON {
        ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() <= f32::EPSILON {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    let hue_degrees = 60.0 * hue_sector;

    hsla(
        (hue_degrees / 360.0).rem_euclid(1.0),
        saturation.clamp(0.0, 1.0),
        lightness.clamp(0.0, 1.0),
        a,
    )
}

fn srgb_to_linear(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

fn oklch_to_srgb_gamut_mapped(l: f32, c: f32, h_degrees: f32) -> (f32, f32, f32) {
    if let Some(rgb) = oklch_to_srgb_if_in_gamut(l, c, h_degrees) {
        return rgb;
    }

    let mut lo = 0.0;
    let mut hi = c.max(0.0);
    let mut best = (1.0, 1.0, 1.0);

    for _ in 0..12 {
        let mid = (lo + hi) * 0.5;
        if let Some(rgb) = oklch_to_srgb_if_in_gamut(l, mid, h_degrees) {
            lo = mid;
            best = rgb;
        } else {
            hi = mid;
        }
    }

    best
}

fn oklch_to_srgb_if_in_gamut(l: f32, c: f32, h_degrees: f32) -> Option<(f32, f32, f32)> {
    let h = h_degrees.to_radians();
    let a = c * h.cos();
    let b = c * h.sin();

    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;

    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;

    let r_linear = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
    let g_linear = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3;
    let b_linear = -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;

    if !(0.0..=1.0).contains(&r_linear)
        || !(0.0..=1.0).contains(&g_linear)
        || !(0.0..=1.0).contains(&b_linear)
    {
        return None;
    }

    Some((
        linear_to_srgb(r_linear).clamp(0.0, 1.0),
        linear_to_srgb(g_linear).clamp(0.0, 1.0),
        linear_to_srgb(b_linear).clamp(0.0, 1.0),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) {
        const EPSILON: f32 = 2e-5;
        assert!(
            (a - b).abs() <= EPSILON,
            "expected {a} ~= {b}, delta={}",
            (a - b).abs()
        );
    }

    #[test]
    fn wheel_uv_round_trip_is_stable() {
        let uv = wheel_uv_from_hs(210.0, 0.7);
        let (h, s) = wheel_hs_from_uv(uv);
        approx_eq(h, 210.0);
        approx_eq(s, 0.7);
    }

    #[test]
    fn model_cache_keys_are_unique() {
        assert_ne!(WHITE_MIX_HUE_WHEEL_CACHE_KEY, HSL_WHEEL_CACHE_KEY);
        assert_ne!(WHITE_MIX_HUE_WHEEL_CACHE_KEY, GAMMA_HSV_WHEEL_CACHE_KEY);
        assert_ne!(WHITE_MIX_HUE_WHEEL_CACHE_KEY, OKLCH_WHEEL_CACHE_KEY);
        assert_ne!(HSL_WHEEL_CACHE_KEY, GAMMA_HSV_WHEEL_CACHE_KEY);
        assert_ne!(HSL_WHEEL_CACHE_KEY, OKLCH_WHEEL_CACHE_KEY);
        assert_ne!(GAMMA_HSV_WHEEL_CACHE_KEY, OKLCH_WHEEL_CACHE_KEY);
    }

    #[test]
    fn gamma_corrected_center_is_white() {
        let model = GammaCorrectedHsvWheelModel;
        let color = model.color_at_uv(
            &Hsv {
                h: 0.0,
                s: 0.0,
                v: 0.5,
                a: 1.0,
            },
            (0.5, 0.5),
        );
        let rgb = color.to_rgb();
        approx_eq(rgb.r, 1.0);
        approx_eq(rgb.g, 1.0);
        approx_eq(rgb.b, 1.0);
    }

    #[test]
    fn oklch_gamut_mapping_stays_bounded() {
        let (r, g, b) = oklch_to_srgb_gamut_mapped(0.7, 1.2, 45.0);
        assert!((0.0..=1.0).contains(&r));
        assert!((0.0..=1.0).contains(&g));
        assert!((0.0..=1.0).contains(&b));
    }
}
