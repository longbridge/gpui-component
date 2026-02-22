use anyhow::anyhow;
use gpui::Hsla;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::hash::{Hash, Hasher};

/// An Oklch color without alpha, based on the OKLab perceptual color space.
///
/// <https://oklch.fyi>
#[derive(Debug, Clone, Copy, JsonSchema)]
#[repr(C)]
pub struct Oklch {
    /// Lightness (0.0 to 1.0)
    pub l: f32,
    /// Chroma (0.0 to ~0.4 for sRGB gamut, clamped to 0.0..1.0)
    pub c: f32,
    /// Hue (0.0 to 360.0)
    pub h: f32,
    /// Alpha (0.0 to 1.0), default is 1.0 (fully opaque)
    pub a: f32,
}

impl Oklch {
    /// Return a new Oklch color with the same l, c, h values but modified alpha by multiplying with the given factor.
    pub fn opacity(self, factor: f32) -> Self {
        Oklch {
            a: self.a * factor.clamp(0., 1.),
            ..self
        }
    }
}

/// Create an Oklch color from l, c, h values.
///
/// - l: Lightness (0.0 to 1.0)
/// - c: Chroma (0.0 to 1.0)
/// - h: Hue (0.0 to 360.0)
pub fn oklch(l: f32, c: f32, h: f32) -> Oklch {
    Oklch {
        l: l.clamp(0., 1.),
        c: c.clamp(0., 1.),
        h: h.clamp(0., 360.),
        a: 1.0,
    }
}

/// Create an Oklch color with alpha from l, c, h, a values.
///
/// - l: Lightness (0.0 to 1.0)
/// - c: Chroma (0.0 to 1.0)
/// - h: Hue (0.0 to 360.0)
/// - a: Alpha (0.0 to 1.0)
pub fn oklcha(l: f32, c: f32, h: f32, a: f32) -> Oklch {
    Oklch {
        l: l.clamp(0., 1.),
        c: c.clamp(0., 1.),
        h: h.clamp(0., 360.),
        a: a.clamp(0., 1.),
    }
}

impl PartialEq for Oklch {
    fn eq(&self, other: &Self) -> bool {
        self.l
            .total_cmp(&other.l)
            .then(self.c.total_cmp(&other.c))
            .then(self.h.total_cmp(&other.h).then(self.a.total_cmp(&other.a)))
            .is_eq()
    }
}

impl PartialOrd for Oklch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Oklch {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.l
            .total_cmp(&other.l)
            .then(self.c.total_cmp(&other.c))
            .then(self.h.total_cmp(&other.h).then(self.a.total_cmp(&other.a)))
    }
}

impl Eq for Oklch {}

impl Hash for Oklch {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(u32::from_be_bytes(self.l.to_be_bytes()));
        state.write_u32(u32::from_be_bytes(self.c.to_be_bytes()));
        state.write_u32(u32::from_be_bytes(self.h.to_be_bytes()));
        state.write_u32(u32::from_be_bytes(self.a.to_be_bytes()));
    }
}

impl Serialize for Oklch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Hsla::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Oklch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Hsla::deserialize(deserializer)?.into())
    }
}

/// Convert OKLab (L, a, b) to linear sRGB (r, g, b).
fn oklab_to_linear_srgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    (
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    )
}

/// Apply sRGB gamma correction to a linear channel value.
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert sRGB (r, g, b) in [0,1] to HSL (h, s, l) where h, s, l are all in [0,1].
fn srgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < f32::EPSILON {
        let mut h = (g - b) / d;
        if g < b {
            h += 6.0;
        }
        h
    } else if (max - g).abs() < f32::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    (h / 6.0, s, l)
}

/// Convert HSL (h, s, l) where h, s, l are all in [0,1] to sRGB (r, g, b) in [0,1].
fn hsl_to_srgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s.abs() < f32::EPSILON {
        return (l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 1.0 / 2.0 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    }

    (
        hue_to_rgb(p, q, h + 1.0 / 3.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 1.0 / 3.0),
    )
}

/// Remove sRGB gamma to get a linear channel value.
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert linear sRGB (r, g, b) to OKLab (L, a, b).
fn linear_srgb_to_oklab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    (
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    )
}

/// Core conversion: GPUI Hsla → Oklch.
///
/// Conversion path: HSL → sRGB → linear sRGB → OKLab → Oklch
fn hsla_to_oklch(hsla: Hsla) -> Oklch {
    let (r, g, b) = hsl_to_srgb(hsla.h, hsla.s, hsla.l);

    let lr = srgb_to_linear(r);
    let lg = srgb_to_linear(g);
    let lb = srgb_to_linear(b);

    let (ok_l, ok_a, ok_b) = linear_srgb_to_oklab(lr, lg, lb);

    let c = (ok_a * ok_a + ok_b * ok_b).sqrt();
    let h = if c < f32::EPSILON {
        0.0
    } else {
        ok_b.atan2(ok_a).to_degrees().rem_euclid(360.0)
    };

    Oklch {
        l: ok_l,
        c,
        h,
        a: hsla.a,
    }
}

impl From<Hsla> for Oklch {
    fn from(value: Hsla) -> Self {
        hsla_to_oklch(value)
    }
}

/// Core conversion: Oklch (L, C, H) + alpha → GPUI Hsla.
///
/// Conversion path: Oklch → OKLab → linear sRGB → sRGB → HSL
fn oklch_to_hsla(l: f32, c: f32, h: f32, a: f32) -> Hsla {
    let h_rad = h.to_radians();
    let ok_a = c * h_rad.cos();
    let ok_b = c * h_rad.sin();

    let (lr, lg, lb) = oklab_to_linear_srgb(l, ok_a, ok_b);

    let r = linear_to_srgb(lr).clamp(0.0, 1.0);
    let g = linear_to_srgb(lg).clamp(0.0, 1.0);
    let b = linear_to_srgb(lb).clamp(0.0, 1.0);

    let (hh, ss, ll) = srgb_to_hsl(r, g, b);

    Hsla {
        h: hh,
        s: ss,
        l: ll,
        a,
    }
}

impl From<Oklch> for Hsla {
    fn from(value: Oklch) -> Self {
        oklch_to_hsla(value.l, value.c, value.h, value.a)
    }
}

/// Parse string to Oklch color.
///
/// - `oklch(L C H)` or `oklch(L C H / A)` format, where L, C, H are required and A is optional (default 1.0).
/// - `oklch(L, C, H)` or `oklch(L, C, H / A)` format is not supported (commas are not allowed).
impl TryFrom<&str> for Oklch {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let value = value.trim();
        if !value.starts_with("oklch(") || !value.ends_with(')') {
            return Err(anyhow!("Invalid Oklch format"));
        }

        let content = &value[6..value.len() - 1];
        let (color_part, a) = if let Some((cp, ap)) = content.split_once('/') {
            (cp.trim(), ap.trim().parse::<f32>()?)
        } else {
            (content.trim(), 1.0)
        };

        let parts: Vec<&str> = if color_part.contains(',') {
            color_part.split(',').map(|s| s.trim()).collect()
        } else {
            color_part.split_whitespace().collect()
        };
        if parts.len() != 3 {
            return Err(anyhow!(
                "Invalid Oklch format, expected `oklch(L C H)` or `oklch(L C H / A)`"
            ));
        }

        let l = parts[0].parse::<f32>()?;
        let c = parts[1].parse::<f32>()?;
        let h = parts[2].parse::<f32>()?;

        Ok(Oklch { l, c, h, a })
    }
}

#[cfg(test)]
mod tests {
    use gpui::Hsla;

    use super::*;

    fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
        (a - b).abs() < epsilon
    }

    fn hsla_approx_eq(a: Hsla, b: Hsla, epsilon: f32) -> bool {
        approx_eq(a.h, b.h, epsilon)
            && approx_eq(a.s, b.s, epsilon)
            && approx_eq(a.l, b.l, epsilon)
            && approx_eq(a.a, b.a, epsilon)
    }

    #[test]
    fn test_oklch_constructor() {
        let color = oklch(0.5, 0.2, 180.0);
        assert_eq!(color.l, 0.5);
        assert_eq!(color.c, 0.2);
        assert_eq!(color.h, 180.0);
    }

    #[test]
    fn test_oklch_clamp() {
        let color = oklch(-0.1, 1.5, 400.0);
        assert_eq!(color.l, 0.0);
        assert_eq!(color.c, 1.0);
        assert_eq!(color.h, 360.0);
    }

    #[test]
    fn test_oklcha_constructor() {
        let color = oklcha(0.5, 0.2, 180.0, 0.8);
        assert_eq!(color.l, 0.5);
        assert_eq!(color.c, 0.2);
        assert_eq!(color.h, 180.0);
        assert_eq!(color.a, 0.8);
    }

    #[test]
    fn test_oklcha_clamp() {
        let color = oklcha(2.0, -0.5, -10.0, 1.5);
        assert_eq!(color.l, 1.0);
        assert_eq!(color.c, 0.0);
        assert_eq!(color.h, 0.0);
        assert_eq!(color.a, 1.0);
    }

    // --- Oklch → Hsla conversion tests ---

    #[test]
    fn test_oklch_black_to_hsla() {
        let hsla: Hsla = oklch(0.0, 0.0, 0.0).into();
        assert!(hsla_approx_eq(
            hsla,
            Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 1.0
            },
            0.01
        ));
    }

    #[test]
    fn test_oklch_white_to_hsla() {
        let hsla: Hsla = oklch(1.0, 0.0, 0.0).into();
        assert!(hsla_approx_eq(
            hsla,
            Hsla {
                h: 0.0,
                s: 0.0,
                l: 1.0,
                a: 1.0
            },
            0.01
        ));
    }

    #[test]
    fn test_oklch_red_to_hsla() {
        // Red #ff0000 ≈ oklch(0.6279 0.2577 29.23)
        let hsla: Hsla = oklch(0.6279, 0.2577, 29.23).into();
        assert!(approx_eq(hsla.l, 0.5, 0.02));
        assert!(approx_eq(hsla.s, 1.0, 0.05));
        assert_eq!(hsla.a, 1.0);
    }

    #[test]
    fn test_oklch_green_to_hsla() {
        // Green #00ff00 ≈ oklch(0.8664 0.2948 142.50)
        let hsla: Hsla = oklch(0.8664, 0.2948, 142.50).into();
        assert!(approx_eq(hsla.h, 120.0 / 360.0, 0.02));
        assert!(approx_eq(hsla.l, 0.5, 0.02));
        assert!(approx_eq(hsla.s, 1.0, 0.05));
        assert_eq!(hsla.a, 1.0);
    }

    #[test]
    fn test_oklch_blue_to_hsla() {
        // Blue #0000ff ≈ oklch(0.4520 0.3131 264.05)
        let hsla: Hsla = oklch(0.4520, 0.3131, 264.05).into();
        assert!(approx_eq(hsla.h, 240.0 / 360.0, 0.02));
        assert!(approx_eq(hsla.l, 0.5, 0.02));
        assert!(approx_eq(hsla.s, 1.0, 0.05));
        assert_eq!(hsla.a, 1.0);
    }

    #[test]
    fn test_oklch_gray_achromatic() {
        // When chroma is 0, result should be achromatic (saturation ≈ 0)
        let hsla: Hsla = oklch(0.5, 0.0, 0.0).into();
        assert!(approx_eq(hsla.s, 0.0, 0.01));

        // Different hue with zero chroma should give same gray
        let hsla2: Hsla = oklch(0.5, 0.0, 180.0).into();
        assert!(approx_eq(hsla2.s, 0.0, 0.01));
        assert!(approx_eq(hsla.l, hsla2.l, 0.001));
    }

    #[test]
    fn test_oklcha_alpha_preserved() {
        let hsla: Hsla = oklcha(0.0, 0.0, 0.0, 0.5).into();
        assert!(hsla_approx_eq(
            hsla,
            Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.5
            },
            0.01
        ));

        let hsla: Hsla = oklcha(1.0, 0.0, 0.0, 0.25).into();
        assert!(approx_eq(hsla.a, 0.25, 0.001));
    }

    #[test]
    fn test_oklch_lightness_ordering() {
        // Higher OKLab L should produce higher HSL lightness
        let dark: Hsla = oklch(0.2, 0.0, 0.0).into();
        let mid: Hsla = oklch(0.5, 0.0, 0.0).into();
        let light: Hsla = oklch(0.8, 0.0, 0.0).into();
        assert!(dark.l < mid.l);
        assert!(mid.l < light.l);
    }

    // --- TryFrom<&str> parsing tests ---

    #[test]
    fn test_oklch_parse_valid() {
        let color = Oklch::try_from("oklch(0.51 0.1 210)").unwrap();
        assert_eq!(color.l, 0.51);
        assert_eq!(color.c, 0.1);
        assert_eq!(color.h, 210.0);
        assert_eq!(color, Oklch::try_from("oklch(0.51, 0.1, 210)").unwrap())
    }

    #[test]
    fn test_oklch_parse_whitespace() {
        let color = Oklch::try_from("  oklch(0.7 0.15 120)  ").unwrap();
        assert_eq!(color.l, 0.7);
        assert_eq!(color.c, 0.15);
        assert_eq!(color.h, 120.0);
    }

    #[test]
    fn test_oklcha_parse_with_alpha() {
        let color = Oklch::try_from("oklch(0.51 0.1 210 / 0.5)").unwrap();
        assert_eq!(color.l, 0.51);
        assert_eq!(color.c, 0.1);
        assert_eq!(color.h, 210.0);
        assert_eq!(color.a, 0.5);
        assert_eq!(
            color,
            Oklch::try_from("oklch(0.51, 0.1, 210 / 0.5)").unwrap()
        );
    }

    #[test]
    fn test_oklch_parse_invalid() {
        assert!(Oklch::try_from("hsl(0 0% 0%)").is_err());
        assert!(Oklch::try_from("oklch(0.5 0.1)").is_err());
        assert!(Oklch::try_from("oklch(0.5 0.1 210 0.5)").is_err());
        assert!(Oklch::try_from("oklch()").is_err());
        assert!(Oklch::try_from("random text").is_err());
    }

    #[test]
    fn test_oklch_parsed_converts_to_hsla() {
        // Parse and convert: oklch(0.6279 0.2577 29.23) ≈ red
        let color = Oklch::try_from("oklch(0.6279 0.2577 29.23)").unwrap();
        let hsla: Hsla = color.into();
        assert!(approx_eq(hsla.l, 0.5, 0.02));
        assert!(approx_eq(hsla.s, 1.0, 0.05));
    }

    // --- Hsla → Oklch conversion tests ---

    #[test]
    fn test_hsla_black_to_oklch() {
        let oklch: Oklch = Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.0,
            a: 1.0,
        }
        .into();
        assert!(approx_eq(oklch.l, 0.0, 0.01));
        assert!(approx_eq(oklch.c, 0.0, 0.01));
        assert_eq!(oklch.a, 1.0);
    }

    #[test]
    fn test_hsla_white_to_oklch() {
        let oklch: Oklch = Hsla {
            h: 0.0,
            s: 0.0,
            l: 1.0,
            a: 1.0,
        }
        .into();
        assert!(approx_eq(oklch.l, 1.0, 0.01));
        assert!(approx_eq(oklch.c, 0.0, 0.01));
        assert_eq!(oklch.a, 1.0);
    }

    #[test]
    fn test_hsla_red_to_oklch() {
        // hsl(0, 100%, 50%) → oklch(≈0.628, ≈0.258, ≈29.2)
        let oklch: Oklch = Hsla {
            h: 0.0,
            s: 1.0,
            l: 0.5,
            a: 1.0,
        }
        .into();
        assert!(approx_eq(oklch.l, 0.628, 0.01));
        assert!(approx_eq(oklch.c, 0.258, 0.01));
        assert!(approx_eq(oklch.h, 29.2, 1.0));
    }

    #[test]
    fn test_hsla_green_to_oklch() {
        // hsl(120/360, 100%, 50%) → oklch(≈0.866, ≈0.295, ≈142.5)
        let oklch: Oklch = Hsla {
            h: 120.0 / 360.0,
            s: 1.0,
            l: 0.5,
            a: 1.0,
        }
        .into();
        assert!(approx_eq(oklch.l, 0.866, 0.01));
        assert!(approx_eq(oklch.c, 0.295, 0.01));
        assert!(approx_eq(oklch.h, 142.5, 1.0));
    }

    #[test]
    fn test_hsla_blue_to_oklch() {
        // hsl(240/360, 100%, 50%) → oklch(≈0.452, ≈0.313, ≈264.1)
        let oklch: Oklch = Hsla {
            h: 240.0 / 360.0,
            s: 1.0,
            l: 0.5,
            a: 1.0,
        }
        .into();
        assert!(approx_eq(oklch.l, 0.452, 0.01));
        assert!(approx_eq(oklch.c, 0.313, 0.01));
        assert!(approx_eq(oklch.h, 264.1, 1.0));
    }

    #[test]
    fn test_hsla_gray_to_oklch() {
        // Achromatic gray → chroma should be ~0
        let oklch: Oklch = Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.5,
            a: 1.0,
        }
        .into();
        assert!(approx_eq(oklch.c, 0.0, 0.001));
    }

    #[test]
    fn test_hsla_alpha_preserved_to_oklch() {
        let oklch: Oklch = Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.0,
            a: 0.33,
        }
        .into();
        assert!(approx_eq(oklch.a, 0.33, 0.001));
    }

    // --- Roundtrip tests ---

    #[test]
    fn test_oklch_hsla_roundtrip() {
        let cases = [
            oklch(0.0, 0.0, 0.0),          // black
            oklch(1.0, 0.0, 0.0),          // white
            oklch(0.5, 0.0, 0.0),          // gray
            oklch(0.6279, 0.2577, 29.23),  // red
            oklch(0.8664, 0.2948, 142.50), // green
            oklch(0.4520, 0.3131, 264.05), // blue
            oklch(0.7, 0.15, 60.0),        // warm color
            oklch(0.6, 0.1, 300.0),        // purple-ish
        ];

        for original in cases {
            let hsla: Hsla = original.into();
            let roundtrip: Oklch = hsla.into();
            assert!(
                approx_eq(original.l, roundtrip.l, 0.01),
                "L mismatch for {:?}: {} vs {}",
                original,
                original.l,
                roundtrip.l
            );
            // For achromatic colors, chroma and hue may differ, only check chroma ≈ 0
            if original.c < 0.001 {
                assert!(
                    approx_eq(roundtrip.c, 0.0, 0.001),
                    "C mismatch for achromatic {:?}",
                    original
                );
            } else {
                assert!(
                    approx_eq(original.c, roundtrip.c, 0.01),
                    "C mismatch for {:?}: {} vs {}",
                    original,
                    original.c,
                    roundtrip.c
                );
                assert!(
                    approx_eq(original.h, roundtrip.h, 1.0),
                    "H mismatch for {:?}: {} vs {}",
                    original,
                    original.h,
                    roundtrip.h
                );
            }
        }
    }

    #[test]
    fn test_hsla_oklch_roundtrip() {
        let cases = [
            Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 1.0,
            }, // black
            Hsla {
                h: 0.0,
                s: 0.0,
                l: 1.0,
                a: 1.0,
            }, // white
            Hsla {
                h: 0.0,
                s: 1.0,
                l: 0.5,
                a: 1.0,
            }, // red
            Hsla {
                h: 0.333,
                s: 1.0,
                l: 0.5,
                a: 1.0,
            }, // green-ish
            Hsla {
                h: 0.667,
                s: 1.0,
                l: 0.5,
                a: 1.0,
            }, // blue-ish
            Hsla {
                h: 0.5,
                s: 0.5,
                l: 0.7,
                a: 0.8,
            }, // muted cyan
        ];

        for original in cases {
            let oklch: Oklch = original.into();
            let roundtrip: Hsla = oklch.into();
            assert!(
                approx_eq(original.l, roundtrip.l, 0.01),
                "L mismatch for {:?}: {} vs {}",
                original,
                original.l,
                roundtrip.l
            );
            assert!(
                approx_eq(original.a, roundtrip.a, 0.001),
                "A mismatch for {:?}",
                original
            );
            // For achromatic, hue/saturation may drift
            if original.s > 0.01 {
                assert!(
                    approx_eq(original.s, roundtrip.s, 0.02),
                    "S mismatch for {:?}: {} vs {}",
                    original,
                    original.s,
                    roundtrip.s
                );
                assert!(
                    approx_eq(original.h, roundtrip.h, 0.01),
                    "H mismatch for {:?}: {} vs {}",
                    original,
                    original.h,
                    roundtrip.h
                );
            }
        }
    }
}
