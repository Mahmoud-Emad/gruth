//! Application state — all data structures and state transitions for the TUI.

use crate::config::Theme;
use crate::git_ops::{BranchEntry, CommitEntry, GitInfo, RepoDetails, RepoStatus};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// --- Enums ---

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    ThemePicker,
    Help,
    ErrorInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatusFilter {
    All,
    Clean,
    Dirty,
    Behind,
    Ahead,
    Errors,
    Stale,
}

impl StatusFilter {
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Clean => "clean",
            Self::Dirty => "dirty",
            Self::Behind => "behind",
            Self::Ahead => "ahead",
            Self::Errors => "errors",
            Self::Stale => "stale",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::All => Self::Clean,
            Self::Clean => Self::Dirty,
            Self::Dirty => Self::Behind,
            Self::Behind => Self::Ahead,
            Self::Ahead => Self::Errors,
            Self::Errors => Self::Stale,
            Self::Stale => Self::All,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortOrder {
    Name,
    Status,
    LastCommit,
    Behind,
}

impl SortOrder {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Status => "status",
            Self::LastCommit => "commit",
            Self::Behind => "behind",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Name => Self::Status,
            Self::Status => Self::LastCommit,
            Self::LastCommit => Self::Behind,
            Self::Behind => Self::Name,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "status" => Self::Status,
            "commit" => Self::LastCommit,
            "behind" => Self::Behind,
            _ => Self::Name,
        }
    }
}

// --- RepoInfo ---

#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub path: PathBuf,
    pub display_name: String,
    pub branch: String,
    pub status: RepoStatus,
    pub ahead: usize,
    pub behind: usize,
    pub last_commit_age: String,
    pub last_commit_secs: u64,
    pub branch_count: usize,
    pub last_updated: Option<Instant>,
    pub error: Option<String>,
    pub fetching: bool,
    pub pulling: bool,
    pub pull_result: Option<Result<String, String>>,
}

impl RepoInfo {
    pub fn new(path: PathBuf, root: &PathBuf) -> Self {
        let display_name = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let display_name = if display_name.is_empty() {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string())
        } else {
            display_name
        };

        Self {
            path,
            display_name,
            branch: "...".to_string(),
            status: RepoStatus::Clean,
            ahead: 0,
            behind: 0,
            last_commit_age: "...".to_string(),
            last_commit_secs: 0,
            branch_count: 0,
            last_updated: None,
            error: None,
            fetching: true,
            pulling: false,
            pull_result: None,
        }
    }

    pub fn update_from_git_info(&mut self, info: GitInfo) {
        self.branch = info.branch;
        self.status = info.status;
        self.ahead = info.ahead;
        self.behind = info.behind;
        self.last_commit_age = info.last_commit_age;
        self.last_commit_secs = info.last_commit_secs;
        self.branch_count = info.branch_count;
        self.last_updated = Some(Instant::now());
        self.error = None;
        self.fetching = false;
        self.pulling = false;
        self.pull_result = None;
    }

    pub fn set_error(&mut self, err: String) {
        self.error = Some(err);
        self.last_updated = Some(Instant::now());
        self.fetching = false;
    }
}

// --- Detail Pane ---

pub struct DetailPane {
    pub display_name: String,
    pub commits: Vec<CommitEntry>,
    pub changed_files: Vec<String>,
    pub remote_urls: Vec<(String, String)>,
    pub branches: Vec<BranchEntry>,
    pub scroll: usize,
}

impl DetailPane {
    pub fn from_details(repo: &RepoInfo, details: RepoDetails) -> Self {
        Self {
            display_name: repo.display_name.clone(),
            commits: details.recent_commits,
            changed_files: details.changed_files,
            remote_urls: details.remote_urls,
            branches: details.branches,
            scroll: 0,
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }
}

// --- Toast Messages ---

#[derive(Debug, Clone)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub created: Instant,
}

impl Toast {
    pub fn is_expired(&self) -> bool {
        self.created.elapsed() > Duration::from_secs(5)
    }
}

// --- AppState ---

pub struct AppState {
    pub repos: Vec<RepoInfo>,
    pub selected: usize,
    pub scan_root: PathBuf,
    pub interval: Duration,
    pub scanning: bool,
    pub last_full_refresh: Option<Instant>,
    pub tick: usize,
    pub pending_refreshes: usize,

    // Stale
    pub stale_days: u64,

    // Filter/Search
    pub input_mode: InputMode,
    pub search_query: String,
    pub status_filter: StatusFilter,
    pub filtered_indices: Vec<usize>,

    // Sort
    pub sort_order: SortOrder,

    // Detail pane
    pub detail_pane: Option<DetailPane>,

    // Theme
    pub theme: Theme,

    // Notifications
    pub notifications: bool,

    // Theme picker
    pub theme_picker_index: usize,
    pub theme_before_preview: Option<Theme>,

    // Toast messages
    pub toasts: Vec<Toast>,

    // Error info overlay
    pub error_info_text: Option<String>,

    // Help overlay scroll
    pub help_scroll: usize,

    // Update available (latest version tag, e.g. "v0.3.0")
    pub update_available: Option<String>,
}

impl AppState {
    pub fn new(
        scan_root: PathBuf,
        interval: Duration,
        stale_days: u64,
        sort_order: SortOrder,
        theme: Theme,
        notifications: bool,
    ) -> Self {
        Self {
            repos: Vec::new(),
            selected: 0,
            scan_root,
            interval,
            scanning: true,
            last_full_refresh: None,
            tick: 0,
            pending_refreshes: 0,
            stale_days,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            status_filter: StatusFilter::All,
            filtered_indices: Vec::new(),
            sort_order,
            detail_pane: None,
            theme,
            notifications,
            theme_picker_index: 0,
            theme_before_preview: None,
            toasts: Vec::new(),
            error_info_text: None,
            help_scroll: 0,
            update_available: None,
        }
    }

    pub fn set_repos(&mut self, paths: Vec<PathBuf>) {
        let count = paths.len();
        self.repos = paths
            .into_iter()
            .map(|p| RepoInfo::new(p, &self.scan_root))
            .collect();
        self.scanning = false;
        self.recompute_filtered();
        if count > 0 {
            self.toast(format!("Found {} repositories", count), ToastLevel::Info);
        }
    }

    /// Reconcile the repo list with a fresh filesystem scan.
    /// Removes repos whose directories no longer exist, adds newly discovered ones,
    /// and preserves state (status, branch, etc.) for repos that are still present.
    /// Returns paths of newly added repos that need an initial refresh.
    pub fn reconcile_repos(&mut self, current_paths: Vec<PathBuf>) -> Vec<PathBuf> {
        use std::collections::HashSet;

        let current_set: HashSet<&PathBuf> = current_paths.iter().collect();
        let existing_set: HashSet<PathBuf> = self.repos.iter().map(|r| r.path.clone()).collect();

        // Remove repos that no longer exist on disk
        let before = self.repos.len();
        self.repos.retain(|r| current_set.contains(&r.path));
        let removed = before - self.repos.len();

        // Add repos that are new
        let new_paths: Vec<PathBuf> = current_paths
            .into_iter()
            .filter(|p| !existing_set.contains(p))
            .collect();
        let added = new_paths.len();

        for p in &new_paths {
            self.repos.push(RepoInfo::new(p.clone(), &self.scan_root));
        }

        if removed > 0 || added > 0 {
            let mut parts = Vec::new();
            if added > 0 {
                parts.push(format!("+{} new", added));
            }
            if removed > 0 {
                parts.push(format!("-{} removed", removed));
            }
            self.toast(format!("Repos updated: {}", parts.join(", ")), ToastLevel::Info);
        }

        self.recompute_filtered();
        new_paths
    }

    pub fn update_repo(&mut self, path: &PathBuf, result: Result<GitInfo, String>) -> Option<String> {
        let mut notify_name = None;
        if let Some(repo) = self.repos.iter_mut().find(|r| &r.path == path) {
            match result {
                Ok(ref info) => {
                    // Detect: was synced (behind=0), now behind
                    if self.notifications
                        && repo.last_updated.is_some()
                        && repo.behind == 0
                        && info.behind > 0
                    {
                        notify_name = Some(repo.display_name.clone());
                    }
                    repo.update_from_git_info(info.clone());
                }
                Err(err) => repo.set_error(err),
            }
        }
        self.pending_refreshes = self.pending_refreshes.saturating_sub(1);
        self.recompute_filtered();
        notify_name
    }

    pub fn should_refresh(&self) -> bool {
        match self.last_full_refresh {
            Some(last) => last.elapsed() >= self.interval,
            None => !self.scanning,
        }
    }

    pub fn mark_refreshing(&mut self) {
        self.last_full_refresh = Some(Instant::now());
        self.pending_refreshes = self.repos.len();
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn spinner(&self) -> &'static str {
        SPINNER_FRAMES[self.tick % SPINNER_FRAMES.len()]
    }

    pub fn is_refreshing(&self) -> bool {
        self.pending_refreshes > 0 || self.scanning
    }

    // --- Stale ---

    pub fn is_stale(&self, repo: &RepoInfo) -> bool {
        repo.last_commit_secs >= self.stale_days * 86400 && repo.last_commit_secs > 0
    }

    pub fn stale_count(&self) -> usize {
        self.repos
            .iter()
            .filter(|r| self.is_stale(r))
            .count()
    }

    // --- Navigation ---

    pub fn select_next(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = (self.selected + 1).min(self.filtered_indices.len() - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn selected_repo(&self) -> Option<&RepoInfo> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&idx| self.repos.get(idx))
    }

    // --- Filter/Search ---

    pub fn cycle_filter(&mut self) {
        self.status_filter = self.status_filter.next();
        self.recompute_filtered();
        let label = self.status_filter.label();
        let count = self.filtered_indices.len();
        self.toast(
            format!("Filter: {} ({} repos)", label, count),
            ToastLevel::Info,
        );
    }

    pub fn cycle_sort(&mut self) {
        self.sort_order = self.sort_order.next();
        self.recompute_filtered();
        self.toast(
            format!("Sort: {}", self.sort_order.label()),
            ToastLevel::Info,
        );
    }

    pub fn recompute_filtered(&mut self) {
        let query = self.search_query.to_lowercase();

        let mut indices: Vec<usize> = (0..self.repos.len())
            .filter(|&i| {
                let repo = &self.repos[i];

                // Search filter
                if !query.is_empty() && !repo.display_name.to_lowercase().contains(&query) {
                    return false;
                }

                // Status filter
                match self.status_filter {
                    StatusFilter::All => true,
                    StatusFilter::Clean => {
                        repo.status == RepoStatus::Clean && repo.error.is_none()
                    }
                    StatusFilter::Dirty => repo.status == RepoStatus::Dirty,
                    StatusFilter::Behind => repo.behind > 0,
                    StatusFilter::Ahead => repo.ahead > 0,
                    StatusFilter::Errors => repo.error.is_some(),
                    StatusFilter::Stale => self.is_stale(repo),
                }
            })
            .collect();

        // Sort
        let repos = &self.repos;
        match self.sort_order {
            SortOrder::Name => indices.sort_by(|&a, &b| {
                repos[a].display_name.cmp(&repos[b].display_name)
            }),
            SortOrder::Status => indices.sort_by(|&a, &b| {
                let status_rank = |r: &RepoInfo| -> u8 {
                    if r.error.is_some() {
                        0
                    } else {
                        match r.status {
                            RepoStatus::Conflicts => 1,
                            RepoStatus::Dirty => 2,
                            RepoStatus::Clean => 3,
                        }
                    }
                };
                status_rank(&repos[a])
                    .cmp(&status_rank(&repos[b]))
                    .then(repos[a].display_name.cmp(&repos[b].display_name))
            }),
            SortOrder::LastCommit => indices.sort_by(|&a, &b| {
                repos[a]
                    .last_commit_secs
                    .cmp(&repos[b].last_commit_secs)
                    .then(repos[a].display_name.cmp(&repos[b].display_name))
            }),
            SortOrder::Behind => indices.sort_by(|&a, &b| {
                repos[b]
                    .behind
                    .cmp(&repos[a].behind)
                    .then(repos[a].display_name.cmp(&repos[b].display_name))
            }),
        }

        self.filtered_indices = indices;
        self.selected = self
            .selected
            .min(self.filtered_indices.len().saturating_sub(1));
    }

    // --- Detail Pane ---

    pub fn open_detail_pane(&mut self, details: RepoDetails) {
        if let Some(repo) = self.selected_repo() {
            self.detail_pane = Some(DetailPane::from_details(repo, details));
        }
    }

    pub fn close_detail_pane(&mut self) {
        self.detail_pane = None;
    }

    // --- Pull ---

    pub fn set_pulling(&mut self, path: &PathBuf) {
        let display_name = self
            .repos
            .iter()
            .find(|r| &r.path == path)
            .map(|r| r.display_name.clone());

        if let Some(repo) = self.repos.iter_mut().find(|r| &r.path == path) {
            repo.pulling = true;
            repo.pull_result = None;
        }

        if let Some(name) = display_name {
            self.toast(format!("Pulling {}...", name), ToastLevel::Info);
        }
    }

    pub fn set_pull_result(&mut self, path: &PathBuf, result: Result<String, String>) {
        let display_name = self
            .repos
            .iter()
            .find(|r| &r.path == path)
            .map(|r| r.display_name.clone())
            .unwrap_or_default();

        if let Some(repo) = self.repos.iter_mut().find(|r| &r.path == path) {
            repo.pulling = false;
            repo.pull_result = Some(result.clone());
        }

        match result {
            Ok(msg) => self.toast(
                format!("{}: {}", display_name, msg),
                ToastLevel::Success,
            ),
            Err(err) => self.toast(
                format!("{}: {}", display_name, err),
                ToastLevel::Error,
            ),
        }
    }

    // --- Theme Picker ---

    pub fn open_theme_picker(&mut self) {
        self.theme_before_preview = Some(self.theme.clone());
        self.input_mode = InputMode::ThemePicker;

        // Start on the currently cached theme, or 0
        let presets = Theme::presets();
        let cached = crate::config::load_cached_theme();
        self.theme_picker_index = cached
            .and_then(|name| presets.iter().position(|p| p.name == name))
            .unwrap_or(0);
    }

    pub fn theme_picker_next(&mut self) {
        let presets = Theme::presets();
        if !presets.is_empty() {
            self.theme_picker_index = (self.theme_picker_index + 1).min(presets.len() - 1);
            self.theme = presets[self.theme_picker_index].theme.clone();
        }
    }

    pub fn theme_picker_prev(&mut self) {
        let presets = Theme::presets();
        self.theme_picker_index = self.theme_picker_index.saturating_sub(1);
        if let Some(preset) = presets.get(self.theme_picker_index) {
            self.theme = preset.theme.clone();
        }
    }

    pub fn theme_picker_confirm(&mut self) {
        let presets = Theme::presets();
        let name = presets
            .get(self.theme_picker_index)
            .map(|p| p.name)
            .unwrap_or("Custom");
        crate::config::save_cached_theme(name);
        self.toast(format!("Theme: {}", name), ToastLevel::Success);
        self.theme_before_preview = None;
        self.input_mode = InputMode::Normal;
    }

    pub fn theme_picker_cancel(&mut self) {
        if let Some(original) = self.theme_before_preview.take() {
            self.theme = original;
        }
        self.input_mode = InputMode::Normal;
    }

    // --- Help ---

    pub fn open_help(&mut self) {
        self.help_scroll = 0;
        self.input_mode = InputMode::Help;
    }

    pub fn close_help(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    // --- Error Info ---

    pub fn show_error_info(&mut self) {
        if let Some(repo) = self.selected_repo() {
            if let Some(ref err) = repo.error {
                self.error_info_text = Some(err.clone());
                self.input_mode = InputMode::ErrorInfo;
            }
        }
    }

    pub fn close_error_info(&mut self) {
        self.error_info_text = None;
        self.input_mode = InputMode::Normal;
    }

    // --- Toasts ---

    pub fn toast(&mut self, message: String, level: ToastLevel) {
        self.toasts.push(Toast {
            message,
            level,
            created: Instant::now(),
        });
    }

    pub fn expire_toasts(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    pub fn active_toast(&self) -> Option<&Toast> {
        self.toasts.last()
    }

    // --- Stats ---

    pub fn repo_count(&self) -> usize {
        self.filtered_indices.len()
    }

    pub fn total_count(&self) -> usize {
        self.repos.len()
    }

    pub fn clean_count(&self) -> usize {
        self.repos
            .iter()
            .filter(|r| r.status == RepoStatus::Clean && r.error.is_none())
            .count()
    }

    pub fn dirty_count(&self) -> usize {
        self.repos
            .iter()
            .filter(|r| r.status == RepoStatus::Dirty)
            .count()
    }

    pub fn error_count(&self) -> usize {
        self.repos.iter().filter(|r| r.error.is_some()).count()
    }
}
