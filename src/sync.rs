//! Headless sync mode — fetch and pull all clean repos.

use crate::git_ops::{self, RepoStatus};
use crate::scanner;
use anyhow::Result;
use std::path::Path;

pub fn run_sync(root: &Path, max_depth: usize, excluded: &[String]) -> Result<()> {
    println!(
        "\x1b[36m gruth sync\x1b[0m \x1b[90m│\x1b[0m scanning {}...",
        root.display()
    );
    println!();

    let repos = scanner::scan_repos(root, max_depth, excluded);
    if repos.is_empty() {
        println!("\x1b[31m  ✗ No git repositories found\x1b[0m");
        return Ok(());
    }

    println!("\x1b[90m  Found {} repositories\x1b[0m", repos.len());
    println!();

    let mut synced = 0;
    let mut skipped = 0;
    let mut errors = 0;
    let mut up_to_date = 0;

    let root_canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    for repo_path in &repos {
        let display = repo_path
            .strip_prefix(&root_canonical)
            .unwrap_or(repo_path)
            .to_string_lossy();

        if let Err(e) = git_ops::fetch_all_remotes(repo_path) {
            println!("\x1b[31m  ✗ FETCH\x1b[0m  {} \x1b[90m({})\x1b[0m", display, e);
            errors += 1;
            continue;
        }

        let info = match git_ops::get_repo_info(repo_path) {
            Ok(info) => info,
            Err(e) => {
                println!("\x1b[31m  ✗ ERR  \x1b[0m  {} \x1b[90m({})\x1b[0m", display, e);
                errors += 1;
                continue;
            }
        };

        match info.status {
            RepoStatus::Dirty | RepoStatus::Conflicts => {
                let label = if info.status == RepoStatus::Conflicts { "conflicts" } else { "dirty" };
                println!("\x1b[33m  ⊘ SKIP \x1b[0m  {} \x1b[90m[{}] ({})\x1b[0m", display, info.branch, label);
                skipped += 1;
            }
            RepoStatus::Clean => {
                if info.behind == 0 {
                    println!("\x1b[32m  ✓ OK   \x1b[0m  {} \x1b[90m[{}]\x1b[0m", display, info.branch);
                    up_to_date += 1;
                } else {
                    match git_ops::pull_current_branch(repo_path) {
                        Ok(msg) => {
                            println!("\x1b[36m  ↓ PULL \x1b[0m  {} \x1b[90m[{}] (↓{} — {})\x1b[0m", display, info.branch, info.behind, msg);
                            synced += 1;
                        }
                        Err(e) => {
                            println!("\x1b[31m  ✗ PULL \x1b[0m  {} \x1b[90m[{}] ({})\x1b[0m", display, info.branch, e);
                            errors += 1;
                        }
                    }
                }
            }
        }
    }

    println!();
    println!("\x1b[90m  ─────────────────────────────────────\x1b[0m");
    println!(
        "  \x1b[36m↓ {}\x1b[0m pulled  \x1b[32m✓ {}\x1b[0m up-to-date  \x1b[33m⊘ {}\x1b[0m skipped  \x1b[31m✗ {}\x1b[0m errors",
        synced, up_to_date, skipped, errors
    );
    println!();

    Ok(())
}
