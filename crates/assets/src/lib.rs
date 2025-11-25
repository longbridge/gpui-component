use anyhow::anyhow;
use gpui::{AssetSource, Result, SharedString};
use rust_embed::RustEmbed;
use std::borrow::Cow;

/// Embed application assets for GPUI Component.
///
/// This assets provides icons svg files for [IconName](https://docs.rs/gpui-component/latest/gpui_component/enum.IconName.html).
///
/// ```
/// use gpui::*;
/// use gpui_component_assets::Assets;
///
/// let app = Application::new().with_assets(Assets);
/// ```
#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/**/*.svg"]
pub struct Assets;

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
