//! Read-only, opt-in local smoke check. Prints formats/counts, never paths or contents.
//! cargo run -p glimpse-services --example audit_formats -- /path/to/project
use std::{collections::BTreeMap, path::Path};

fn main() -> anyhow::Result<()> {
    let root = std::env::args_os()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Pass a project directory"))?;
    anyhow::ensure!(Path::new(&root).is_dir(), "Expected a directory");
    let mut samples = BTreeMap::<String, Vec<std::path::PathBuf>>::new();
    // Only explicitly supported source/image formats; never probe credentials or
    // unknown binary payloads. Hidden directories and dependency/build trees skipped.
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(false)
        .git_exclude(false)
        .git_global(false)
        .filter_entry(|entry| {
            !matches!(
                entry.file_name().to_str(),
                Some(
                    "node_modules"
                        | "target"
                        | "vendor"
                        | "Pods"
                        | "venv"
                        | "__pycache__"
                        | "dist"
                        | "build"
                )
            )
        })
        .build();
    for entry in walker {
        let entry = entry?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(
            ext.as_str(),
            "java"
                | "sql"
                | "vue"
                | "xml"
                | "mtlx"
                | "tmx"
                | "glsl"
                | "metal"
                | "cu"
                | "pyx"
                | "pxd"
                | "jsonl"
                | "csv"
                | "rst"
                | "exr"
                | "hdr"
                | "png"
                | "svg"
                | "webp"
        ) {
            continue;
        }
        let selected = samples.entry(ext).or_default();
        if selected.len() < 5 {
            selected.push(path.to_owned());
        }
    }
    println!("format\tsamples\topened\tfailed");
    for (ext, paths) in samples {
        let mut passed = 0;
        for path in &paths {
            let ok = if glimpse_services::media::supports(path) {
                glimpse_services::media::read_image(path).is_ok()
            } else {
                glimpse_services::files::read_document(path).is_ok()
            };
            passed += usize::from(ok);
        }
        println!("{ext}\t{}\t{passed}\t{}", paths.len(), paths.len() - passed);
    }
    Ok(())
}
