//! gruth — Git Repository UTility Helper
//!
//! A TUI dashboard for monitoring and syncing multiple git repositories.
//! Run `gruth` to launch the directory picker, or `gruth -p <dir>` to
//! start monitoring immediately. Use `gruth --sync` for headless batch pulls.

mod app;
mod cli;
mod config;
mod dir_picker;
mod events;
mod git_ops;
mod scanner;
mod sync;
mod ui;

use anyhow::Result;
use app::{AppState, InputMode, SortOrder, ToastLevel};
use clap::Parser;
use cli::Args;
use crossterm::{
    event::{KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use events::{AppEvent, EventHandler};
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

enum ExitReason {
    Quit,
    BackToPicker,
}

enum RepoResult {
    ScanComplete(Vec<PathBuf>),
    RescanComplete(Vec<PathBuf>),
    RepoUpdated(PathBuf, Result<git_ops::GitInfo, String>),
    PullComplete(PathBuf, Result<String, String>),
    DetailLoaded(PathBuf, Result<git_ops::RepoDetails, String>),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = config::Config::load();

    let depth = args.depth.or(config.depth).unwrap_or(10);
    let interval_secs = args.interval.or(config.interval).unwrap_or(5);
    let stale_days = args.stale_days.or(config.stale_days).unwrap_or(30);
    let default_sort = SortOrder::from_str(config.default_sort.as_deref().unwrap_or("name"));
    let theme = config::resolve_theme(&config);
    let notifications = config.notifications.unwrap_or(true);
    let excluded = config.excluded_paths.unwrap_or_default();

    // Sync mode: headless fetch + pull, then exit
    if args.sync {
        let root = match args.path {
            Some(p) => p.canonicalize().unwrap_or(p),
            None => std::env::current_dir()?,
        };
        return sync::run_sync(&root, depth, &excluded);
    }

    // Direct path: run TUI once, no picker loop
    if let Some(p) = args.path {
        let root = p.canonicalize().unwrap_or(p);
        run_tui(root, depth, interval_secs, stale_days, default_sort, excluded, theme, notifications).await?;
        return Ok(());
    }

    // No path given: picker → TUI → picker loop
    loop {
        let root = match dir_picker::run_picker()? {
            Some(p) => p,
            None => return Ok(()),
        };

        match run_tui(
            root,
            depth,
            interval_secs,
            stale_days,
            default_sort.clone(),
            excluded.clone(),
            theme.clone(),
            notifications,
        )
        .await?
        {
            ExitReason::Quit => return Ok(()),
            ExitReason::BackToPicker => continue,
        }
    }
}

async fn run_tui(
    root: PathBuf,
    max_depth: usize,
    interval_secs: u64,
    stale_days: u64,
    default_sort: SortOrder,
    excluded: Vec<String>,
    theme: config::Theme,
    notifications: bool,
) -> Result<ExitReason> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let interval = Duration::from_secs(interval_secs);
    let mut app = AppState::new(root.clone(), interval, stale_days, default_sort, theme, notifications);
    let mut events = EventHandler::new(Duration::from_millis(250));
    let (tx, mut rx) = mpsc::unbounded_channel::<RepoResult>();

    // Kick off background repo scan
    let scan_tx = tx.clone();
    let scan_root = root.clone();
    let scan_excluded = excluded.clone();
    tokio::task::spawn_blocking(move || {
        let repos = scanner::scan_repos(&scan_root, max_depth, &scan_excluded);
        let _ = scan_tx.send(RepoResult::ScanComplete(repos));
    });

    let exit_reason = loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        while let Ok(result) = rx.try_recv() {
            handle_result(&mut app, result, &tx);
        }

        tokio::select! {
            event = events.next() => {
                match event? {
                    AppEvent::Key(key) => {
                        if let Some(reason) = handle_key(&mut app, key, &tx) {
                            break reason;
                        }
                    }
                    AppEvent::Tick => {
                        app.tick();
                        app.expire_toasts();
                        if app.should_refresh() && !app.repos.is_empty() {
                            spawn_rescan(&root, max_depth, &excluded, &tx);
                            app.mark_refreshing();
                        }
                    }
                }
            }
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(exit_reason)
}

// ---------------------------------------------------------------------------
// Background result handling
// ---------------------------------------------------------------------------

fn handle_result(app: &mut AppState, result: RepoResult, tx: &mpsc::UnboundedSender<RepoResult>) {
    match result {
        RepoResult::ScanComplete(paths) => {
            app.set_repos(paths);
            if !app.repos.is_empty() {
                spawn_refresh_all(app, tx);
                app.mark_refreshing();
            }
        }
        RepoResult::RescanComplete(paths) => {
            app.reconcile_repos(paths);
            // Refresh all repos (existing + new)
            spawn_refresh_all(app, tx);
            // New repos need their pending count added
            app.pending_refreshes = app.repos.len();
        }
        RepoResult::RepoUpdated(path, info) => {
            if let Some(repo_name) = app.update_repo(&path, info) {
                send_notification(&repo_name);
            }
        }
        RepoResult::PullComplete(path, result) => {
            app.set_pull_result(&path, result);
            let refresh_tx = tx.clone();
            tokio::task::spawn_blocking(move || {
                let info = git_ops::get_repo_info(&path).map_err(|e| e.to_string());
                let _ = refresh_tx.send(RepoResult::RepoUpdated(path, info));
            });
        }
        RepoResult::DetailLoaded(path, details) => {
            if let Ok(details) = details {
                if app.selected_repo().map(|r| &r.path) == Some(&path) {
                    app.open_detail_pane(details);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Key handling — returns Some(ExitReason) if the TUI should exit
// ---------------------------------------------------------------------------

fn handle_key(
    app: &mut AppState,
    key: crossterm::event::KeyEvent,
    tx: &mpsc::UnboundedSender<RepoResult>,
) -> Option<ExitReason> {
    match app.input_mode {
        InputMode::Help => handle_key_help(app, key.code),
        InputMode::ErrorInfo => handle_key_error_info(app, key.code),
        InputMode::ThemePicker => handle_key_theme_picker(app, key.code),
        InputMode::Search => handle_key_search(app, key.code),
        InputMode::Normal => handle_key_normal(app, key.code, key.modifiers, tx),
    }
}

fn handle_key_help(app: &mut AppState, code: KeyCode) -> Option<ExitReason> {
    match code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => app.close_help(),
        KeyCode::Up | KeyCode::Char('k') => app.help_scroll = app.help_scroll.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => app.help_scroll += 1,
        _ => {}
    }
    None
}

fn handle_key_error_info(app: &mut AppState, code: KeyCode) -> Option<ExitReason> {
    match code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('i') | KeyCode::Enter => {
            app.close_error_info()
        }
        _ => {}
    }
    None
}

fn handle_key_theme_picker(app: &mut AppState, code: KeyCode) -> Option<ExitReason> {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => app.theme_picker_cancel(),
        KeyCode::Enter => app.theme_picker_confirm(),
        KeyCode::Up | KeyCode::Char('k') => app.theme_picker_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.theme_picker_next(),
        _ => {}
    }
    None
}

fn handle_key_search(app: &mut AppState, code: KeyCode) -> Option<ExitReason> {
    match code {
        KeyCode::Esc => {
            app.search_query.clear();
            app.input_mode = InputMode::Normal;
            app.recompute_filtered();
        }
        KeyCode::Enter => app.input_mode = InputMode::Normal,
        KeyCode::Backspace => {
            app.search_query.pop();
            app.recompute_filtered();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.recompute_filtered();
        }
        _ => {}
    }
    None
}

fn handle_key_normal(
    app: &mut AppState,
    code: KeyCode,
    modifiers: KeyModifiers,
    tx: &mpsc::UnboundedSender<RepoResult>,
) -> Option<ExitReason> {
    match (code, modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Some(ExitReason::Quit),
        (KeyCode::Char('b'), _) => return Some(ExitReason::BackToPicker),
        (KeyCode::Char('q'), _) => {
            if app.detail_pane.is_some() {
                app.close_detail_pane();
            } else {
                return Some(ExitReason::Quit);
            }
        }
        (KeyCode::Esc, _) => {
            if app.detail_pane.is_some() {
                app.close_detail_pane();
            } else if app.status_filter != app::StatusFilter::All || !app.search_query.is_empty() {
                app.status_filter = app::StatusFilter::All;
                app.search_query.clear();
                app.recompute_filtered();
            } else {
                return Some(ExitReason::Quit);
            }
        }

        // Modes
        (KeyCode::Char('?'), _) => app.open_help(),
        (KeyCode::Char('i'), _) => app.show_error_info(),
        (KeyCode::Char('/'), _) => {
            app.input_mode = InputMode::Search;
            app.search_query.clear();
        }
        (KeyCode::Char('t'), _) => app.open_theme_picker(),
        (KeyCode::Char('f'), _) => app.cycle_filter(),
        (KeyCode::Char('s'), _) => app.cycle_sort(),

        // Actions
        (KeyCode::Char('p'), _) => handle_pull(app, tx),
        (KeyCode::Char('P'), _) => handle_pull_all(app, tx),
        (KeyCode::Char('r'), _) => {
            spawn_refresh_all(app, tx);
            app.mark_refreshing();
            app.toast("Refreshing all repos...".into(), ToastLevel::Info);
        }
        (KeyCode::Enter, _) => {
            if app.detail_pane.is_some() {
                app.close_detail_pane();
            } else if let Some(repo) = app.selected_repo() {
                let path = repo.path.clone();
                let detail_tx = tx.clone();
                tokio::task::spawn_blocking(move || {
                    let result = git_ops::get_repo_details(&path).map_err(|e| e.to_string());
                    let _ = detail_tx.send(RepoResult::DetailLoaded(path, result));
                });
            }
        }

        // Navigation
        (KeyCode::Up | KeyCode::Char('k'), _) => {
            if let Some(ref mut pane) = app.detail_pane {
                pane.scroll_up();
            } else {
                app.select_prev();
            }
        }
        (KeyCode::Down | KeyCode::Char('j'), _) => {
            if let Some(ref mut pane) = app.detail_pane {
                pane.scroll_down();
            } else {
                app.select_next();
            }
        }
        _ => {}
    }
    None
}

fn handle_pull(app: &mut AppState, tx: &mpsc::UnboundedSender<RepoResult>) {
    let Some(repo) = app.selected_repo() else { return };

    if repo.pulling {
        app.toast("Already pulling...".into(), ToastLevel::Warning);
    } else if repo.status == git_ops::RepoStatus::Dirty {
        app.toast(format!("{}: dirty — commit or stash first", repo.display_name), ToastLevel::Warning);
    } else if repo.status == git_ops::RepoStatus::Conflicts {
        app.toast(format!("{}: has conflicts — resolve first", repo.display_name), ToastLevel::Error);
    } else if repo.behind == 0 {
        app.toast(format!("{}: already up to date", repo.display_name), ToastLevel::Info);
    } else if repo.error.is_some() {
        app.toast(format!("{}: repo has errors", repo.display_name), ToastLevel::Error);
    } else {
        let path = repo.path.clone();
        app.set_pulling(&path);
        let pull_tx = tx.clone();
        tokio::task::spawn_blocking(move || {
            let result = git_ops::pull_current_branch(&path).map_err(|e| e.to_string());
            let _ = pull_tx.send(RepoResult::PullComplete(path, result));
        });
    }
}

fn handle_pull_all(app: &mut AppState, tx: &mpsc::UnboundedSender<RepoResult>) {
    let pullable: Vec<PathBuf> = app
        .repos
        .iter()
        .filter(|r| {
            !r.pulling
                && r.status == git_ops::RepoStatus::Clean
                && r.behind > 0
                && r.error.is_none()
        })
        .map(|r| r.path.clone())
        .collect();

    if pullable.is_empty() {
        app.toast("No repos to pull".into(), ToastLevel::Info);
        return;
    }

    let count = pullable.len();
    for path in pullable {
        app.set_pulling(&path);
        let pull_tx = tx.clone();
        tokio::task::spawn_blocking(move || {
            let result = git_ops::pull_current_branch(&path).map_err(|e| e.to_string());
            let _ = pull_tx.send(RepoResult::PullComplete(path, result));
        });
    }
    app.toast(format!("Pulling {} repos...", count), ToastLevel::Info);
}

// ---------------------------------------------------------------------------
// Background tasks
// ---------------------------------------------------------------------------

fn spawn_rescan(
    root: &PathBuf,
    max_depth: usize,
    excluded: &[String],
    tx: &mpsc::UnboundedSender<RepoResult>,
) {
    let root = root.clone();
    let excluded = excluded.to_vec();
    let tx = tx.clone();
    tokio::task::spawn_blocking(move || {
        let paths = scanner::scan_repos(&root, max_depth, &excluded);
        let _ = tx.send(RepoResult::RescanComplete(paths));
    });
}

fn spawn_refresh_all(app: &AppState, tx: &mpsc::UnboundedSender<RepoResult>) {
    for repo in &app.repos {
        let path = repo.path.clone();
        let tx = tx.clone();
        tokio::task::spawn_blocking(move || {
            let _ = git_ops::fetch_all_remotes(&path);
            let result = git_ops::get_repo_info(&path).map_err(|e| e.to_string());
            let _ = tx.send(RepoResult::RepoUpdated(path, result));
        });
    }
}

fn send_notification(repo_name: &str) {
    let _ = notify_rust::Notification::new()
        .summary("gruth — repo behind")
        .body(&format!("{} has new commits to pull", repo_name))
        .icon("git")
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show();
}
