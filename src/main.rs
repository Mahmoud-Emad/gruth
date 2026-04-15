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
use app::{AppState, InputMode, SortOrder};
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

/// Why the TUI exited.
enum ExitReason {
    Quit,
    BackToPicker,
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

    if args.sync {
        let root = match args.path {
            Some(p) => p.canonicalize().unwrap_or(p),
            None => std::env::current_dir()?,
        };
        return sync::run_sync(&root, depth, &excluded);
    }

    // If --path was given, run TUI directly (no picker loop)
    if let Some(p) = args.path {
        let root = p.canonicalize().unwrap_or(p);
        run_tui(root, depth, interval_secs, stale_days, default_sort.clone(), excluded, theme.clone(), notifications).await?;
        return Ok(());
    }

    // Picker loop: picker → TUI → back to picker if user presses 'b'
    loop {
        let root = match dir_picker::run_picker()? {
            Some(p) => p,
            None => return Ok(()),
        };

        let reason = run_tui(
            root,
            depth,
            interval_secs,
            stale_days,
            default_sort.clone(),
            excluded.clone(),
            theme.clone(),
            notifications,
        )
        .await?;

        match reason {
            ExitReason::Quit => return Ok(()),
            ExitReason::BackToPicker => continue,
        }
    }
}

enum RepoResult {
    ScanComplete(Vec<PathBuf>),
    RepoUpdated(PathBuf, Result<git_ops::GitInfo, String>),
    PullComplete(PathBuf, Result<String, String>),
    DetailLoaded(PathBuf, Result<git_ops::RepoDetails, String>),
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
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let interval = Duration::from_secs(interval_secs);
    let mut app = AppState::new(root.clone(), interval, stale_days, default_sort, theme, notifications);
    let mut events = EventHandler::new(Duration::from_millis(250));
    let (tx, mut rx) = mpsc::unbounded_channel::<RepoResult>();

    // Initial scan
    let scan_tx = tx.clone();
    let scan_root = root.clone();
    tokio::task::spawn_blocking(move || {
        let repos = scanner::scan_repos(&scan_root, max_depth, &excluded);
        let _ = scan_tx.send(RepoResult::ScanComplete(repos));
    });

    let mut exit_reason = ExitReason::Quit;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        // Drain background results
        while let Ok(result) = rx.try_recv() {
            match result {
                RepoResult::ScanComplete(paths) => {
                    app.set_repos(paths);
                    if app.repos.is_empty() {
                        // No repos found — allow going back
                    } else {
                        spawn_refresh_all(&app, &tx);
                        app.mark_refreshing();
                    }
                }
                RepoResult::RepoUpdated(path, info) => {
                    if let Some(repo_name) = app.update_repo(&path, info) {
                        send_notification(&repo_name);
                    }
                }
                RepoResult::PullComplete(path, result) => {
                    app.set_pull_result(&path, result);
                    // Refresh the repo to get updated status
                    let refresh_tx = tx.clone();
                    let refresh_path = path.clone();
                    tokio::task::spawn_blocking(move || {
                        let info = git_ops::get_repo_info(&refresh_path).map_err(|e| e.to_string());
                        let _ = refresh_tx.send(RepoResult::RepoUpdated(refresh_path, info));
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

        // Handle events
        tokio::select! {
            event = events.next() => {
                match event? {
                    AppEvent::Key(key) => {
                        match app.input_mode {
                            InputMode::ThemePicker => match key.code {
                                KeyCode::Esc | KeyCode::Char('q') => {
                                    app.theme_picker_cancel();
                                }
                                KeyCode::Enter => {
                                    app.theme_picker_confirm();
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    app.theme_picker_prev();
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    app.theme_picker_next();
                                }
                                _ => {}
                            },
                            InputMode::Search => match key.code {
                                KeyCode::Esc => {
                                    app.search_query.clear();
                                    app.input_mode = InputMode::Normal;
                                    app.recompute_filtered();
                                }
                                KeyCode::Enter => {
                                    app.input_mode = InputMode::Normal;
                                }
                                KeyCode::Backspace => {
                                    app.search_query.pop();
                                    app.recompute_filtered();
                                }
                                KeyCode::Char(c) => {
                                    app.search_query.push(c);
                                    app.recompute_filtered();
                                }
                                _ => {}
                            },
                            InputMode::Normal => match (key.code, key.modifiers) {
                                (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                                (KeyCode::Char('b'), _) => {
                                    exit_reason = ExitReason::BackToPicker;
                                    break;
                                }
                                (KeyCode::Char('q'), _) => {
                                    if app.detail_pane.is_some() {
                                        app.close_detail_pane();
                                    } else {
                                        break;
                                    }
                                }
                                (KeyCode::Esc, _) => {
                                    if app.detail_pane.is_some() {
                                        app.close_detail_pane();
                                    } else if app.status_filter != app::StatusFilter::All
                                        || !app.search_query.is_empty()
                                    {
                                        app.status_filter = app::StatusFilter::All;
                                        app.search_query.clear();
                                        app.recompute_filtered();
                                    } else {
                                        break;
                                    }
                                }
                                (KeyCode::Char('/'), _) => {
                                    app.input_mode = InputMode::Search;
                                    app.search_query.clear();
                                }
                                (KeyCode::Char('t'), _) => app.open_theme_picker(),
                                (KeyCode::Char('f'), _) => app.cycle_filter(),
                                (KeyCode::Char('s'), _) => app.cycle_sort(),
                                (KeyCode::Char('p'), _) => {
                                    if let Some(repo) = app.selected_repo() {
                                        if repo.pulling {
                                            app.toast("Already pulling...".to_string(), app::ToastLevel::Warning);
                                        } else if repo.status == git_ops::RepoStatus::Dirty {
                                            app.toast(
                                                format!("{}: dirty — commit or stash first", repo.display_name),
                                                app::ToastLevel::Warning,
                                            );
                                        } else if repo.status == git_ops::RepoStatus::Conflicts {
                                            app.toast(
                                                format!("{}: has conflicts — resolve first", repo.display_name),
                                                app::ToastLevel::Error,
                                            );
                                        } else if repo.behind == 0 {
                                            app.toast(
                                                format!("{}: already up to date", repo.display_name),
                                                app::ToastLevel::Info,
                                            );
                                        } else if repo.error.is_some() {
                                            app.toast(
                                                format!("{}: repo has errors", repo.display_name),
                                                app::ToastLevel::Error,
                                            );
                                        } else {
                                            let path = repo.path.clone();
                                            app.set_pulling(&path);
                                            let pull_tx = tx.clone();
                                            tokio::task::spawn_blocking(move || {
                                                let result = git_ops::pull_current_branch(&path)
                                                    .map_err(|e| e.to_string());
                                                let _ = pull_tx.send(RepoResult::PullComplete(path, result));
                                            });
                                        }
                                    }
                                }
                                (KeyCode::Char('r'), _) => {
                                    spawn_refresh_all(&app, &tx);
                                    app.mark_refreshing();
                                    app.toast("Refreshing all repos...".to_string(), app::ToastLevel::Info);
                                }
                                (KeyCode::Enter, _) => {
                                    if app.detail_pane.is_some() {
                                        app.close_detail_pane();
                                    } else if let Some(repo) = app.selected_repo() {
                                        let path = repo.path.clone();
                                        let detail_tx = tx.clone();
                                        tokio::task::spawn_blocking(move || {
                                            let result = git_ops::get_repo_details(&path)
                                                .map_err(|e| e.to_string());
                                            let _ = detail_tx
                                                .send(RepoResult::DetailLoaded(path, result));
                                        });
                                    }
                                }
                                (KeyCode::Up | KeyCode::Char('k'), _) => {
                                    if app.detail_pane.is_some() {
                                        if let Some(ref mut pane) = app.detail_pane {
                                            pane.scroll_up();
                                        }
                                    } else {
                                        app.select_prev();
                                    }
                                }
                                (KeyCode::Down | KeyCode::Char('j'), _) => {
                                    if app.detail_pane.is_some() {
                                        if let Some(ref mut pane) = app.detail_pane {
                                            pane.scroll_down();
                                        }
                                    } else {
                                        app.select_next();
                                    }
                                }
                                _ => {}
                            },
                        }
                    }
                    AppEvent::Tick => {
                        app.tick();
                        app.expire_toasts();
                        if app.should_refresh() && !app.repos.is_empty() {
                            spawn_refresh_all(&app, &tx);
                            app.mark_refreshing();
                        }
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(exit_reason)
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
