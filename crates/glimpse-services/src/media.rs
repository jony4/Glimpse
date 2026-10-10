//! Bounded image decoding, kept off the UI executor.
use anyhow::{Context, Result, ensure};
use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

pub struct ImageDocument {
    pub path: PathBuf,
    pub png: Vec<u8>,
    pub fingerprint: String,
}
pub fn supports(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "webp"
            | "gif"
            | "bmp"
            | "tif"
            | "tiff"
            | "ico"
            | "icns"
            | "svg"
            | "exr"
            | "hdr"
    )
}
pub fn read_image(path: &Path) -> Result<ImageDocument> {
    use std::hash::{Hash, Hasher};
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 32 * 1024 * 1024,
        "Image exceeds 32 MiB limit"
    );
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hash);
    let fingerprint = format!("image:{:x}", hash.finish());
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let png = if ext == "svg" {
        // No resources_dir: external file references are intentionally not resolved.
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default())?;
        // Vector documents can describe enormous canvases. Rasterize a bounded
        // preview instead of rejecting them or allocating their full native size.
        let size = tree.size();
        let scale = (4000.0 / size.width().max(size.height())).min(1.0);
        let width = (size.width() * scale).ceil().clamp(1.0, 4000.0) as u32;
        let height = (size.height() * scale).ceil().clamp(1.0, 4000.0) as u32;
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(width, height).context("Invalid SVG dimensions")?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        pixmap.encode_png()?
    } else {
        let data = if ext == "icns" {
            icns_png(&bytes)?
        } else {
            &bytes
        };
        let mut reader = image::ImageReader::new(Cursor::new(data)).with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        let decoded = reader.decode()?;
        ensure!(
            u64::from(decoded.width()) * u64::from(decoded.height()) <= 16_000_000,
            "Image exceeds 16 megapixels"
        );
        let mut out = Cursor::new(Vec::new());
        // Float EXR/HDR pixels must be converted before PNG encoding. This is an
        // SDR preview, not a color-managed HDR mastering surface.
        let preview = if matches!(
            decoded,
            image::DynamicImage::ImageRgb32F(_) | image::DynamicImage::ImageRgba32F(_)
        ) {
            image::DynamicImage::ImageRgba8(decoded.to_rgba8())
        } else {
            decoded
        };
        preview.write_to(&mut out, image::ImageFormat::Png)?;
        out.into_inner()
    };
    Ok(ImageDocument {
        path: std::path::absolute(path)?,
        png,
        fingerprint,
    })
}
// Modern macOS ICNS contains PNG representations; choose the largest representation.
fn icns_png(bytes: &[u8]) -> Result<&[u8]> {
    ensure!(
        bytes.starts_with(b"icns") && bytes.len() >= 8,
        "Invalid ICNS file"
    );
    let mut offset = 8;
    let mut best: Option<&[u8]> = None;
    while offset + 8 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[offset + 4..offset + 8].try_into()?) as usize;
        ensure!(
            len >= 8 && len <= bytes.len() - offset,
            "Invalid ICNS entry"
        );
        let data = &bytes[offset + 8..offset + len];
        if data.starts_with(b"\x89PNG\r\n\x1a\n") && best.is_none_or(|b| data.len() > b.len()) {
            best = Some(data);
        }
        offset += len;
    }
    best.context("This legacy ICNS has no PNG representation")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn float_images_produce_displayable_png_previews() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let pixels = image::Rgb32FImage::from_pixel(2, 2, image::Rgb([0.25, 0.5, 1.0]));
        for (extension, format) in [
            ("exr", image::ImageFormat::OpenExr),
            ("hdr", image::ImageFormat::Hdr),
        ] {
            let path = dir.path().join(format!("preview.{extension}"));
            image::DynamicImage::ImageRgb32F(pixels.clone()).save_with_format(&path, format)?;
            assert!(supports(&path));
            let preview = read_image(&path)?;
            let decoded = image::load_from_memory(&preview.png)?;
            assert_eq!((decoded.width(), decoded.height()), (2, 2));
            assert_eq!(decoded.to_rgb8().get_pixel(0, 0).0, [64, 128, 255]);
            std::fs::write(&path, b"invalid image")?;
            assert!(read_image(&path).is_err());
        }
        Ok(())
    }

    #[test]
    fn opens_project_assets_and_rejects_malformed_images() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for p in [
            "assets/branding/glimpse.png",
            "assets/branding/glimpse.svg",
            "assets/macos/Glimpse.icns",
        ] {
            assert!(read_image(&root.join(p))?.png.starts_with(b"\x89PNG"));
        }
        assert!(icns_png(b"icns\0\0\0\x10bad!\0\0\0\0").is_err());
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("large.svg");
        std::fs::write(
            &path,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100000" height="50000"><rect width="100000" height="50000" fill="red"/></svg>"#,
        )?;
        let preview = image::load_from_memory(&read_image(&path)?.png)?;
        assert_eq!((preview.width(), preview.height()), (4000, 2000));
        assert_eq!(preview.to_rgb8().get_pixel(2000, 1000).0, [255, 0, 0]);
        Ok(())
    }
}
