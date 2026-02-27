use anyhow::anyhow;
use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

/// Embed application assets for GPUI Component.
///
/// This assets provides icons svg files for [IconName](https://docs.rs/gpui-component/latest/gpui_component/enum.IconName.html).
///
/// ## Usage
///
/// ```rust,no_run
/// use gpui::*;
/// use gpui_component_assets::Assets;
///
/// let app = gpui_platform::application().with_assets(Assets);
/// ```
///
/// ## Platform Differences
///
/// - **Native (Desktop)**: Icons are embedded in the binary using RustEmbed
/// - **WASM (Web)**: Icons are loaded from https://lucide.dev/ CDN at runtime
///   - This significantly reduces WASM bundle size
///   - Icons are loaded on-demand when first used
///   - The Icon component handles remote loading automatically
#[cfg(not(target_arch = "wasm32"))]
#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "icons/**/*.svg"]
pub struct Assets;

#[cfg(target_arch = "wasm32")]
pub struct Assets;

#[cfg(not(target_arch = "wasm32"))]
impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}

#[cfg(target_arch = "wasm32")]
impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // For WASM, return a placeholder SVG that references the remote icon from CDN
        // This keeps the WASM bundle small while still displaying icons correctly
        if path.is_empty() {
            return Ok(None);
        }

        // Generate a placeholder SVG that loads the icon from CDN
        if path.starts_with("icons/") && path.ends_with(".svg") {
            // Extract icon name from path: icons/lucide/alert-circle.svg -> alert-circle
            let icon_name = path
                .trim_start_matches("icons/lucide/")
                .trim_end_matches(".svg");

            // Use jsdelivr CDN for Lucide icons (faster and more reliable than lucide.dev)
            let remote_url = format!("https://cdn.jsdelivr.net/npm/lucide-static@latest/icons/{}.svg", icon_name);

            // Create a placeholder SVG that references the remote icon
            // The browser will fetch the actual icon content automatically
            let placeholder_svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><image href="{}" width="24" height="24"/></svg>"#,
                remote_url
            );

            Ok(Some(Cow::Owned(placeholder_svg.into_bytes())))
        } else {
            Ok(None)
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        // For WASM, we don't have embedded assets to list
        // Return empty list as icons will be loaded remotely from CDN
        let _ = path;
        Ok(Vec::new())
    }
}
