//! Recursive git repository discovery.

use glob::Pattern;
use std::path::{Path, PathBuf};

/// Recursively discover git repositories under `root` up to `max_depth` levels deep.
/// The root directory itself is excluded from results.
/// Directories matching any pattern in `excluded` are skipped.
pub fn scan_repos(root: &Path, max_depth: usize, excluded: &[String]) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let patterns: Vec<Pattern> = excluded
        .iter()
        .filter_map(|p| Pattern::new(p).ok())
        .collect();
    scan_recursive(&root, 0, max_depth, &patterns, &mut repos);
    repos.sort();
    repos
}

fn scan_recursive(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    excluded: &[Pattern],
    repos: &mut Vec<PathBuf>,
) {
    if depth > max_depth {
        return;
    }

    // Only record child repos, not the root itself
    if depth > 0 && dir.join(".git").exists() {
        repos.push(dir.to_path_buf());
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('.') {
            continue;
        }

        if excluded.iter().any(|p| p.matches(&name_str)) {
            continue;
        }

        if let Ok(ft) = entry.file_type() {
            if ft.is_symlink() {
                continue;
            }
        }

        scan_recursive(&path, depth + 1, max_depth, excluded, repos);
    }
}
