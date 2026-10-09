use gpui_kit::{AssetSource, SharedString};
use std::borrow::Cow;

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        if path == "branding/glimpse.png" {
            return Ok(Some(Cow::Borrowed(include_bytes!(
                "../../../../assets/branding/glimpse.png"
            ))));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        let mut assets = gpui_kit::assets::Assets.list(path)?;
        if "branding/glimpse.png".starts_with(path) {
            assets.push("branding/glimpse.png".into());
        }
        Ok(assets)
    }
}
