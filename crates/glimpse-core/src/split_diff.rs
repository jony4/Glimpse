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
            spans.push(start..text.len());
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
            out.before.push_str(line);
            out.before.push('\n');
            out.after.push_str(line);
            out.after.push('\n');
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
            } else if !in_hunk {
                out.before.push_str(line);
                out.before.push('\n');
                out.after.push_str(line);
                out.after.push('\n');
            }
        }
    }
    flush(&mut out, &mut deletes, &mut adds);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
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
