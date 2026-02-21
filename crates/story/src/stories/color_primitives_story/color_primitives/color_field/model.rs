use crate::stories::color_primitives_story::color_spec::Hsv;
use gpui::Hsla;
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorFieldModelKind {
    Generic,
    SvAtHue,
    HsAtValue,
    HvAtSaturation,
    HueSaturationLightness,
    HueSaturationWheel,
}

pub trait ColorFieldModel2D: 'static {
    fn apply_uv(&self, hsv: &mut Hsv, uv: (f32, f32));
    fn uv_from_hsv(&self, hsv: &Hsv) -> (f32, f32);
    fn color_at_uv(&self, hsv: &Hsv, uv: (f32, f32)) -> Hsla;
    fn cache_key_part(&self) -> u64 {
        0
    }
    fn kind(&self) -> ColorFieldModelKind {
        ColorFieldModelKind::Generic
    }

    fn thumb_color(&self, hsv: &Hsv) -> Hsla {
        hsv.to_hsla_ext()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SvAtHueModel;

impl ColorFieldModel2D for SvAtHueModel {
    fn apply_uv(&self, hsv: &mut Hsv, uv: (f32, f32)) {
        hsv.s = uv.0.clamp(0.0, 1.0);
        hsv.v = (1.0 - uv.1).clamp(0.0, 1.0);
    }

    fn uv_from_hsv(&self, hsv: &Hsv) -> (f32, f32) {
        (hsv.s.clamp(0.0, 1.0), (1.0 - hsv.v).clamp(0.0, 1.0))
    }

    fn color_at_uv(&self, hsv: &Hsv, uv: (f32, f32)) -> Hsla {
        Hsv {
            h: hsv.h,
            s: uv.0.clamp(0.0, 1.0),
            v: (1.0 - uv.1).clamp(0.0, 1.0),
            a: 1.0,
        }
        .to_hsla_ext()
    }

    fn kind(&self) -> ColorFieldModelKind {
        ColorFieldModelKind::SvAtHue
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub struct HsAtValueModel;

impl ColorFieldModel2D for HsAtValueModel {
    fn apply_uv(&self, hsv: &mut Hsv, uv: (f32, f32)) {
        hsv.h = (uv.0.clamp(0.0, 1.0) * 360.0).clamp(0.0, 360.0);
        hsv.s = (1.0 - uv.1).clamp(0.0, 1.0);
    }

    fn uv_from_hsv(&self, hsv: &Hsv) -> (f32, f32) {
        (
            (hsv.h / 360.0).clamp(0.0, 1.0),
            (1.0 - hsv.s).clamp(0.0, 1.0),
        )
    }

    fn color_at_uv(&self, hsv: &Hsv, uv: (f32, f32)) -> Hsla {
        Hsv {
            h: (uv.0.clamp(0.0, 1.0) * 360.0).clamp(0.0, 360.0),
            s: (1.0 - uv.1).clamp(0.0, 1.0),
            v: hsv.v.clamp(0.0, 1.0),
            a: 1.0,
        }
        .to_hsla_ext()
    }

    fn kind(&self) -> ColorFieldModelKind {
        ColorFieldModelKind::HsAtValue
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub struct HvAtSaturationModel;

impl ColorFieldModel2D for HvAtSaturationModel {
    fn apply_uv(&self, hsv: &mut Hsv, uv: (f32, f32)) {
        hsv.h = (uv.0.clamp(0.0, 1.0) * 360.0).clamp(0.0, 360.0);
        hsv.v = (1.0 - uv.1).clamp(0.0, 1.0);
    }

    fn uv_from_hsv(&self, hsv: &Hsv) -> (f32, f32) {
        (
            (hsv.h / 360.0).clamp(0.0, 1.0),
            (1.0 - hsv.v).clamp(0.0, 1.0),
        )
    }

    fn color_at_uv(&self, hsv: &Hsv, uv: (f32, f32)) -> Hsla {
        Hsv {
            h: (uv.0.clamp(0.0, 1.0) * 360.0).clamp(0.0, 360.0),
            s: hsv.s.clamp(0.0, 1.0),
            v: (1.0 - uv.1).clamp(0.0, 1.0),
            a: 1.0,
        }
        .to_hsla_ext()
    }

    fn kind(&self) -> ColorFieldModelKind {
        ColorFieldModelKind::HvAtSaturation
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HueSaturationLightnessModel;

impl ColorFieldModel2D for HueSaturationLightnessModel {
    fn apply_uv(&self, hsv: &mut Hsv, uv: (f32, f32)) {
        let y = uv.1.clamp(0.0, 1.0);
        hsv.h = (uv.0.clamp(0.0, 1.0) * 360.0).clamp(0.0, 360.0);
        if y <= 0.5 {
            hsv.s = (2.0 * y).clamp(0.0, 1.0);
            hsv.v = 1.0;
        } else {
            hsv.s = 1.0;
            hsv.v = (2.0 * (1.0 - y)).clamp(0.0, 1.0);
        }
    }

    fn uv_from_hsv(&self, hsv: &Hsv) -> (f32, f32) {
        let y = if hsv.v >= 0.999 {
            0.5 * hsv.s
        } else {
            0.5 + 0.5 * (1.0 - hsv.v)
        };
        ((hsv.h / 360.0).clamp(0.0, 1.0), y.clamp(0.0, 1.0))
    }

    fn color_at_uv(&self, _hsv: &Hsv, uv: (f32, f32)) -> Hsla {
        let y = uv.1.clamp(0.0, 1.0);
        let h = (uv.0.clamp(0.0, 1.0) * 360.0).clamp(0.0, 360.0);
        let (s, v) = if y <= 0.5 {
            ((2.0 * y).clamp(0.0, 1.0), 1.0)
        } else {
            (1.0, (2.0 * (1.0 - y)).clamp(0.0, 1.0))
        };

        Hsv { h, s, v, a: 1.0 }.to_hsla_ext()
    }

    fn kind(&self) -> ColorFieldModelKind {
        ColorFieldModelKind::HueSaturationLightness
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HueSaturationWheelModel;

impl ColorFieldModel2D for HueSaturationWheelModel {
    fn apply_uv(&self, hsv: &mut Hsv, uv: (f32, f32)) {
        let dx = uv.0 - 0.5;
        let dy = 0.5 - uv.1;
        let hue = dy.atan2(dx).rem_euclid(TAU).to_degrees();
        let saturation = ((dx * dx + dy * dy).sqrt() / 0.5).clamp(0.0, 1.0);

        hsv.h = hue;
        hsv.s = saturation;
    }

    fn uv_from_hsv(&self, hsv: &Hsv) -> (f32, f32) {
        let angle = hsv.h.rem_euclid(360.0).to_radians();
        let radius = hsv.s.clamp(0.0, 1.0) * 0.5;
        (
            (0.5 + radius * angle.cos()).clamp(0.0, 1.0),
            (0.5 - radius * angle.sin()).clamp(0.0, 1.0),
        )
    }

    fn color_at_uv(&self, hsv: &Hsv, uv: (f32, f32)) -> Hsla {
        let dx = uv.0 - 0.5;
        let dy = 0.5 - uv.1;
        let hue = dy.atan2(dx).rem_euclid(TAU).to_degrees();
        let saturation = ((dx * dx + dy * dy).sqrt() / 0.5).clamp(0.0, 1.0);

        Hsv {
            h: hue,
            s: saturation,
            v: hsv.v.clamp(0.0, 1.0),
            a: 1.0,
        }
        .to_hsla_ext()
    }

    fn kind(&self) -> ColorFieldModelKind {
        ColorFieldModelKind::HueSaturationWheel
    }
}
