use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use git2::{
    BranchType, Cred, CredentialType, FetchOptions, RemoteCallbacks, Repository, StatusOptions,
};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum RepoStatus {
    Clean,
    Dirty,
    Conflicts,
}

#[derive(Debug, Clone)]
pub struct GitInfo {
    pub branch: String,
    pub status: RepoStatus,
    pub ahead: usize,
    pub behind: usize,
    pub last_commit_age: String,
    pub last_commit_secs: u64,
    pub branch_count: usize,
}

#[derive(Debug, Clone)]
pub struct CommitEntry {
    pub message: String,
    pub author: String,
    pub date: String,
}

#[derive(Debug, Clone)]
pub struct BranchEntry {
    pub name: String,
    pub is_head: bool,
    pub upstream_gone: bool,
    pub is_merged: bool,
}

#[derive(Debug, Clone)]
pub struct RepoDetails {
    pub recent_commits: Vec<CommitEntry>,
    pub changed_files: Vec<String>,
    pub remote_urls: Vec<(String, String)>,
    pub branches: Vec<BranchEntry>,
}

/// Collect branch, status, ahead/behind, and last commit age for a repository.
pub fn get_repo_info(path: &Path) -> Result<GitInfo> {
    let repo = Repository::open(path).context("Failed to open repository")?;

    let branch = get_branch(&repo);
    let status = get_status(&repo)?;
    let (ahead, behind) = get_ahead_behind(&repo);
    let (last_commit_age, last_commit_secs) = get_last_commit_age(&repo);
    let branch_count = get_branch_count(&repo);

    Ok(GitInfo {
        branch,
        status,
        ahead,
        behind,
        last_commit_age,
        last_commit_secs,
        branch_count,
    })
}

/// Fetch all configured remotes for a repository.
pub fn fetch_all_remotes(path: &Path) -> Result<()> {
    let repo = Repository::open(path)?;
    let remotes = repo.remotes()?;

    for remote_name in remotes.iter().flatten() {
        let _ = fetch_remote(&repo, remote_name);
    }
    Ok(())
}

/// Fast-forward pull the current branch.
pub fn pull_current_branch(path: &Path) -> Result<String> {
    let repo = Repository::open(path)?;

    let head = repo.head().context("HEAD not found")?;
    let branch_name = head.shorthand().unwrap_or("unknown").to_string();

    let branch = repo
        .find_branch(&branch_name, BranchType::Local)
        .context("Local branch not found")?;
    let upstream = branch
        .upstream()
        .context("No upstream tracking branch")?;
    let upstream_oid = upstream
        .get()
        .target()
        .context("Upstream has no target")?;

    let annotated_commit = repo.find_annotated_commit(upstream_oid)?;
    let (analysis, _) = repo.merge_analysis(&[&annotated_commit])?;

    if analysis.is_up_to_date() {
        return Ok("Already up to date".to_string());
    }

    if !analysis.is_fast_forward() {
        anyhow::bail!("Cannot fast-forward, histories have diverged");
    }

    let refname = format!("refs/heads/{}", branch_name);
    let mut reference = repo.find_reference(&refname)?;
    reference.set_target(upstream_oid, "gruth: fast-forward pull")?;
    repo.set_head(&refname)?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;

    Ok(format!("Fast-forwarded {}", branch_name))
}

/// Get detailed info for a repository (commits, changed files, remotes, branches).
pub fn get_repo_details(path: &Path) -> Result<RepoDetails> {
    let repo = Repository::open(path)?;

    let recent_commits = get_recent_commits(&repo, 10);
    let changed_files = get_changed_files(&repo);
    let remote_urls = get_remote_urls(&repo);
    let branches = get_branch_details(&repo);

    Ok(RepoDetails {
        recent_commits,
        changed_files,
        remote_urls,
        branches,
    })
}

fn get_branch(repo: &Repository) -> String {
    match repo.head() {
        Ok(head) => {
            if head.is_branch() {
                head.shorthand().unwrap_or("unknown").to_string()
            } else {
                match head.target() {
                    Some(oid) => format!("{:.7}", oid),
                    None => "detached".to_string(),
                }
            }
        }
        Err(_) => "no HEAD".to_string(),
    }
}

fn get_status(repo: &Repository) -> Result<RepoStatus> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts))?;

    let mut has_conflicts = false;
    let mut has_changes = false;

    for entry in statuses.iter() {
        let s = entry.status();
        if s.is_conflicted() {
            has_conflicts = true;
            break;
        }
        if !s.is_ignored() {
            has_changes = true;
        }
    }

    if has_conflicts {
        Ok(RepoStatus::Conflicts)
    } else if has_changes {
        Ok(RepoStatus::Dirty)
    } else {
        Ok(RepoStatus::Clean)
    }
}

fn get_ahead_behind(repo: &Repository) -> (usize, usize) {
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return (0, 0),
    };

    let branch_name = match head.shorthand() {
        Some(name) => name.to_string(),
        None => return (0, 0),
    };

    let local_branch = match repo.find_branch(&branch_name, BranchType::Local) {
        Ok(b) => b,
        Err(_) => return (0, 0),
    };

    let upstream = match local_branch.upstream() {
        Ok(u) => u,
        Err(_) => return (0, 0),
    };

    let local_oid = match head.target() {
        Some(oid) => oid,
        None => return (0, 0),
    };

    let upstream_oid = match upstream.get().target() {
        Some(oid) => oid,
        None => return (0, 0),
    };

    repo.graph_ahead_behind(local_oid, upstream_oid)
        .unwrap_or((0, 0))
}

fn get_last_commit_age(repo: &Repository) -> (String, u64) {
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return ("n/a".to_string(), 0),
    };

    let commit = match head.peel_to_commit() {
        Ok(c) => c,
        Err(_) => return ("n/a".to_string(), 0),
    };

    let secs = commit.time().seconds();
    let dt = DateTime::from_timestamp(secs, 0);

    match dt {
        Some(commit_time) => {
            let duration = Utc::now().signed_duration_since(commit_time);
            let std_dur = duration.to_std().unwrap_or(Duration::ZERO);
            let age_secs = std_dur.as_secs();
            (format_duration(std_dur), age_secs)
        }
        None => ("n/a".to_string(), 0),
    }
}

fn get_branch_count(repo: &Repository) -> usize {
    repo.branches(Some(BranchType::Local))
        .map(|branches| branches.count())
        .unwrap_or(0)
}

fn get_recent_commits(repo: &Repository, count: usize) -> Vec<CommitEntry> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return Vec::new(),
    };

    let oid = match head.target() {
        Some(oid) => oid,
        None => return Vec::new(),
    };

    let mut revwalk = match repo.revwalk() {
        Ok(rw) => rw,
        Err(_) => return Vec::new(),
    };

    if revwalk.push(oid).is_err() {
        return Vec::new();
    }

    revwalk
        .filter_map(|r| r.ok())
        .take(count)
        .filter_map(|oid| {
            let commit = repo.find_commit(oid).ok()?;
            let message = commit
                .message()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            let author = commit.author().name().unwrap_or("unknown").to_string();
            let time = commit.time().seconds();
            let date = DateTime::from_timestamp(time, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            Some(CommitEntry {
                message,
                author,
                date,
            })
        })
        .collect()
}

fn get_changed_files(repo: &Repository) -> Vec<String> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    statuses
        .iter()
        .filter_map(|entry| {
            let path = entry.path()?.to_string();
            let s = entry.status();
            let prefix = if s.is_index_new() || s.is_wt_new() {
                "A"
            } else if s.is_index_modified() || s.is_wt_modified() {
                "M"
            } else if s.is_index_deleted() || s.is_wt_deleted() {
                "D"
            } else if s.is_index_renamed() || s.is_wt_renamed() {
                "R"
            } else {
                "?"
            };
            Some(format!("{} {}", prefix, path))
        })
        .collect()
}

fn get_remote_urls(repo: &Repository) -> Vec<(String, String)> {
    let remotes = match repo.remotes() {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    remotes
        .iter()
        .flatten()
        .filter_map(|name| {
            let remote = repo.find_remote(name).ok()?;
            let url = remote.url()?.to_string();
            Some((name.to_string(), url))
        })
        .collect()
}

fn get_branch_details(repo: &Repository) -> Vec<BranchEntry> {
    let branches = match repo.branches(Some(BranchType::Local)) {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };

    let head_oid = repo.head().ok().and_then(|h| h.target());

    branches
        .filter_map(|b| b.ok())
        .map(|(branch, _)| {
            let name = branch
                .name()
                .ok()
                .flatten()
                .unwrap_or("unknown")
                .to_string();
            let is_head = branch.is_head();

            let upstream_gone = match branch.upstream() {
                Ok(_) => false,
                Err(e) => e.code() == git2::ErrorCode::NotFound,
            };

            let is_merged = if is_head {
                false
            } else {
                match (branch.get().target(), head_oid) {
                    (Some(branch_oid), Some(head_oid)) => repo
                        .graph_descendant_of(head_oid, branch_oid)
                        .unwrap_or(false),
                    _ => false,
                }
            };

            BranchEntry {
                name,
                is_head,
                upstream_gone,
                is_merged,
            }
        })
        .collect()
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    match secs {
        0..=59 => format!("{}s ago", secs),
        60..=3599 => format!("{}m ago", secs / 60),
        3600..=86399 => format!("{}h ago", secs / 3600),
        86400..=604799 => format!("{}d ago", secs / 86400),
        604800..=2591999 => format!("{}w ago", secs / 604800),
        _ => format!("{}mo ago", secs / 2592000),
    }
}

fn fetch_remote(repo: &Repository, remote_name: &str) -> Result<()> {
    let mut remote = repo.find_remote(remote_name)?;
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username, allowed| {
        if allowed.contains(CredentialType::SSH_KEY) {
            Cred::ssh_key_from_agent(username.unwrap_or("git"))
        } else if allowed.contains(CredentialType::DEFAULT) {
            Cred::default()
        } else {
            Err(git2::Error::from_str("no suitable credential method"))
        }
    });
    let mut opts = FetchOptions::new();
    opts.remote_callbacks(callbacks);
    remote.fetch(&[] as &[&str], Some(&mut opts), None)?;
    Ok(())
}
