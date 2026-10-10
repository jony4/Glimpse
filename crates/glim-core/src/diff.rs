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

/// Present only hunk contents, with highlight ranges in the displayed text.
/// Keep non-textual changes informative without exposing patch metadata.
pub fn diff_content(patch: &str) -> (String, Vec<DiffSpan>) {
    let original_spans = changed_lines(patch);
    let mut original_spans = original_spans.iter().peekable();
    let mut spans = Vec::new();
    let mut text = String::new();
    let mut offset = 0;
    let mut columns = 0;
    for line in patch.split_inclusive('\n') {
        if line.starts_with("diff ") {
            columns = 0;
        } else if line.starts_with("@@") {
            columns = line.bytes().take_while(|b| *b == b'@').count() - 1;
            if !text.is_empty() {
                text.push('\n');
            }
        } else if columns > 0
            && line
                .as_bytes()
                .get(..columns)
                .is_some_and(|prefix| prefix.iter().all(|b| matches!(b, b' ' | b'+' | b'-')))
        {
            let start = text.len();
            text.push_str(line);
            while original_spans
                .peek()
                .is_some_and(|s| s.range.start < offset)
            {
                original_spans.next();
            }
            if let Some(span) = original_spans.peek().filter(|s| s.range.start == offset) {
                spans.push(DiffSpan {
                    range: start..start + line.trim_end_matches('\n').len(),
                    addition: span.addition,
                });
            }
        }
        offset += line.len();
    }
    if text.is_empty() {
        text.push_str(
            if patch.contains("Binary files") || patch.contains("GIT binary patch") {
                "Binary file changed."
            } else {
                "No textual changes."
            },
        );
    }
    (text, spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_hides_metadata_but_keeps_header_like_content_and_unicode() {
        let patch = "diff --git a/a b/a\nnew file mode 100644\nindex 000..111\n--- /dev/null\n+++ b/a\n@@ -0,0 +1,2 @@\n+++实际内容\n+中文\n@@ -8 +10 @@\n-old\n+new\n";
        let (text, spans) = diff_content(patch);
        assert_eq!(text, "+++实际内容\n+中文\n\n-old\n+new\n");
        assert_eq!(
            spans
                .iter()
                .map(|s| (&text[s.range.clone()], s.addition))
                .collect::<Vec<_>>(),
            [
                ("+++实际内容", true),
                ("+中文", true),
                ("-old", false),
                ("+new", true)
            ]
        );
        let (combined, spans) = diff_content("diff --cc a\n@@@ -1 -1 +1 @@@\n +合并\n");
        assert_eq!(combined, " +合并\n");
        assert_eq!(&combined[spans[0].range.clone()], " +合并");
        assert_eq!(
            diff_content("Binary files a/a and b/a differ\n").0,
            "Binary file changed."
        );
        assert_eq!(
            diff_content("old mode 100644\nnew mode 100755\n").0,
            "No textual changes."
        );
    }

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
