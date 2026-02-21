use gpui::{hsla, Hsla};

// Pure palette-generation logic used by composition stories.
// Keep UI-specific rendering concerns in `color_combinations.rs`.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorCombination {
    #[default]
    Monochromatic,
    Complementary,
    Analogous,
    Triadic,
    Tetradic,
    Pentadic,
    Hexadic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CombinationSwatch {
    pub role: &'static str,
    pub formula: &'static str,
    pub color: Hsla,
}

impl ColorCombination {
    pub fn from_label(label: &str) -> Self {
        match label {
            "Monochromatic" => Self::Monochromatic,
            "Complementary" => Self::Complementary,
            "Analogous" => Self::Analogous,
            "Triadic" => Self::Triadic,
            "Tetradic" => Self::Tetradic,
            "Pentadic" => Self::Pentadic,
            "Hexadic" => Self::Hexadic,
            _ => Self::Monochromatic,
        }
    }

    pub fn palette(self, base: Hsla) -> Vec<CombinationSwatch> {
        match self {
            Self::Monochromatic => monochromatic_palette(base),
            Self::Complementary => complementary_palette(base),
            Self::Analogous => analogous_palette(base),
            Self::Triadic => triadic_palette(base),
            Self::Tetradic => tetradic_palette(base),
            Self::Pentadic => pentadic_palette(base),
            Self::Hexadic => hexadic_palette(base),
        }
    }
}

fn monochromatic_palette(base: Hsla) -> Vec<CombinationSwatch> {
    vec![
        CombinationSwatch {
            role: "Base",
            formula: "H,S,L",
            color: base,
        },
        CombinationSwatch {
            role: "Tint",
            formula: "H,S*0.78,L+0.14",
            color: hsla(
                base.h,
                (base.s * 0.78).clamp(0.0, 1.0),
                (base.l + 0.14).clamp(0.0, 1.0),
                base.a,
            ),
        },
    ]
}

fn complementary_palette(base: Hsla) -> Vec<CombinationSwatch> {
    vec![
        CombinationSwatch {
            role: "Base",
            formula: "H,S,L",
            color: base,
        },
        CombinationSwatch {
            role: "Complement",
            formula: "H+180deg,S,L",
            color: rotate_hue(base, 180.0),
        },
    ]
}

fn analogous_palette(base: Hsla) -> Vec<CombinationSwatch> {
    vec![
        CombinationSwatch {
            role: "Left",
            formula: "H-30deg,S,L",
            color: rotate_hue(base, -30.0),
        },
        CombinationSwatch {
            role: "Base",
            formula: "H,S,L",
            color: base,
        },
        CombinationSwatch {
            role: "Right",
            formula: "H+30deg,S,L",
            color: rotate_hue(base, 30.0),
        },
    ]
}

fn triadic_palette(base: Hsla) -> Vec<CombinationSwatch> {
    vec![
        CombinationSwatch {
            role: "Base",
            formula: "H,S,L",
            color: base,
        },
        CombinationSwatch {
            role: "Triad B",
            formula: "H+120deg,S,L",
            color: rotate_hue(base, 120.0),
        },
        CombinationSwatch {
            role: "Triad C",
            formula: "H+240deg,S,L",
            color: rotate_hue(base, 240.0),
        },
    ]
}

fn tetradic_palette(base: Hsla) -> Vec<CombinationSwatch> {
    vec![
        CombinationSwatch {
            role: "Base",
            formula: "H,S,L",
            color: base,
        },
        CombinationSwatch {
            role: "Tetrad B",
            formula: "H+90deg,S,L",
            color: rotate_hue(base, 90.0),
        },
        CombinationSwatch {
            role: "Tetrad C",
            formula: "H+180deg,S,L",
            color: rotate_hue(base, 180.0),
        },
        CombinationSwatch {
            role: "Tetrad D",
            formula: "H+270deg,S,L",
            color: rotate_hue(base, 270.0),
        },
    ]
}

fn pentadic_palette(base: Hsla) -> Vec<CombinationSwatch> {
    vec![
        CombinationSwatch {
            role: "Base",
            formula: "H,S,L",
            color: base,
        },
        CombinationSwatch {
            role: "Pentad B",
            formula: "H+72deg,S,L",
            color: rotate_hue(base, 72.0),
        },
        CombinationSwatch {
            role: "Pentad C",
            formula: "H+144deg,S,L",
            color: rotate_hue(base, 144.0),
        },
        CombinationSwatch {
            role: "Pentad D",
            formula: "H+216deg,S,L",
            color: rotate_hue(base, 216.0),
        },
        CombinationSwatch {
            role: "Pentad E",
            formula: "H+288deg,S,L",
            color: rotate_hue(base, 288.0),
        },
    ]
}

fn hexadic_palette(base: Hsla) -> Vec<CombinationSwatch> {
    vec![
        CombinationSwatch {
            role: "Base",
            formula: "H,S,L",
            color: base,
        },
        CombinationSwatch {
            role: "Hexad B",
            formula: "H+60deg,S,L",
            color: rotate_hue(base, 60.0),
        },
        CombinationSwatch {
            role: "Hexad C",
            formula: "H+120deg,S,L",
            color: rotate_hue(base, 120.0),
        },
        CombinationSwatch {
            role: "Hexad D",
            formula: "H+180deg,S,L",
            color: rotate_hue(base, 180.0),
        },
        CombinationSwatch {
            role: "Hexad E",
            formula: "H+240deg,S,L",
            color: rotate_hue(base, 240.0),
        },
        CombinationSwatch {
            role: "Hexad F",
            formula: "H+300deg,S,L",
            color: rotate_hue(base, 300.0),
        },
    ]
}

fn rotate_hue(base: Hsla, delta_degrees: f32) -> Hsla {
    hsla(
        (base.h + delta_degrees / 360.0).rem_euclid(1.0),
        base.s,
        base.l,
        base.a,
    )
}
