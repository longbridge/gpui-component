use gpui::{Hsla, Rgba};

mod interpolation;
pub use interpolation::{interpolate_hsl, interpolate_lab, interpolate_rgb};

#[cfg(test)]
mod tests;

pub mod constants {
    /// The size of the checkerboard squares for alpha backgrounds.
    pub const CHECKERBOARD_SIZE: f32 = 8.0;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorChannel {
    pub name: &'static str,
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub step: Option<f32>,
    pub unit: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HueAlpha {
    pub h: f32,
    pub a: f32,
}

impl HueAlpha {
    pub const HUE: &'static str = "hue";
    pub const ALPHA: &'static str = "alpha";

    const CHANNELS: [ColorChannel; 2] = [
        ColorChannel {
            name: Self::HUE,
            label: "Hue",
            min: 0.0,
            max: 360.0,
            step: Some(1.0),
            unit: "°",
        },
        ColorChannel {
            name: Self::ALPHA,
            label: "Alpha",
            min: 0.0,
            max: 1.0,
            step: None,
            unit: "",
        },
    ];
}

impl ColorSpecification for HueAlpha {
    fn name(&self) -> &'static str {
        "Hue+Alpha"
    }

    fn channels(&self) -> &[ColorChannel] {
        &Self::CHANNELS
    }

    fn get_value(&self, channel_name: &str) -> f32 {
        match channel_name {
            Self::HUE => self.h,
            Self::ALPHA => self.a,
            _ => 0.0,
        }
    }

    fn set_value(&mut self, channel_name: &str, value: f32) {
        match channel_name {
            Self::HUE => self.h = value.clamp(0.0, 360.0),
            Self::ALPHA => self.a = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn to_hsla(&self) -> Hsla {
        Hsla {
            h: self.h / 360.0,
            s: 1.0,
            l: 0.5,
            a: self.a,
        }
    }

    fn from_hsla(hsla: Hsla) -> Self {
        Self {
            h: hsla.h * 360.0,
            a: hsla.a,
        }
    }
}

pub trait ColorSpecification: 'static + Clone + Copy + Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    fn channels(&self) -> &[ColorChannel];
    #[allow(dead_code)]
    fn get_value(&self, channel_name: &str) -> f32;
    fn set_value(&mut self, channel_name: &str, value: f32);
    fn to_hsla(&self) -> Hsla;
    fn from_hsla(hsla: Hsla) -> Self;

    #[allow(dead_code)]
    fn format_value(&self, channel_name: &str) -> String {
        let value = self.get_value(channel_name);
        format!("{:.3}", value)
    }

    // By default, spaces are continuous RGB-bound so this is just their static bounds
    fn channel_bounds(&self, channel_name: &str) -> (f32, f32) {
        let channel = self
            .channels()
            .iter()
            .find(|c| c.name == channel_name)
            .unwrap();
        (channel.min, channel.max)
    }

    #[allow(dead_code)]
    fn set_auto_clamp(&mut self, _auto_clamp: bool) {}

    #[allow(dead_code)]
    fn set_dynamic_range(&mut self, _dynamic_range: bool) {}

    fn clamp_spec_to_gamut(&mut self) {}

    fn clamp_channel_to_gamut(&self, channel_name: &str, proposed: f32) -> f32 {
        let bounds = self.channel_bounds(channel_name);
        proposed.clamp(bounds.0, bounds.1)
    }

    #[allow(dead_code)]
    fn summary(&self) -> String {
        let alpha = self.get_value("alpha");
        if self.name() == "HSL" {
            format!(
                "hsla({:.0}°, {:.0}%, {:.0}%, {:.2})",
                self.get_value("hue"),
                self.get_value("saturation") * 100.0,
                self.get_value("lightness") * 100.0,
                alpha
            )
        } else if self.name() == "HSV" {
            format!(
                "hsva({:.0}°, {:.0}%, {:.0}%, {:.2})",
                self.get_value("hue"),
                self.get_value("saturation") * 100.0,
                self.get_value("value") * 100.0,
                alpha
            )
        } else if self.name() == "RGBA" {
            format!(
                "rgba({:.0}, {:.0}, {:.0}, {:.2})",
                self.get_value("red"),
                self.get_value("green"),
                self.get_value("blue"),
                alpha
            )
        } else if self.name() == "Lab" {
            format!(
                "lab({:.0}, {:.0}, {:.0}, {:.2})",
                self.get_value("lightness"),
                self.get_value("a"),
                self.get_value("b"),
                alpha
            )
        } else if self.name() == "Hue+Alpha" {
            format!("hue+alpha({:.0}°, {:.2})", self.get_value("hue"), alpha)
        } else {
            String::new()
        }
    }

    fn is_out_of_gamut(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsl {
    pub h: f32, // 0..360
    pub s: f32, // 0..1
    pub l: f32, // 0..1
    pub a: f32, // 0..1
}

impl Hsl {
    pub const HUE: &'static str = "hue";
    pub const SATURATION: &'static str = "saturation";
    pub const LIGHTNESS: &'static str = "lightness";
    pub const ALPHA: &'static str = "alpha";

    const CHANNELS: [ColorChannel; 4] = [
        ColorChannel {
            name: Self::HUE,
            label: "Hue",
            min: 0.0,
            max: 360.0,
            step: Some(1.0),
            unit: "°",
        },
        ColorChannel {
            name: Self::SATURATION,
            label: "Saturation",
            min: 0.0,
            max: 1.0,
            step: None,
            unit: "%",
        },
        ColorChannel {
            name: Self::LIGHTNESS,
            label: "Lightness",
            min: 0.0,
            max: 1.0,
            step: None,
            unit: "%",
        },
        ColorChannel {
            name: Self::ALPHA,
            label: "Alpha",
            min: 0.0,
            max: 1.0,
            step: None,
            unit: "",
        },
    ];
}

impl ColorSpecification for Hsl {
    fn name(&self) -> &'static str {
        "HSL"
    }

    fn channels(&self) -> &[ColorChannel] {
        &Self::CHANNELS
    }

    fn get_value(&self, channel_name: &str) -> f32 {
        match channel_name {
            Self::HUE => self.h,
            Self::SATURATION => self.s,
            Self::LIGHTNESS => self.l,
            Self::ALPHA => self.a,
            _ => 0.0,
        }
    }

    fn set_value(&mut self, channel_name: &str, value: f32) {
        match channel_name {
            Self::HUE => self.h = value.clamp(0.0, 360.0),
            Self::SATURATION => self.s = value.clamp(0.0, 1.0),
            Self::LIGHTNESS => self.l = value.clamp(0.0, 1.0),
            Self::ALPHA => self.a = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn to_hsla(&self) -> Hsla {
        Hsla {
            h: self.h / 360.0,
            s: self.s,
            l: self.l,
            a: self.a,
        }
    }

    fn from_hsla(hsla: Hsla) -> Self {
        Self {
            h: hsla.h * 360.0,
            s: hsla.s,
            l: hsla.l,
            a: hsla.a,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    pub h: f32, // 0..360
    pub s: f32, // 0..1
    pub v: f32, // 0..1
    pub a: f32, // 0..1
}

impl Hsv {
    pub const HUE: &'static str = "hue";
    pub const SATURATION: &'static str = "saturation";
    pub const VALUE: &'static str = "value";
    pub const ALPHA: &'static str = "alpha";

    const CHANNELS: [ColorChannel; 4] = [
        ColorChannel {
            name: Self::HUE,
            label: "Hue",
            min: 0.0,
            max: 360.0,
            step: Some(1.0),
            unit: "°",
        },
        ColorChannel {
            name: Self::SATURATION,
            label: "Saturation",
            min: 0.0,
            max: 1.0,
            step: None,
            unit: "%",
        },
        ColorChannel {
            name: Self::VALUE,
            label: "Value",
            min: 0.0,
            max: 1.0,
            step: None,
            unit: "%",
        },
        ColorChannel {
            name: Self::ALPHA,
            label: "Alpha",
            min: 0.0,
            max: 1.0,
            step: None,
            unit: "",
        },
    ];

    pub fn from_hsla_ext(hsla: Hsla) -> Self {
        let rgba = hsla.to_rgb();
        Self::from_rgba(rgba)
    }

    pub fn from_rgba(rgba: Rgba) -> Self {
        let r = rgba.r;
        let g = rgba.g;
        let b = rgba.b;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let d = max - min;

        let s = if max == 0.0 { 0.0 } else { d / max };
        let v = max;

        let mut h = 0.0;
        if max != min {
            if max == r {
                h = (g - b) / d + (if g < b { 6.0 } else { 0.0 });
            } else if max == g {
                h = (b - r) / d + 2.0;
            } else {
                h = (r - g) / d + 4.0;
            }
            h *= 60.0;
        }

        Self { h, s, v, a: rgba.a }
    }

    pub fn to_hsla_ext(self) -> Hsla {
        let h = self.h / 360.0;
        let c = self.v * self.s;
        let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
        let m = self.v - c;

        let (r, g, b) = if h < 1.0 / 6.0 {
            (c, x, 0.0)
        } else if h < 2.0 / 6.0 {
            (x, c, 0.0)
        } else if h < 3.0 / 6.0 {
            (0.0, c, x)
        } else if h < 4.0 / 6.0 {
            (0.0, x, c)
        } else if h < 5.0 / 6.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Rgba {
            r: r + m,
            g: g + m,
            b: b + m,
            a: self.a,
        }
        .into()
    }
}

impl ColorSpecification for Hsv {
    fn name(&self) -> &'static str {
        "HSV"
    }

    fn channels(&self) -> &[ColorChannel] {
        &Self::CHANNELS
    }

    fn get_value(&self, channel_name: &str) -> f32 {
        match channel_name {
            Self::HUE => self.h,
            Self::SATURATION => self.s,
            Self::VALUE => self.v,
            Self::ALPHA => self.a,
            _ => 0.0,
        }
    }

    fn set_value(&mut self, channel_name: &str, value: f32) {
        match channel_name {
            Self::HUE => self.h = value.clamp(0.0, 360.0),
            Self::SATURATION => self.s = value.clamp(0.0, 1.0),
            Self::VALUE => self.v = value.clamp(0.0, 1.0),
            Self::ALPHA => self.a = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn to_hsla(&self) -> Hsla {
        self.to_hsla_ext()
    }

    fn from_hsla(hsla: Hsla) -> Self {
        Self::from_hsla_ext(hsla)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RgbaSpec {
    pub r: f32, // 0..255
    pub g: f32, // 0..255
    pub b: f32, // 0..255
    pub a: f32, // 0..1
}

impl RgbaSpec {
    pub const RED: &'static str = "red";
    pub const GREEN: &'static str = "green";
    pub const BLUE: &'static str = "blue";
    pub const ALPHA: &'static str = "alpha";

    const CHANNELS: [ColorChannel; 4] = [
        ColorChannel {
            name: Self::RED,
            label: "Red",
            min: 0.0,
            max: 255.0,
            step: Some(1.0),
            unit: "",
        },
        ColorChannel {
            name: Self::GREEN,
            label: "Green",
            min: 0.0,
            max: 255.0,
            step: Some(1.0),
            unit: "",
        },
        ColorChannel {
            name: Self::BLUE,
            label: "Blue",
            min: 0.0,
            max: 255.0,
            step: Some(1.0),
            unit: "",
        },
        ColorChannel {
            name: Self::ALPHA,
            label: "Alpha",
            min: 0.0,
            max: 1.0,
            step: None,
            unit: "",
        },
    ];
}

impl ColorSpecification for RgbaSpec {
    fn name(&self) -> &'static str {
        "RGBA"
    }

    fn channels(&self) -> &[ColorChannel] {
        &Self::CHANNELS
    }

    fn get_value(&self, channel_name: &str) -> f32 {
        match channel_name {
            Self::RED => self.r,
            Self::GREEN => self.g,
            Self::BLUE => self.b,
            Self::ALPHA => self.a,
            _ => 0.0,
        }
    }

    fn set_value(&mut self, channel_name: &str, value: f32) {
        match channel_name {
            Self::RED => self.r = value.clamp(0.0, 255.0),
            Self::GREEN => self.g = value.clamp(0.0, 255.0),
            Self::BLUE => self.b = value.clamp(0.0, 255.0),
            Self::ALPHA => self.a = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn to_hsla(&self) -> Hsla {
        Rgba {
            r: self.r / 255.0,
            g: self.g / 255.0,
            b: self.b / 255.0,
            a: self.a,
        }
        .into()
    }

    fn from_hsla(hsla: Hsla) -> Self {
        let rgba = hsla.to_rgb();
        Self {
            r: rgba.r * 255.0,
            g: rgba.g * 255.0,
            b: rgba.b * 255.0,
            a: rgba.a,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lab {
    pub l: f32,     // 0..100
    pub a: f32,     // -128..127
    pub b: f32,     // -128..127
    pub alpha: f32, // 0..1
    pub auto_clamp: bool,
    pub dynamic_range: bool,
}

impl Lab {
    pub const LIGHTNESS: &'static str = "lightness";
    pub const A: &'static str = "a";
    pub const B: &'static str = "b";
    pub const ALPHA: &'static str = "alpha";

    const CHANNELS: [ColorChannel; 4] = [
        ColorChannel {
            name: Self::LIGHTNESS,
            label: "Lightness (L*)",
            min: 0.0,
            max: 100.0,
            step: Some(1.0),
            unit: "",
        },
        ColorChannel {
            name: Self::A,
            label: "Green-Red (a*)",
            min: -128.0,
            max: 127.0,
            step: Some(1.0),
            unit: "",
        },
        ColorChannel {
            name: Self::B,
            label: "Blue-Yellow (b*)",
            min: -128.0,
            max: 127.0,
            step: Some(1.0),
            unit: "",
        },
        ColorChannel {
            name: Self::ALPHA,
            label: "Alpha",
            min: 0.0,
            max: 1.0,
            step: None,
            unit: "",
        },
    ];

    pub fn to_hsla_checked(&self) -> (Hsla, bool) {
        let (rgba, out_of_gamut) = lab_to_rgb_checked(self.l, self.a, self.b, self.alpha);
        (rgba.into(), out_of_gamut)
    }

    // Non-trait Math Helpers originally from LabMixer

    const CLAMP_SEARCH_STEPS: usize = 96;
    const CLAMP_REFINE_STEPS: usize = 14;

    fn spec_in_gamut(spec: Lab) -> bool {
        !spec.to_hsla_checked().1
    }

    fn channel_bounds_math(axis: &str) -> (f32, f32) {
        match axis {
            Self::LIGHTNESS => (0.0, 100.0),
            Self::A | Self::B => (-128.0, 127.0),
            _ => (0.0, 1.0),
        }
    }

    fn clamp_channel_to_gamut_math(base_spec: Lab, axis_name: &str, proposed: f32) -> f32 {
        if axis_name == Self::ALPHA {
            return proposed.clamp(0.0, 1.0);
        }
        let (min, max) = Self::channel_bounds_math(axis_name);
        let proposed = proposed.clamp(min, max);
        let mut proposed_spec = base_spec;
        proposed_spec.set_value(axis_name, proposed);
        if Self::spec_in_gamut(proposed_spec) {
            return proposed;
        }

        let mut anchor = base_spec.get_value(axis_name).clamp(min, max);
        let mut anchor_spec = base_spec;
        anchor_spec.set_value(axis_name, anchor);

        // Fallback for unexpected states where base_spec is already out of gamut.
        if !Self::spec_in_gamut(anchor_spec) {
            let step = (max - min) / Self::CLAMP_SEARCH_STEPS as f32;
            let mut best = None::<(f32, f32)>;
            for i in 0..=Self::CLAMP_SEARCH_STEPS {
                let sample_value = if i == Self::CLAMP_SEARCH_STEPS {
                    max
                } else {
                    min + step * i as f32
                };
                let mut sample_spec = base_spec;
                sample_spec.set_value(axis_name, sample_value);
                if Self::spec_in_gamut(sample_spec) {
                    let dist = (sample_value - proposed).abs();
                    match best {
                        Some((_, best_dist)) if best_dist <= dist => {}
                        _ => best = Some((sample_value, dist)),
                    }
                }
            }
            let Some((best_anchor, _)) = best else {
                return proposed;
            };
            anchor = best_anchor;
        }

        if (proposed - anchor).abs() <= f32::EPSILON {
            return anchor;
        }

        let mut in_gamut = anchor;
        let mut out_of_gamut = proposed;
        for _ in 0..Self::CLAMP_REFINE_STEPS {
            let mid = (in_gamut + out_of_gamut) * 0.5;
            let mut sample = base_spec;
            sample.set_value(axis_name, mid);
            if Self::spec_in_gamut(sample) {
                in_gamut = mid;
            } else {
                out_of_gamut = mid;
            }
        }
        in_gamut.clamp(min, max)
    }

    fn gamut_interval_around(base_spec: Lab, axis_name: &str) -> (f32, f32) {
        if axis_name == Self::ALPHA {
            return (0.0, 1.0);
        }
        let (min, max) = Self::channel_bounds_math(axis_name);
        let mut current = base_spec.get_value(axis_name).clamp(min, max);
        let mut current_spec = base_spec;
        current_spec.set_value(axis_name, current);

        if !Self::spec_in_gamut(current_spec) {
            current = Self::clamp_channel_to_gamut_math(base_spec, axis_name, current);
            current_spec.set_value(axis_name, current);
            if !Self::spec_in_gamut(current_spec) {
                return (min, max);
            }
        }

        let step = (max - min) / Self::CLAMP_SEARCH_STEPS as f32;

        let mut lower_in = current;
        let mut probe = current;
        let mut lower_found = false;
        while probe > min {
            let next = (probe - step).max(min);
            let mut sample = base_spec;
            sample.set_value(axis_name, next);
            if Self::spec_in_gamut(sample) {
                lower_in = next;
                probe = next;
            } else {
                let mut in_v = lower_in;
                let mut out_v = next;
                for _ in 0..Self::CLAMP_REFINE_STEPS {
                    let mid = (in_v + out_v) * 0.5;
                    let mut refined = base_spec;
                    refined.set_value(axis_name, mid);
                    if Self::spec_in_gamut(refined) {
                        in_v = mid;
                    } else {
                        out_v = mid;
                    }
                }
                lower_in = in_v;
                lower_found = true;
                break;
            }
        }
        if !lower_found {
            lower_in = min;
        }

        let mut upper_in = current;
        probe = current;
        let mut upper_found = false;
        while probe < max {
            let next = (probe + step).min(max);
            let mut sample = base_spec;
            sample.set_value(axis_name, next);
            if Self::spec_in_gamut(sample) {
                upper_in = next;
                probe = next;
            } else {
                let mut in_v = upper_in;
                let mut out_v = next;
                for _ in 0..Self::CLAMP_REFINE_STEPS {
                    let mid = (in_v + out_v) * 0.5;
                    let mut refined = base_spec;
                    refined.set_value(axis_name, mid);
                    if Self::spec_in_gamut(refined) {
                        in_v = mid;
                    } else {
                        out_v = mid;
                    }
                }
                upper_in = in_v;
                upper_found = true;
                break;
            }
        }
        if !upper_found {
            upper_in = max;
        }

        (lower_in, upper_in)
    }
}

impl ColorSpecification for Lab {
    fn name(&self) -> &'static str {
        "Lab"
    }

    fn channels(&self) -> &[ColorChannel] {
        &Self::CHANNELS
    }

    fn get_value(&self, channel_name: &str) -> f32 {
        match channel_name {
            Self::LIGHTNESS => self.l,
            Self::A => self.a,
            Self::B => self.b,
            Self::ALPHA => self.alpha,
            _ => 0.0,
        }
    }

    fn set_value(&mut self, channel_name: &str, value: f32) {
        match channel_name {
            Self::LIGHTNESS => self.l = value.clamp(0.0, 100.0),
            Self::A => self.a = value.clamp(-128.0, 127.0),
            Self::B => self.b = value.clamp(-128.0, 127.0),
            Self::ALPHA => self.alpha = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn to_hsla(&self) -> Hsla {
        self.to_hsla_checked().0
    }

    fn from_hsla(hsla: Hsla) -> Self {
        let rgba = hsla.to_rgb();
        let (l, a, b) = rgb_to_lab(rgba);
        Self {
            l,
            a,
            b,
            alpha: rgba.a,
            auto_clamp: false,
            dynamic_range: false,
        }
    }

    fn set_auto_clamp(&mut self, auto_clamp: bool) {
        self.auto_clamp = auto_clamp;
        if auto_clamp {
            self.dynamic_range = false;
        }
    }

    fn set_dynamic_range(&mut self, dynamic_range: bool) {
        self.dynamic_range = dynamic_range;
        if dynamic_range {
            self.auto_clamp = false;
        }
    }

    fn channel_bounds(&self, channel_name: &str) -> (f32, f32) {
        if self.dynamic_range && channel_name != Self::ALPHA {
            Self::gamut_interval_around(*self, channel_name)
        } else {
            Self::channel_bounds_math(channel_name)
        }
    }

    fn clamp_spec_to_gamut(&mut self) {
        if !self.auto_clamp && !self.dynamic_range {
            return;
        }

        for _ in 0..3 {
            if Self::spec_in_gamut(*self) {
                break;
            }
            self.l = Self::clamp_channel_to_gamut_math(*self, Self::LIGHTNESS, self.l);
            self.a = Self::clamp_channel_to_gamut_math(*self, Self::A, self.a);
            self.b = Self::clamp_channel_to_gamut_math(*self, Self::B, self.b);
        }
    }

    fn clamp_channel_to_gamut(&self, channel_name: &str, proposed: f32) -> f32 {
        if self.dynamic_range {
            let mut proposed_spec = *self;
            proposed_spec.set_value(channel_name, proposed);
            if Self::spec_in_gamut(proposed_spec) {
                return proposed;
            }
            return Self::clamp_channel_to_gamut_math(*self, channel_name, proposed);
        }
        if !self.auto_clamp {
            return proposed;
        }
        Self::clamp_channel_to_gamut_math(*self, channel_name, proposed)
    }

    fn is_out_of_gamut(&self) -> bool {
        !Self::spec_in_gamut(*self)
    }
}

fn rgb_to_lab(rgb: Rgba) -> (f32, f32, f32) {
    let r = pivot_rgb(rgb.r);
    let g = pivot_rgb(rgb.g);
    let b = pivot_rgb(rgb.b);

    let x = (r * 0.4124 + g * 0.3576 + b * 0.1805) / 0.95047;
    let y = (r * 0.2126 + g * 0.7152 + b * 0.0722) / 1.00000;
    let z = (r * 0.0193 + g * 0.1192 + b * 0.9505) / 1.08883;

    let x = pivot_xyz(x);
    let y = pivot_xyz(y);
    let z = pivot_xyz(z);

    let l = (116.0 * y) - 16.0;
    let a = 500.0 * (x - y);
    let b = 200.0 * (y - z);

    (l, a, b)
}

fn lab_to_rgb(l: f32, a: f32, b: f32, alpha: f32) -> Rgba {
    lab_to_rgb_checked(l, a, b, alpha).0
}

fn lab_to_rgb_checked(l: f32, a: f32, b: f32, alpha: f32) -> (Rgba, bool) {
    let mut y = (l + 16.0) / 116.0;
    let mut x = a / 500.0 + y;
    let mut z = y - b / 200.0;

    let x3 = x * x * x;
    let z3 = z * z * z;

    x = if x3 > 0.008856 {
        x3
    } else {
        (x - 16.0 / 116.0) / 7.787
    };
    y = if l > 8.0 {
        ((l + 16.0) / 116.0).powi(3)
    } else {
        l / 903.3
    };
    z = if z3 > 0.008856 {
        z3
    } else {
        (z - 16.0 / 116.0) / 7.787
    };

    // Multiply by white point (D65)
    let x = x * 0.95047;
    let y = y * 1.00000;
    let z = z * 1.08883;

    let r = x * 3.2406 + y * -1.5372 + z * -0.4986;
    let g = x * -0.9689 + y * 1.8758 + z * 0.0415;
    let b = x * 0.0557 + y * -0.2040 + z * 1.0570;

    let srgb_r = rev_pivot_rgb(r);
    let srgb_g = rev_pivot_rgb(g);
    let srgb_b = rev_pivot_rgb(b);

    const GAMUT_EPSILON: f32 = 1e-4;
    let out_of_gamut = !srgb_r.is_finite()
        || !srgb_g.is_finite()
        || !srgb_b.is_finite()
        || srgb_r < -GAMUT_EPSILON
        || srgb_r > 1.0 + GAMUT_EPSILON
        || srgb_g < -GAMUT_EPSILON
        || srgb_g > 1.0 + GAMUT_EPSILON
        || srgb_b < -GAMUT_EPSILON
        || srgb_b > 1.0 + GAMUT_EPSILON;

    let clamp_channel = |channel: f32| {
        if channel.is_finite() {
            channel.clamp(0.0, 1.0)
        } else {
            0.0
        }
    };

    (
        Rgba {
            r: clamp_channel(srgb_r),
            g: clamp_channel(srgb_g),
            b: clamp_channel(srgb_b),
            a: alpha,
        },
        out_of_gamut,
    )
}

fn pivot_rgb(n: f32) -> f32 {
    if n > 0.04045 {
        ((n + 0.055) / 1.055).powf(2.4)
    } else {
        n / 12.92
    }
}

fn rev_pivot_rgb(n: f32) -> f32 {
    if n > 0.0031308 {
        1.055 * n.powf(1.0 / 2.4) - 0.055
    } else {
        12.92 * n
    }
}

fn pivot_xyz(n: f32) -> f32 {
    if n > 0.008856 {
        n.powf(1.0 / 3.0)
    } else {
        (7.787 * n) + (16.0 / 116.0)
    }
}
