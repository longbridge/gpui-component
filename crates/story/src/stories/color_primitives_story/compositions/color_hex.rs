use gpui::{Hsla, Rgba, SharedString};

pub fn parse_rgb_hex(input: &str) -> Result<Hsla, &'static str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter #RRGGBB");
    }

    let Some(value) = trimmed.strip_prefix('#') else {
        return Err("Color must start with #");
    };

    if value.len() != 6 {
        return Err("Expected 6 hex digits (#RRGGBB)");
    }

    let parse = |range: std::ops::Range<usize>| -> Result<u8, &'static str> {
        u8::from_str_radix(&value[range], 16).map_err(|_| "Invalid hex channel")
    };
    let red = parse(0..2)?;
    let green = parse(2..4)?;
    let blue = parse(4..6)?;

    Ok(Rgba {
        r: red as f32 / 255.0,
        g: green as f32 / 255.0,
        b: blue as f32 / 255.0,
        a: 1.0,
    }
    .into())
}

pub fn format_rgb_hex(color: Hsla) -> SharedString {
    let rgb = color.to_rgb();
    format!(
        "#{:02X}{:02X}{:02X}",
        (rgb.r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgb.g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgb.b.clamp(0.0, 1.0) * 255.0).round() as u8
    )
    .into()
}
