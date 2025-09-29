use std::{borrow::Cow, path::PathBuf};

use anyhow::{anyhow, Result};

use gpui::AssetSource;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../assets"]
#[include = "icons/**/*"]
#[exclude = "*.DS_Store"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .or_else(|| {
                let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                let path = manifest_dir.join(path);

                std::fs::read(path)
                    .map(|data| Some(std::borrow::Cow::Owned(data)))
                    .ok()
            })
            .ok_or_else(|| anyhow!("could not find asset at path \"{}\"", path))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| {
                if p.starts_with(path) {
                    Some(p.into())
                } else {
                    None
                }
            })
            .collect())
    }
}
