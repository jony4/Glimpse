use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffSpan {
    pub range: Range<usize>,
    pub addition: bool,
}

/// Find changed lines in unified/combined patches without mistaking file headers for edits.
/// Ranges are UTF-8 byte offsets, suitable for read-only text presentation.
pub fn changed_lines(patch: &str) -> Vec<DiffSpan> {
    let mut spans = Vec::new();
    let mut offset = 0;
    let mut columns = 0;
    for line in patch.split_inclusive('\n') {
        if line.starts_with("diff ") {
            columns = 0;
        } else if line.starts_with("@@") {
            columns = line.bytes().take_while(|byte| *byte == b'@').count() - 1;
        } else if columns > 0 {
            let prefix = &line.as_bytes()[..columns.min(line.len())];
            if prefix.iter().all(|byte| matches!(byte, b' ' | b'+' | b'-')) {
                let added = prefix.contains(&b'+');
                let removed = prefix.contains(&b'-');
                if added != removed {
                    spans.push(DiffSpan {
                        range: offset..offset + line.trim_end_matches('\n').len(),
                        addition: added,
                    });
                }
            }
        }
        offset += line.len();
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_ranges_preserve_unicode_and_exclude_headers() {
        let patch = "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-旧内容\n+新内容\ndiff --git a/b b/b\n+++ b/b\n@@@ -1 -1 +1 @@@\n +combined\n";
        let spans = changed_lines(patch);
        assert_eq!(
            spans
                .iter()
                .map(|s| (&patch[s.range.clone()], s.addition))
                .collect::<Vec<_>>(),
            [("-旧内容", false), ("+新内容", true), (" +combined", true)]
        );
    }
}
