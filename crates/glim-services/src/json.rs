//! Bounded JSON structure for an on-demand, read-only tree.
use anyhow::{Result, ensure};
use serde_json::Value;

pub struct JsonNode {
    pub key: String,
    pub value: String,
    pub kind: &'static str,
    pub children: Vec<usize>,
}
pub fn parse(text: &str) -> Result<Vec<JsonNode>> {
    ensure!(
        text.len() <= 4 * 1024 * 1024,
        "Tree view is limited to 4 MiB; use Source for larger JSON."
    );
    let value: Value = serde_json::from_str(text)?;
    let mut nodes = Vec::new();
    fn add(key: String, value: Value, nodes: &mut Vec<JsonNode>) -> Result<usize> {
        ensure!(
            nodes.len() < 50_000,
            "Tree view is limited to 50,000 nodes; use Source."
        );
        let index = nodes.len();
        let (kind, label) = match &value {
            Value::Object(v) => ("object", format!("{{ {} keys }}", v.len())),
            Value::Array(v) => ("array", format!("[ {} items ]", v.len())),
            Value::String(v) => (
                "string",
                format!(
                    "{:?}{}",
                    v.chars().take(512).collect::<String>(),
                    if v.chars().count() > 512 { "…" } else { "" }
                ),
            ),
            Value::Number(v) => ("number", v.to_string()),
            Value::Bool(v) => ("boolean", v.to_string()),
            Value::Null => ("null", "null".into()),
        };
        nodes.push(JsonNode {
            key: key.chars().take(512).collect(),
            value: label,
            kind,
            children: Vec::new(),
        });
        match value {
            Value::Object(values) => {
                for (key, value) in values {
                    let child = add(key, value, nodes)?;
                    nodes[index].children.push(child);
                }
            }
            Value::Array(values) => {
                for (i, value) in values.into_iter().enumerate() {
                    let child = add(format!("[{i}]"), value, nodes)?;
                    nodes[index].children.push(child);
                }
            }
            _ => {}
        }
        Ok(index)
    }
    add("$".into(), value, &mut nodes)?;
    Ok(nodes)
}
