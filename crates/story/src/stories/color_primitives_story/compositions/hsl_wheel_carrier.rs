use super::super::color_spec::Hsv;
use gpui::{Hsla, hsla};

/// Explicit adapter between an HSL wheel UI state and `ColorFieldState`'s HSV payload.
///
/// For wheel models, `Hsv.v` is intentionally used as an HSL lightness carrier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HslWheelCarrier {
    pub hue_degrees: f32,
    pub saturation: f32,
    pub lightness: f32,
    pub alpha: f32,
}

impl HslWheelCarrier {
    pub fn from_hsla(color: Hsla) -> Self {
        Self {
            hue_degrees: (color.h * 360.0).rem_euclid(360.0),
            saturation: color.s.clamp(0.0, 1.0),
            lightness: color.l.clamp(0.0, 1.0),
            alpha: color.a.clamp(0.0, 1.0),
        }
    }

    pub fn from_field_hsv(hsv: Hsv) -> Self {
        Self {
            hue_degrees: hsv.h.clamp(0.0, 360.0),
            saturation: hsv.s.clamp(0.0, 1.0),
            lightness: hsv.v.clamp(0.0, 1.0),
            alpha: hsv.a.clamp(0.0, 1.0),
        }
    }

    pub fn into_field_hsv(self) -> Hsv {
        Hsv {
            h: self.hue_degrees.clamp(0.0, 360.0),
            s: self.saturation.clamp(0.0, 1.0),
            // Intentional: ColorField wheel models use `v` as an HSL lightness carrier.
            v: self.lightness.clamp(0.0, 1.0),
            a: self.alpha.clamp(0.0, 1.0),
        }
    }

    pub fn to_hsla(self) -> Hsla {
        hsla(
            (self.hue_degrees / 360.0).rem_euclid(1.0),
            self.saturation.clamp(0.0, 1.0),
            self.lightness.clamp(0.0, 1.0),
            self.alpha.clamp(0.0, 1.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) {
        assert!(
            (a - b).abs() < 1e-6,
            "expected {a} ~= {b}, delta={}",
            (a - b).abs()
        );
    }

    #[test]
    fn from_hsla_maps_lightness_into_field_value() {
        let source = hsla(0.5, 0.25, 0.75, 0.6);
        let field = HslWheelCarrier::from_hsla(source).into_field_hsv();

        approx_eq(field.h, 180.0);
        approx_eq(field.s, 0.25);
        approx_eq(field.v, 0.75);
        approx_eq(field.a, 0.6);
    }

    #[test]
    fn field_round_trip_preserves_hsl_channels() {
        let carrier = HslWheelCarrier {
            hue_degrees: 315.0,
            saturation: 0.7,
            lightness: 0.35,
            alpha: 0.9,
        };

        let round_trip = HslWheelCarrier::from_field_hsv(carrier.into_field_hsv());
        assert_eq!(round_trip, carrier);
    }
}
