use gpui::{Hsla, Rgba};

pub fn interpolate_rgb(start: Hsla, end: Hsla, t: f32) -> Hsla {
    let start_rgba: Rgba = start.into();
    let end_rgba: Rgba = end.into();

    let r = start_rgba.r + (end_rgba.r - start_rgba.r) * t;
    let g = start_rgba.g + (end_rgba.g - start_rgba.g) * t;
    let b = start_rgba.b + (end_rgba.b - start_rgba.b) * t;
    let a = start_rgba.a + (end_rgba.a - start_rgba.a) * t;

    Rgba { r, g, b, a }.into()
}

pub fn interpolate_hsl(start: Hsla, end: Hsla, t: f32) -> Hsla {
    // Hue interpolation needs to handle the wrap-around
    let mut h1 = start.h;
    let mut h2 = end.h;

    let dh = h2 - h1;
    if dh > 0.5 {
        h1 += 1.0;
    } else if dh < -0.5 {
        h2 += 1.0;
    }

    let h = (h1 + (h2 - h1) * t) % 1.0;
    let s = start.s + (end.s - start.s) * t;
    let l = start.l + (end.l - start.l) * t;
    let a = start.a + (end.a - start.a) * t;

    gpui::hsla(h, s, l, a)
}

/// A simple Lab-like interpolation using XYZ as an intermediary.
/// For true Lab we'd need more complex math, but this is better than RGB.
pub fn interpolate_lab(start: Hsla, end: Hsla, t: f32) -> Hsla {
    let start_rgb: Rgba = start.into();
    let end_rgb: Rgba = end.into();

    // To Lab (simplified)
    let (l1, a1, b1) = super::rgb_to_lab(start_rgb);
    let (l2, a2, b2) = super::rgb_to_lab(end_rgb);

    let l = l1 + (l2 - l1) * t;
    let a = a1 + (a2 - a1) * t;
    let b = b1 + (b2 - b1) * t;
    let alpha = start_rgb.a + (end_rgb.a - start_rgb.a) * t;

    super::lab_to_rgb(l, a, b, alpha).into()
}
