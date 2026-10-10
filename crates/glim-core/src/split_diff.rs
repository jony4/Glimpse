/// Two aligned text columns built from unified hunks. Combined merge patches are
/// deliberately left in their original inline representation.
pub struct SplitPatch {
    pub before: String,
    pub after: String,
    pub removed: Vec<std::ops::Range<usize>>,
    pub added: Vec<std::ops::Range<usize>>,
}

pub fn split_patch(patch: &str) -> Option<SplitPatch> {
    if patch.is_empty() || patch.lines().any(|line| line.starts_with("@@@")) {
        return None;
    }
    let mut out = SplitPatch {
        before: String::new(),
        after: String::new(),
        removed: Vec::new(),
        added: Vec::new(),
    };
    let (mut old, mut new) = (0usize, 0usize);
    let (mut deletes, mut adds) = (Vec::new(), Vec::new());
    let mut in_hunk = false;
    fn append(
        text: &mut String,
        spans: &mut Vec<std::ops::Range<usize>>,
        line: Option<(usize, &str)>,
        changed: bool,
    ) {
        let start = text.len();
        if let Some((number, content)) = line {
            text.push_str(&format!("{number:>5}  {content}"));
        }
        text.push('\n');
        if changed && line.is_some() {
            if let Some(previous) = spans.last_mut().filter(|r| r.end == start) {
                previous.end = text.len();
            } else {
                spans.push(start..text.len());
            }
        }
    }
    fn flush(
        out: &mut SplitPatch,
        deletes: &mut Vec<(usize, String)>,
        adds: &mut Vec<(usize, String)>,
    ) {
        for i in 0..deletes.len().max(adds.len()) {
            append(
                &mut out.before,
                &mut out.removed,
                deletes.get(i).map(|(n, s)| (*n, s.as_str())),
                true,
            );
            append(
                &mut out.after,
                &mut out.added,
                adds.get(i).map(|(n, s)| (*n, s.as_str())),
                true,
            );
        }
        deletes.clear();
        adds.clear();
    }
    for line in patch.lines() {
        if line.starts_with("@@ ") {
            flush(&mut out, &mut deletes, &mut adds);
            let mut parts = line.split_whitespace().skip(1);
            old = parts
                .next()?
                .trim_start_matches('-')
                .split(',')
                .next()?
                .parse()
                .ok()?;
            new = parts
                .next()?
                .trim_start_matches('+')
                .split(',')
                .next()?
                .parse()
                .ok()?;
            if in_hunk {
                out.before.push('\n');
                out.after.push('\n');
            }
            in_hunk = true;
        } else if in_hunk && line.starts_with('-') {
            deletes.push((old, line[1..].to_owned()));
            old += 1;
        } else if in_hunk && line.starts_with('+') {
            adds.push((new, line[1..].to_owned()));
            new += 1;
        } else {
            if line.starts_with('\\') {
                continue;
            }
            flush(&mut out, &mut deletes, &mut adds);
            if in_hunk && line.starts_with(' ') {
                append(
                    &mut out.before,
                    &mut out.removed,
                    Some((old, &line[1..])),
                    false,
                );
                append(
                    &mut out.after,
                    &mut out.added,
                    Some((new, &line[1..])),
                    false,
                );
                old += 1;
                new += 1;
            } else {
                in_hunk = false;
            }
        }
    }
    flush(&mut out, &mut deletes, &mut adds);
    (!out.before.is_empty() || !out.after.is_empty()).then_some(out)
}

/// Compose independent file patches without losing file boundaries or binary changes.
pub fn split_commit_patch(patch: &str) -> Option<SplitPatch> {
    let mut result = SplitPatch {
        before: String::new(),
        after: String::new(),
        removed: Vec::new(),
        added: Vec::new(),
    };
    let starts: Vec<_> = patch
        .match_indices("diff --git ")
        .filter(|(i, _)| *i == 0 || patch.as_bytes()[i - 1] == b'\n')
        .map(|(i, _)| i)
        .collect();
    if starts.is_empty() {
        return None;
    }
    for (index, start) in starts.iter().enumerate() {
        let section = &patch[*start..starts.get(index + 1).copied().unwrap_or(patch.len())];
        let title = section.lines().next()?.strip_prefix("diff --git ")?;
        let title = title.split_once(" b/").map_or(title, |(_, path)| path);
        let heading = format!("\n{title}\n\n");
        result.before.push_str(&heading);
        result.after.push_str(&heading);
        if let Some(file) = split_patch(section) {
            let left = result.before.len();
            let right = result.after.len();
            result.removed.extend(
                file.removed
                    .into_iter()
                    .map(|r| r.start + left..r.end + left),
            );
            result.added.extend(
                file.added
                    .into_iter()
                    .map(|r| r.start + right..r.end + right),
            );
            result.before.push_str(&file.before);
            result.after.push_str(&file.after);
        } else {
            let (message, _) = crate::diff_content(section);
            result.before.push_str(&message);
            result.before.push('\n');
            result.after.push_str(&message);
            result.after.push('\n');
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_change_blocks_use_one_decoration_and_do_not_cover_context() {
        let patch = format!("@@ -1 +1,10001 @@\n context\n{}", "+新增\n".repeat(10_000));
        let split = split_patch(&patch).unwrap();
        assert_eq!(split.added.len(), 1);
        let changed = &split.after[split.added[0].clone()];
        assert_eq!(changed.lines().count(), 10_000);
        assert!(!changed.contains("context"));
        assert!(split.removed.is_empty());
        assert_eq!(split.before.lines().count(), split.after.lines().count());
    }

    #[test]
    fn hides_headers_and_retains_original_numbers_across_hunks() {
        let split = split_patch("diff --git a/a b/a\nindex aaa..bbb\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+新\n@@ -20 +20 @@\n context\n").unwrap();
        assert_eq!(split.before, "    1  old\n\n   20  context\n");
        assert_eq!(split.after, "    1  新\n\n   20  context\n");
        assert_eq!(&split.after[split.added[0].clone()], "    1  新\n");
        assert!(split_patch("Binary files a/a and b/a differ\n").is_none());
    }

    #[test]
    fn aligns_replacements_and_preserves_unicode_line_numbers() {
        let split = split_patch("@@ -8,2 +8,3 @@\n-old\n+新\n+extra\n context\n").unwrap();
        assert_eq!(split.before.lines().count(), split.after.lines().count());
        assert!(split.before.contains("    8  old\n\n    9  context"));
        assert!(
            split
                .after
                .contains("    8  新\n    9  extra\n   10  context")
        );
        assert!(split.after[split.added[0].clone()].contains("新"));
        assert!(split_patch("@@@ -1,1 -1,1 +1,1 @@@").is_none());
    }
}
