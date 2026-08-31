//! JavaScript access to gpui-base's current semantic theme.
use crate::{theme_tokens, value::Bridged};
use gpui_base::{Theme, ThemeAppearance};
use rquickjs::{Ctx, Exception, Object, Result as JsResult, Value, function::Func};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct ScriptTheme {
    appearance: ThemeAppearance,
    tokens: ScriptTokens,
}
#[derive(Deserialize)]
struct ScriptTokens {
    colors: HashMap<String, String>,
    spacing: HashMap<String, f32>,
    radius: HashMap<String, f32>,
}

pub fn install(ctx: &Ctx<'_>, module: &Object<'_>) -> JsResult<()> {
    ctx.globals().set(
        "__theme_snapshot",
        Func::from(|_: Ctx<'_>| -> JsResult<String> { Ok(snapshot_json()) }),
    )?;
    module.set("set_theme", Func::from(set_theme))?;
    Ok(())
}

fn set_theme<'js>(ctx: Ctx<'js>, value: Value<'js>) -> JsResult<()> {
    if matches!(
        crate::scope::current_phase(),
        Some(crate::scope::ScopePhase::Render) | Some(crate::scope::ScopePhase::Layout)
    ) {
        return Err(Exception::throw_type(
            &ctx,
            "set_theme(theme) cannot run during render or layout; switch themes from an event handler or task",
        ));
    }
    let source = ctx
        .json_stringify(value)?
        .ok_or_else(|| Exception::throw_type(&ctx, "set_theme(theme) expects an object"))?;
    let source = source.to_string()?;
    let supplied: ScriptTheme = serde_json::from_str(&source).map_err(|error| {
        Exception::throw_type(&ctx, &format!("invalid theme snapshot: {error}"))
    })?;
    crate::scope::with_current_app(|cx| {
        let mut tokens = Theme::global(cx).tokens;
        apply_colors(&mut tokens.colors, &supplied.tokens.colors)
            .map_err(|e| Exception::throw_type(&ctx, &e))?;
        apply_scale(
            theme_tokens::spacing_token_names(),
            &supplied.tokens.spacing,
            |name, value| set_spacing(&mut tokens.spacing, name, value),
        )
        .map_err(|e| Exception::throw_type(&ctx, &e))?;
        apply_scale(
            theme_tokens::radius_token_names(),
            &supplied.tokens.radius,
            |name, value| set_radius(&mut tokens.radius, name, value),
        )
        .map_err(|e| Exception::throw_type(&ctx, &e))?;
        let base = Theme::global_mut(cx);
        base.appearance = supplied.appearance;
        base.tokens = tokens;
        theme_tokens::sync(cx);
        cx.refresh_windows();
        Ok(())
    })
    .ok_or_else(|| {
        Exception::throw_type(
            &ctx,
            "set_theme(theme) needs a live host call; call it from an event handler",
        )
    })?
}

fn apply_colors(
    colors: &mut gpui_base::ColorTokens,
    supplied: &HashMap<String, String>,
) -> Result<(), String> {
    for name in theme_tokens::color_token_names() {
        let source = supplied
            .get(*name)
            .ok_or_else(|| format!("theme tokens.colors is missing `{name}`"))?;
        let value = Bridged::Str(source.clone())
            .as_color()
            .map_err(|e| format!("theme color `{name}`: {e}"))?;
        match *name {
            "background" => colors.background = value,
            "foreground" => colors.foreground = value,
            "surface" => colors.surface = value,
            "surface_foreground" => colors.surface_foreground = value,
            "primary" => colors.primary = value,
            "primary_foreground" => colors.primary_foreground = value,
            "secondary" => colors.secondary = value,
            "secondary_foreground" => colors.secondary_foreground = value,
            "muted" => colors.muted = value,
            "muted_foreground" => colors.muted_foreground = value,
            "accent" => colors.accent = value,
            "accent_foreground" => colors.accent_foreground = value,
            "destructive" => colors.destructive = value,
            "destructive_foreground" => colors.destructive_foreground = value,
            "border" => colors.border = value,
            "input" => colors.input = value,
            "ring" => colors.ring = value,
            "selection" => colors.selection = value,
            _ => unreachable!(),
        }
    }
    Ok(())
}

fn apply_scale(
    names: &[&str],
    values: &HashMap<String, f32>,
    mut set: impl FnMut(&str, f32),
) -> Result<(), String> {
    for name in names {
        let value = *values
            .get(*name)
            .ok_or_else(|| format!("theme token scale is missing `{name}`"))?;
        if !value.is_finite() || value < 0. {
            return Err(format!(
                "theme token `{name}` must be finite and non-negative"
            ));
        }
        set(name, value);
    }
    Ok(())
}
fn set_spacing(t: &mut gpui_base::SpacingTokens, n: &str, v: f32) {
    let v = gpui::px(v);
    match n {
        "xxs" => t.xxs = v,
        "xs" => t.xs = v,
        "sm" => t.sm = v,
        "md" => t.md = v,
        "lg" => t.lg = v,
        "xl" => t.xl = v,
        "xxl" => t.xxl = v,
        _ => unreachable!(),
    }
}
fn set_radius(t: &mut gpui_base::RadiusTokens, n: &str, v: f32) {
    let v = gpui::px(v);
    match n {
        "none" => t.none = v,
        "sm" => t.sm = v,
        "md" => t.md = v,
        "lg" => t.lg = v,
        "xl" => t.xl = v,
        "full" => t.full = v,
        _ => unreachable!(),
    }
}

fn snapshot_json() -> String {
    let join = |v: Vec<String>| v.join(",");
    let pairs = theme_tokens::color_token_names()
        .iter()
        .filter_map(|n| theme_tokens::token_color(n).map(|c| format!("\"{n}\":\"{}\"", hex(c))))
        .collect::<Vec<_>>();
    let colors = join(pairs.clone());
    let direct = join(pairs);
    let spacing = join(
        theme_tokens::spacing_token_names()
            .iter()
            .filter_map(|n| {
                theme_tokens::token_spacing(n).map(|v| format!("\"{n}\":{}", f32::from(v)))
            })
            .collect(),
    );
    let radius = join(
        theme_tokens::radius_token_names()
            .iter()
            .filter_map(|n| {
                theme_tokens::token_radius(n).map(|v| format!("\"{n}\":{}", f32::from(v)))
            })
            .collect(),
    );
    let appearance =
        crate::scope::with_current_app(|cx| Theme::global(cx).appearance).unwrap_or_default();
    let name = match appearance {
        ThemeAppearance::Light => "light",
        ThemeAppearance::Dark => "dark",
    };
    format!(
        "{{{direct},\"colors\":{{{colors}}},\"spacing\":{{{spacing}}},\"radius\":{{{radius}}},\"appearance\":\"{name}\",\"is_dark\":{}}}",
        appearance == ThemeAppearance::Dark
    )
}
fn hex(color: gpui::Hsla) -> String {
    let c = gpui::Rgba::from(color);
    let b = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(c.r), b(c.g), b(c.b))
}
