use gpui_kit::{AssetSource, SharedString};
use std::borrow::Cow;

gpui_kit::assets::icon_assets!(ViewerIcons, [Files, GitBranch]);

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        match path {
            "navigation/files.svg" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/navigation/files.svg"
                ))));
            }
            "navigation/git.svg" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/navigation/git.svg"
                ))));
            }
            "file-icons/audio.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/audio.png"
                ))));
            }
            "file-icons/c.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/c.png"
                ))));
            }
            "file-icons/cpp.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/cpp.png"
                ))));
            }
            "file-icons/css.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/css.png"
                ))));
            }
            "file-icons/database.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/database.png"
                ))));
            }
            "file-icons/docker.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/docker.png"
                ))));
            }
            "file-icons/document.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/document.png"
                ))));
            }
            "file-icons/git.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/git.png"
                ))));
            }
            "file-icons/go.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/go.png"
                ))));
            }
            "file-icons/html.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/html.png"
                ))));
            }
            "file-icons/image.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/image.png"
                ))));
            }
            "file-icons/java.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/java.png"
                ))));
            }
            "file-icons/javascript.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/javascript.png"
                ))));
            }
            "file-icons/json.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/json.png"
                ))));
            }
            "file-icons/kotlin.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/kotlin.png"
                ))));
            }
            "file-icons/lock.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/lock.png"
                ))));
            }
            "file-icons/markdown.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/markdown.png"
                ))));
            }
            "file-icons/pdf.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/pdf.png"
                ))));
            }
            "file-icons/php.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/php.png"
                ))));
            }
            "file-icons/python.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/python.png"
                ))));
            }
            "file-icons/react.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/react.png"
                ))));
            }
            "file-icons/ruby.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/ruby.png"
                ))));
            }
            "file-icons/rust.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/rust.png"
                ))));
            }
            "file-icons/settings.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/settings.png"
                ))));
            }
            "file-icons/svelte.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/svelte.png"
                ))));
            }
            "file-icons/swift.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/swift.png"
                ))));
            }
            "file-icons/toml.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/toml.png"
                ))));
            }
            "file-icons/typescript.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/typescript.png"
                ))));
            }
            "file-icons/video.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/video.png"
                ))));
            }
            "file-icons/vue.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/vue.png"
                ))));
            }
            "file-icons/xml.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/xml.png"
                ))));
            }
            "file-icons/yaml.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/yaml.png"
                ))));
            }
            "file-icons/zip.png" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../../../assets/file-icons/zip.png"
                ))));
            }
            _ => {}
        }
        if path == "branding/glimpse.png" {
            return Ok(Some(Cow::Borrowed(include_bytes!(
                "../../../../assets/branding/glimpse.png"
            ))));
        }
        if matches!(path, "icons/files.svg" | "icons/git-branch.svg") {
            return ViewerIcons.load(path);
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        let mut assets = gpui_kit::assets::Assets.list(path)?;
        assets.extend(ViewerIcons.list(path)?);
        if "branding/glimpse.png".starts_with(path) {
            assets.push("branding/glimpse.png".into());
        }
        Ok(assets)
    }
}
