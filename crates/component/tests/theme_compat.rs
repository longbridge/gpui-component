use gpui_component::theme::{ThemeConfig, ThemeConfigColors, ThemeMode};

#[test]
fn legacy_theme_config_struct_literal_shape_is_unchanged() {
    let _ = ThemeConfig {
        is_default: false,
        name: "Compatibility".into(),
        mode: ThemeMode::Light,
        font_size: None,
        font_family: None,
        mono_font_family: None,
        mono_font_size: None,
        radius: None,
        radius_lg: None,
        shadow: None,
        colors: ThemeConfigColors::default(),
        highlight: None,
    };
}
