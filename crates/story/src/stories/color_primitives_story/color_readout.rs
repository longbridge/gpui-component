use super::color_spec::{ColorSpecification, Lab};
use gpui::{
    black, div, px, red, white, Hsla, IntoElement, ParentElement as _, SharedString, Styled as _,
};
use gpui_component::{h_flex, v_flex, Colorize as _};

#[derive(Clone, Copy, Debug)]
pub enum PublishedColorSpec {
    /// Optional marker for readouts driven directly from RGB(A) output.
    Rgb,
    /// Publishes the true Lab input values (which may be out of gamut).
    Lab(Lab),
}

pub fn render_color_readout(
    color: Hsla,
    mono_font_family: SharedString,
    include_alpha_specs: bool,
) -> impl IntoElement {
    render_color_readout_with_spec(color, mono_font_family, include_alpha_specs, None)
}

pub fn render_color_readout_with_spec(
    color: Hsla,
    mono_font_family: SharedString,
    include_alpha_specs: bool,
    published_spec: Option<PublishedColorSpec>,
) -> impl IntoElement {
    let round2 = |value: f32| (value * 100.0).round() / 100.0;

    let rgb = color.to_rgb();
    let r = (rgb.r * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = (rgb.g * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = (rgb.b * 255.0).round().clamp(0.0, 255.0) as u8;
    let a = (rgb.a * 255.0).round().clamp(0.0, 255.0) as u8;

    let h = (color.h * 360.0).round();
    let s_pct = (color.s * 100.0).round();
    let l_pct = (color.l * 100.0).round();
    let alpha = round2(color.a);
    let lab = match published_spec {
        Some(PublishedColorSpec::Lab(lab)) => lab,
        _ => Lab::from_hsla(color),
    };
    let lab_out_of_gamut = lab.to_hsla_checked().1;

    let mut rows = vec![
        ("HEX", color.to_hex().to_uppercase(), false),
        ("RGB", format!("rgb({r}, {g}, {b})"), false),
        ("HSL", format!("hsl({h:.0}, {s_pct:.0}%, {l_pct:.0}%)"), false),
        (
            "LAB",
            format!(
                "lab({:.1}, {:+.1}, {:+.1})",
                round2(lab.l),
                round2(lab.a),
                round2(lab.b)
            ),
            lab_out_of_gamut,
        ),
    ];

    if include_alpha_specs {
        rows.insert(1, ("HEXA", format!("#{r:02X}{g:02X}{b:02X}{a:02X}"), false));
        rows.insert(3, ("RGBA", format!("rgba({r}, {g}, {b}, {alpha})"), false));
        rows.insert(
            5,
            (
                "HSLA",
                format!("hsla({h:.0}, {s_pct:.0}%, {l_pct:.0}%, {alpha})"),
                false,
            ),
        );
        rows.push((
            "LABA",
            format!(
                "laba({:.1}, {:+.1}, {:+.1}, {alpha:.2})",
                round2(lab.l),
                round2(lab.a),
                round2(lab.b)
            ),
            lab_out_of_gamut,
        ));
    }

    v_flex()
        .w(px(250.0))
        .p_3()
        .gap_1()
        .bg(black().opacity(0.82))
        .border_1()
        .border_color(white().opacity(0.15))
        .rounded_lg()
        .font_family(mono_font_family)
        .children(rows.into_iter().map(|(name, value, out_of_gamut)| {
            h_flex()
                .w_full()
                .justify_between()
                .items_center()
                .gap_4()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(white().opacity(0.82))
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(if out_of_gamut {
                            red()
                        } else {
                            white().opacity(0.98)
                        })
                        .child(value),
                )
                .into_any_element()
        }))
}
