//! Header-only inspection. Tensor payloads are never read or executed.
use crate::binary::BytePreview;
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, fmt::Write as _, fs::File, io::Read, path::Path};

pub fn read_preview(path: &Path) -> Result<BytePreview> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut prefix = [0u8; 8];
    file.read_exact(&mut prefix)
        .context("Missing safetensors header length")?;
    let length = u64::from_le_bytes(prefix);
    ensure!(
        length <= 8 * 1024 * 1024,
        "Safetensors header exceeds the 8 MiB preview budget"
    );
    ensure!(
        length <= size.saturating_sub(8),
        "Truncated safetensors header"
    );
    let mut bytes = vec![0; length as usize];
    file.read_exact(&mut bytes)?;
    ensure!(
        bytes.first() == Some(&b'{'),
        "Invalid safetensors JSON header"
    );
    let header: serde_json::Value = serde_json::from_slice(&bytes)?;
    let entries = header
        .as_object()
        .context("Expected a safetensors object")?;
    let payload = size - 8 - length;
    let mut types = BTreeMap::<&str, usize>::new();
    let mut parameters = 0u128;
    let mut count = 0usize;
    let mut rows = String::new();
    for (name, tensor) in entries {
        if name == "__metadata__" {
            continue;
        }
        let dtype = tensor["dtype"].as_str().context("Missing tensor dtype")?;
        let shape = tensor["shape"].as_array().context("Missing tensor shape")?;
        let mut elements = 1u128;
        for dimension in shape {
            elements = elements
                .checked_mul(dimension.as_u64().context("Invalid tensor dimension")? as u128)
                .context("Tensor shape overflow")?;
        }
        parameters = parameters
            .checked_add(elements)
            .context("Parameter count overflow")?;
        let offsets = tensor["data_offsets"]
            .as_array()
            .context("Missing tensor offsets")?;
        ensure!(offsets.len() == 2, "Invalid tensor offsets");
        let begin = offsets[0].as_u64().context("Invalid tensor offset")?;
        let end = offsets[1].as_u64().context("Invalid tensor offset")?;
        ensure!(
            begin <= end && end <= payload,
            "Tensor extends beyond the file"
        );
        *types.entry(dtype).or_default() += 1;
        count += 1;
        // Bound the rendered report independently of header size and tensor count.
        if count <= 2000 && rows.len() < 512 * 1024 {
            writeln!(
                rows,
                "{}  ·  {}  ·  {}  ·  {} bytes",
                clipped(name, 200),
                clipped(dtype, 40),
                shape
                    .iter()
                    .take(32)
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(" × ")
                    + if shape.len() > 32 {
                        " × …"
                    } else if shape.is_empty() {
                        "[] (scalar)"
                    } else {
                        ""
                    },
                end - begin
            )?;
        }
    }
    let mut text = format!(
        "SAFETENSORS\n\nFile size       {size} bytes\nHeader          {length} bytes\nTensor payload  {payload} bytes\nTensors         {count}\nParameters      {parameters}\n\nPRECISION\n"
    );
    let type_count = types.len();
    for (dtype, count) in types.into_iter().take(100) {
        writeln!(text, "{:<12} {count} tensors", clipped(dtype, 40))?;
    }
    if type_count > 100 {
        text.push_str("… Precision listing truncated.\n");
    }
    if let Some(metadata) = entries.get("__metadata__") {
        let metadata = metadata.as_object().context("Invalid metadata map")?;
        text.push_str("\nMETADATA\n");
        for (key, value) in metadata.iter().take(100) {
            writeln!(
                text,
                "{}: {}",
                clipped(key, 100),
                clipped(
                    value.as_str().context("Metadata values must be strings")?,
                    2000
                )
            )?;
        }
        if metadata.len() > 100 {
            text.push_str("… Metadata preview limited to 100 entries.\n");
        }
    }
    text.push_str("\nTENSORS · name / dtype / shape / storage\n");
    text.push_str(&rows);
    if count > 2000 || rows.len() >= 512 * 1024 {
        text.push_str("… Tensor listing truncated to the preview budget.\n");
    }
    text.push_str("\nHeader inspection only; weights were not loaded. Offsets are range-checked; this is not full model validation.\n");
    Ok(BytePreview {
        path: std::path::absolute(path)?,
        text,
    })
}
fn clipped(value: &str, limit: usize) -> String {
    let mut result: String = value
        .chars()
        .take(limit)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if value.chars().count() > limit {
        result.push('…');
    }
    result
}
