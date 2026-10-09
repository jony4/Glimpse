//! Build a real hierarchy before flattening visible rows for the virtual list.
use super::Row;
use glimpse_core::{DiffScope, GitChange};
use std::{
    collections::{BTreeMap, HashSet},
    ffi::OsString,
    path::PathBuf,
};

#[derive(Default)]
struct Directory {
    directories: BTreeMap<OsString, Directory>,
    files: Vec<usize>,
}
pub(super) fn rows(
    changes: &[GitChange],
    tree: bool,
    collapsed: &HashSet<(DiffScope, PathBuf)>,
) -> Vec<Row> {
    let mut rows = Vec::new();
    for scope in [
        DiffScope::Conflict,
        DiffScope::Index,
        DiffScope::Worktree,
        DiffScope::Untracked,
    ] {
        let mut files = changes
            .iter()
            .enumerate()
            .filter(|(_, c)| c.scope == scope)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if files.is_empty() {
            continue;
        }
        rows.push(Row::Group(scope));
        files.sort_by(|a, b| changes[*a].path.cmp(&changes[*b].path));
        if !tree {
            rows.extend(files.into_iter().map(|i| Row::File(i, 0)));
            continue;
        }
        let mut root = Directory::default();
        for index in files {
            let mut node = &mut root;
            if let Some(parent) = changes[index].path.parent() {
                for component in parent.components() {
                    node = node
                        .directories
                        .entry(component.as_os_str().to_owned())
                        .or_default();
                }
            }
            node.files.push(index);
        }
        flatten(&root, &PathBuf::new(), 0, scope, collapsed, &mut rows);
    }
    rows
}
fn flatten(
    node: &Directory,
    path: &std::path::Path,
    depth: usize,
    scope: DiffScope,
    collapsed: &HashSet<(DiffScope, PathBuf)>,
    rows: &mut Vec<Row>,
) {
    // Folders first, then files, at every level. A subtree is always contiguous.
    for (name, child) in &node.directories {
        let path = path.join(name);
        rows.push(Row::Directory(scope, path.clone(), depth));
        if !collapsed.contains(&(scope, path.clone())) {
            flatten(child, &path, depth + 1, scope, collapsed, rows);
        }
    }
    rows.extend(node.files.iter().map(|&i| Row::File(i, depth)));
}
#[cfg(test)]
mod tests {
    use super::*;
    fn change(path: &str, scope: DiffScope) -> GitChange {
        GitChange {
            path: path.into(),
            original_path: None,
            status: 'M',
            scope,
        }
    }
    #[test]
    fn folders_precede_files_and_collapse_stays_within_group() {
        let changes = vec![
            change("a.rs", DiffScope::Worktree),
            change("src/z.rs", DiffScope::Worktree),
            change("src/deep/x.rs", DiffScope::Worktree),
            change("src/deep/x.rs", DiffScope::Index),
            change("src/a.rs", DiffScope::Worktree),
        ];
        let collapsed = HashSet::from([(DiffScope::Worktree, PathBuf::from("src/deep"))]);
        let actual = rows(&changes, true, &collapsed);
        assert!(matches!(actual[0], Row::Group(DiffScope::Index)));
        assert!(matches!(actual[3], Row::File(3, 2)));
        assert!(
            matches!(&actual[5], Row::Directory(DiffScope::Worktree, p, 0) if p == std::path::Path::new("src"))
        );
        assert!(matches!(actual[7], Row::File(4, 1)));
        assert!(matches!(actual[8], Row::File(1, 1)));
        assert!(matches!(actual[9], Row::File(0, 0)));
        assert_eq!(actual.len(), 10);
        assert_eq!(rows(&changes, false, &collapsed).len(), 7);
    }
}
