//! The theme surface.
//!
//! A script reads semantic tokens rather than colors: `theme().colors.surface`
//! is a role, and a host that ships a different palette changes what it means
//! without the script changing. Writing the theme is not offered — a script
//! that could repaint the whole host would make the host's own appearance
//! something every application it loads gets a vote on.

use rquickjs::{Ctx, Exception, Object, Result as JsResult, function::Func};

use crate::theme::{self, ThemeMode};

/// Installs `theme` and `set_theme` onto the module object.
pub fn install(ctx: &Ctx<'_>, module: &Object<'_>) -> JsResult<()> {
    let _ = ctx;

    // Built by the prelude from three scalar lookups: a closure cannot return a
    // borrowed `Object<'js>`, the same constraint the element bindings hit.
    ctx.globals().set(
        "__theme_snapshot",
        Func::from(|ctx: Ctx<'_>| -> JsResult<String> {
            let _ = &ctx;
            Ok(snapshot_json())
        }),
    )?;

    module.set(
        "set_theme",
        Func::from(|ctx: Ctx<'_>, name: String| -> JsResult<bool> {
            let mode = ThemeMode::from_name(&name).ok_or_else(|| {
                Exception::throw_type(
                    &ctx,
                    &format!("unknown theme `{name}`; expected `light` or `dark`"),
                )
            })?;

            crate::scope::with_current_app(|cx| theme::set_mode(mode, cx)).ok_or_else(|| {
                Exception::throw_type(
                    &ctx,
                    "set_theme(...) needs a live host call; call it from an event handler",
                )
            })
        }),
    )?;

    Ok(())
}

/// The installed palette as JSON.
///
/// Read fresh on each call rather than cached: a theme switch must be visible
/// to the next render, and this is not a hot path — a view reads a handful of
/// tokens, not thousands.
fn snapshot_json() -> String {
    let quote = |pairs: Vec<String>| pairs.join(",");

    let color_pairs = theme::color_token_names()
        .iter()
        .filter_map(|name| {
            theme::token_color(name).map(|color| format!("\"{name}\":\"{}\"", hex(color)))
        })
        .collect::<Vec<_>>();
    let colors = quote(color_pairs.clone());
    let direct_colors = quote(color_pairs);
    let spacing = quote(
        theme::spacing_token_names()
            .iter()
            .filter_map(|name| {
                theme::token_spacing(name).map(|value| format!("\"{name}\":{}", f32::from(value)))
            })
            .collect(),
    );
    let radius = quote(
        theme::radius_token_names()
            .iter()
            .filter_map(|name| {
                theme::token_radius(name).map(|value| format!("\"{name}\":{}", f32::from(value)))
            })
            .collect(),
    );

    let mode = crate::scope::with_current_app(|cx| theme::mode(cx)).unwrap_or(ThemeMode::Light);

    format!(
        "{{{direct_colors},\"colors\":{{{colors}}},\"spacing\":{{{spacing}}},\"radius\":{{{radius}}},\
         \"mode\":\"{}\",\"is_dark\":{}}}",
        mode.as_str(),
        mode.is_dark()
    )
}

/// Tokens reach the script as `#rrggbb`, the one color spelling the style layer
/// already accepts, so a value read from the theme can be handed straight back
/// to `bg(...)`.
fn hex(color: gpui::Hsla) -> String {
    let rgba = gpui::Rgba::from(color);
    let channel = |value: f32| (value.clamp(0., 1.) * 255.).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(rgba.r),
        channel(rgba.g),
        channel(rgba.b)
    )
}
