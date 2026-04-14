mod app;
mod cli;
mod config;
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = config::Config::load();

    let depth = args.depth.or(config.depth).unwrap_or(10);
    let interval_secs = args.interval.or(config.interval).unwrap_or(5);
    let stale_days = args.stale_days.or(config.stale_days).unwrap_or(30);
    let default_sort = SortOrder::from_str(config.default_sort.as_deref().unwrap_or("name"));
    let excluded = config.excluded_paths.unwrap_or_default();

    let root = args.path.canonicalize().unwrap_or(args.path.clone());

    if args.sync {
        return sync::run_sync(&root, depth, &excluded);
    }

    run_tui(root, depth, interval_secs, stale_days, default_sort, excluded).await
}

enum RepoResult {
    ScanComplete(Vec<PathBuf>),
    RepoUpdated(PathBuf, Result<git_ops::GitInfo, String>),
    DetailLoaded(PathBuf, Result<git_ops::RepoDetails, String>),
}

async fn run_tui(
    root: PathBuf,
    max_depth: usize,
    interval_secs: u64,
    stale_days: u64,
    default_sort: SortOrder,
    excluded: Vec<String>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let interval = Duration::from_secs(interval_secs);
    let mut app = AppState::new(root.clone(), interval, stale_days, default_sort);
    let mut events = EventHandler::new(Duration::from_millis(250));
    let (tx, mut rx) = mpsc::unbounded_channel::<RepoResult>();

    // Initial scan
    let scan_tx = tx.clone();
    let scan_root = root.clone();
    tokio::task::spawn_blocking(move || {
        let repos = scanner::scan_repos(&scan_root, max_depth, &excluded);
        let _ = scan_tx.send(RepoResult::ScanComplete(repos));
    });

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        // Drain background results
        while let Ok(result) = rx.try_recv() {
            match result {
                RepoResult::ScanComplete(paths) => {
                    app.set_repos(paths);
                    spawn_refresh_all(&app, &tx);
                    app.mark_refreshing();
                }
                RepoResult::RepoUpdated(path, info) => {
                    app.update_repo(&path, info);
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
                                (KeyCode::Char('f'), _) => app.cycle_filter(),
                                (KeyCode::Char('s'), _) => app.cycle_sort(),
                                (KeyCode::Char('r'), _) => {
                                    spawn_refresh_all(&app, &tx);
                                    app.mark_refreshing();
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
                        if app.should_refresh() {
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
    Ok(())
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
