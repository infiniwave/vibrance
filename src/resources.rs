use std::borrow::Cow;

use anyhow::{Result, anyhow};
use gpui::{AssetSource, SharedString};
use gpui_component_assets::Assets;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "resources"]
pub struct Resources;

impl AssetSource for Resources {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }
        if path.starts_with("icons") {
            // belongs to gpui component
            return Assets.load(path);
        }
        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .chain(Assets.list(path).unwrap_or_default())
            .collect())
    }
}
