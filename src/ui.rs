use crate::app::{AppState, InputMode, StatusFilter, SortOrder};
use crate::git_ops::RepoStatus;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

const CYAN: Color = Color::Cyan;
const BORDER: Color = Color::DarkGray;
const GREEN: Color = Color::Green;
const YELLOW: Color = Color::Yellow;
const RED: Color = Color::Red;
const MAGENTA: Color = Color::Magenta;
const DIM: Color = Color::DarkGray;
const SELECTED_BG: Color = Color::Rgb(30, 30, 50);
const STALE_COLOR: Color = Color::Rgb(180, 60, 60);

pub fn draw(f: &mut Frame, app: &AppState) {
    let has_detail = app.detail_pane.is_some();

    let chunks = if has_detail {
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Percentage(45),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(f.area())
    } else {
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area())
    };

    draw_header(f, app, chunks[0]);

    if has_detail {
        draw_table(f, app, chunks[1]);
        draw_detail_pane(f, app, chunks[2]);
        draw_footer(f, app, chunks[3]);
    } else {
        draw_table(f, app, chunks[1]);
        draw_footer(f, app, chunks[2]);
    }
}

fn draw_header(f: &mut Frame, app: &AppState, area: Rect) {
    let status = if app.is_refreshing() {
        Span::styled(
            format!("{} syncing", app.spinner()),
            Style::default().fg(YELLOW),
        )
    } else {
        Span::styled("● idle", Style::default().fg(GREEN))
    };

    let line = Line::from(vec![
        Span::styled(" gruth", Style::default().fg(CYAN).bold()),
        Span::styled(" │ ", Style::default().fg(BORDER)),
        Span::styled("git repo monitor", Style::default().fg(DIM)),
        Span::styled(" │ ", Style::default().fg(BORDER)),
        Span::styled(format!("v{}", env!("CARGO_PKG_VERSION")), Style::default().fg(DIM)),
        Span::styled(" │ ", Style::default().fg(BORDER)),
        status,
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER));

    f.render_widget(Paragraph::new(line).block(block), area);
}

fn draw_table(f: &mut Frame, app: &AppState, area: Rect) {
    if app.scanning {
        let msg = Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("{} Scanning for repositories...", app.spinner()),
                Style::default().fg(CYAN),
            ),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER)));
        f.render_widget(msg, area);
        return;
    }

    if app.filtered_indices.is_empty() {
        let msg = if app.repos.is_empty() {
            "No git repositories found"
        } else {
            "No repos match current filter"
        };
        let widget = Paragraph::new(Line::from(vec![
            Span::styled("  ✗ ", Style::default().fg(RED)),
            Span::styled(msg, Style::default().fg(DIM)),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER)));
        f.render_widget(widget, area);
        return;
    }

    let header = Row::new(
        ["  Repository", "Branch", "Status", "Sync", "Commit", "Br"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))),
    )
    .height(1);

    let rows: Vec<Row> = app
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(i, &repo_idx)| {
            let repo = &app.repos[repo_idx];
            let selected = i == app.selected;
            let prefix = if selected { "▸ " } else { "  " };

            let name = Cell::from(format!("{}{}", prefix, repo.display_name)).style(
                if selected {
                    Style::default().fg(Color::White).bold()
                } else {
                    Style::default().fg(Color::White)
                },
            );

            let branch = Cell::from(repo.branch.clone()).style(Style::default().fg(CYAN));

            let (status_text, status_color) = if repo.error.is_some() {
                ("✗ error", RED)
            } else if repo.fetching {
                ("◌ ...", DIM)
            } else {
                match repo.status {
                    RepoStatus::Clean => ("● clean", GREEN),
                    RepoStatus::Dirty => ("● dirty", YELLOW),
                    RepoStatus::Conflicts => ("✖ conflict", RED),
                }
            };
            let status = Cell::from(status_text).style(Style::default().fg(status_color));

            let sync = if repo.error.is_some() {
                Cell::from("—").style(Style::default().fg(DIM))
            } else if repo.fetching {
                Cell::from("...").style(Style::default().fg(DIM))
            } else {
                sync_cell(repo.ahead, repo.behind)
            };

            let age_color = if !repo.fetching && app.is_stale(repo) {
                STALE_COLOR
            } else if repo.fetching {
                DIM
            } else {
                Color::White
            };
            let age = Cell::from(repo.last_commit_age.clone()).style(Style::default().fg(age_color));

            let branches = Cell::from(if repo.fetching {
                "...".to_string()
            } else {
                format!("{} br", repo.branch_count)
            })
            .style(Style::default().fg(if repo.fetching { DIM } else { Color::White }));

            let row = Row::new(vec![name, branch, status, sync, age, branches]);
            if selected {
                row.style(Style::default().bg(SELECTED_BG))
            } else {
                row
            }
        })
        .collect();

    let widths = [
        Constraint::Percentage(30),
        Constraint::Percentage(15),
        Constraint::Percentage(13),
        Constraint::Percentage(12),
        Constraint::Percentage(15),
        Constraint::Percentage(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER)))
        .row_highlight_style(Style::default());

    let mut state = TableState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(table, area, &mut state);
}

fn sync_cell(ahead: usize, behind: usize) -> Cell<'static> {
    if ahead == 0 && behind == 0 {
        return Cell::from("✓ synced").style(Style::default().fg(GREEN));
    }

    let mut spans = Vec::new();
    if ahead > 0 {
        spans.push(Span::styled(format!("↑{}", ahead), Style::default().fg(CYAN).bold()));
    }
    if ahead > 0 && behind > 0 {
        spans.push(Span::raw(" "));
    }
    if behind > 0 {
        spans.push(Span::styled(format!("↓{}", behind), Style::default().fg(MAGENTA).bold()));
    }
    Cell::from(Line::from(spans))
}

fn draw_detail_pane(f: &mut Frame, app: &AppState, area: Rect) {
    let detail = match &app.detail_pane {
        Some(d) => d,
        None => return,
    };

    let mut lines: Vec<Line> = Vec::new();

    // --- Commits ---
    lines.push(Line::from(Span::styled(
        " Recent Commits",
        Style::default().fg(CYAN).bold(),
    )));
    if detail.commits.is_empty() {
        lines.push(Line::from(Span::styled("   No commits", Style::default().fg(DIM))));
    } else {
        for c in &detail.commits {
            lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(&c.date, Style::default().fg(DIM)),
                Span::styled("  ", Style::default()),
                Span::styled(&c.author, Style::default().fg(YELLOW)),
                Span::styled("  ", Style::default()),
                Span::styled(&c.message, Style::default().fg(Color::White)),
            ]));
        }
    }

    lines.push(Line::from(""));

    // --- Changed Files ---
    lines.push(Line::from(Span::styled(
        " Changed Files",
        Style::default().fg(CYAN).bold(),
    )));
    if detail.changed_files.is_empty() {
        lines.push(Line::from(Span::styled(
            "   Working tree clean",
            Style::default().fg(GREEN),
        )));
    } else {
        for f_name in &detail.changed_files {
            let (prefix, color) = match f_name.chars().next() {
                Some('A') => ("A", GREEN),
                Some('M') => ("M", YELLOW),
                Some('D') => ("D", RED),
                Some('R') => ("R", CYAN),
                _ => ("?", DIM),
            };
            let path = f_name.get(2..).unwrap_or(f_name);
            lines.push(Line::from(vec![
                Span::styled(format!("   {} ", prefix), Style::default().fg(color)),
                Span::styled(path, Style::default().fg(Color::White)),
            ]));
        }
    }

    lines.push(Line::from(""));

    // --- Remotes ---
    lines.push(Line::from(Span::styled(
        " Remotes",
        Style::default().fg(CYAN).bold(),
    )));
    if detail.remote_urls.is_empty() {
        lines.push(Line::from(Span::styled("   No remotes", Style::default().fg(DIM))));
    } else {
        for (name, url) in &detail.remote_urls {
            lines.push(Line::from(vec![
                Span::styled(format!("   {} ", name), Style::default().fg(YELLOW)),
                Span::styled(url, Style::default().fg(Color::White)),
            ]));
        }
    }

    lines.push(Line::from(""));

    // --- Branches ---
    lines.push(Line::from(Span::styled(
        " Branches",
        Style::default().fg(CYAN).bold(),
    )));
    if detail.branches.is_empty() {
        lines.push(Line::from(Span::styled("   No branches", Style::default().fg(DIM))));
    } else {
        for b in &detail.branches {
            let mut spans = vec![Span::styled("   ", Style::default())];

            if b.is_head {
                spans.push(Span::styled("* ", Style::default().fg(GREEN).bold()));
            } else {
                spans.push(Span::styled("  ", Style::default()));
            }

            spans.push(Span::styled(&b.name, Style::default().fg(Color::White)));

            if b.upstream_gone {
                spans.push(Span::styled(" ⚠ upstream gone", Style::default().fg(YELLOW)));
            }
            if b.is_merged {
                spans.push(Span::styled(" (merged)", Style::default().fg(DIM)));
            }

            lines.push(Line::from(spans));
        }
    }

    let title = format!(" {} ", detail.display_name);
    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(CYAN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((detail.scroll as u16, 0));

    f.render_widget(paragraph, area);
}

fn draw_footer(f: &mut Frame, app: &AppState, area: Rect) {
    let mut spans: Vec<Span> = Vec::new();

    // Search mode indicator
    if app.input_mode == InputMode::Search {
        spans.push(Span::styled(
            format!("  /{}", app.search_query),
            Style::default().fg(CYAN).bold(),
        ));
        spans.push(Span::styled("▌ ", Style::default().fg(CYAN)));
        spans.push(Span::styled("│", Style::default().fg(BORDER)));
    }

    // Repo counts
    let count_label = if app.filtered_indices.len() == app.total_count() {
        format!("  {} repos ", app.repo_count())
    } else {
        format!("  {}/{} repos ", app.repo_count(), app.total_count())
    };
    spans.push(Span::styled(count_label, Style::default().fg(Color::White).bold()));
    spans.push(Span::styled("│", Style::default().fg(BORDER)));
    spans.push(Span::styled(format!(" ● {} ", app.clean_count()), Style::default().fg(GREEN)));
    spans.push(Span::styled(format!("● {} ", app.dirty_count()), Style::default().fg(YELLOW)));

    if app.error_count() > 0 {
        spans.push(Span::styled(format!("✗ {} ", app.error_count()), Style::default().fg(RED)));
    }
    if app.stale_count() > 0 {
        spans.push(Span::styled(
            format!("⏳{} ", app.stale_count()),
            Style::default().fg(STALE_COLOR),
        ));
    }

    spans.push(Span::styled("│", Style::default().fg(BORDER)));

    // Filter indicator
    if app.status_filter != StatusFilter::All {
        spans.push(Span::styled(
            format!(" [{}]", app.status_filter.label()),
            Style::default().fg(YELLOW),
        ));
    }

    // Sort indicator
    if app.sort_order != SortOrder::Name {
        spans.push(Span::styled(
            format!(" [↕{}]", app.sort_order.label()),
            Style::default().fg(CYAN),
        ));
    }

    if app.status_filter != StatusFilter::All || app.sort_order != SortOrder::Name {
        spans.push(Span::styled(" │", Style::default().fg(BORDER)));
    }

    // Keybinds
    if app.input_mode == InputMode::Search {
        spans.push(Span::styled(" esc", Style::default().fg(CYAN).bold()));
        spans.push(Span::styled(" cancel ", Style::default().fg(DIM)));
        spans.push(Span::styled("⏎", Style::default().fg(CYAN).bold()));
        spans.push(Span::styled(" confirm", Style::default().fg(DIM)));
    } else {
        spans.push(Span::styled(" q", Style::default().fg(CYAN).bold()));
        spans.push(Span::styled(" quit ", Style::default().fg(DIM)));
        spans.push(Span::styled("/", Style::default().fg(CYAN).bold()));
        spans.push(Span::styled(" search ", Style::default().fg(DIM)));
        spans.push(Span::styled("f", Style::default().fg(CYAN).bold()));
        spans.push(Span::styled(" filter ", Style::default().fg(DIM)));
        spans.push(Span::styled("s", Style::default().fg(CYAN).bold()));
        spans.push(Span::styled(" sort ", Style::default().fg(DIM)));
        spans.push(Span::styled("⏎", Style::default().fg(CYAN).bold()));
        spans.push(Span::styled(" detail", Style::default().fg(DIM)));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER));

    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}
