use anyhow::anyhow;
use gpui::Hsla;
use std::hash::{Hash, Hasher};

/// An Oklch color without alpha, based on the OKLab perceptual color space.
///
/// <https://oklch.fyi>
#[derive(Debug, Clone, Copy)]
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
}
