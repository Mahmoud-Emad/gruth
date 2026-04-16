//! TUI rendering — header, table, detail pane, footer, overlays.

use crate::app::{AppState, InputMode, StatusFilter, SortOrder, Toast, ToastLevel};
use crate::config::Theme;
use crate::git_ops::RepoStatus;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

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

    // Overlays (order matters — last drawn is on top)
    if let Some(toast) = app.active_toast() {
        draw_toast(f, app, toast);
    }
    match app.input_mode {
        InputMode::ThemePicker => draw_theme_picker(f, app),
        InputMode::Help => draw_help(f, app),
        InputMode::ErrorInfo => draw_error_info(f, app),
        _ => {}
    }
}

fn draw_header(f: &mut Frame, app: &AppState, area: Rect) {
    let t = &app.theme;
    let status = if app.is_refreshing() {
        Span::styled(
            format!("{} syncing", app.spinner()),
            Style::default().fg(t.dirty),
        )
    } else {
        Span::styled("● idle", Style::default().fg(t.clean))
    };

    let mut spans = vec![
        Span::styled(" gruth", Style::default().fg(t.accent).bold()),
        Span::styled(" │ ", Style::default().fg(t.border)),
        Span::styled("Git Repository UTility Helper", Style::default().fg(t.dim)),
        Span::styled(" │ ", Style::default().fg(t.border)),
        Span::styled(format!("v{}", env!("CARGO_PKG_VERSION")), Style::default().fg(t.dim)),
        Span::styled(" │ ", Style::default().fg(t.border)),
        status,
    ];

    if let Some(ref version) = app.update_available {
        let v = version.strip_prefix('v').unwrap_or(version);
        spans.push(Span::styled(" │ ", Style::default().fg(t.border)));
        spans.push(Span::styled(
            format!("↑ {} available", v),
            Style::default().fg(t.dirty).bold(),
        ));
    }

    let line = Line::from(spans);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border));

    f.render_widget(Paragraph::new(line).block(block), area);
}

/// Determine which columns to show based on terminal width.
struct ColumnLayout {
    show_branch: bool,
    show_sync: bool,
    show_commit: bool,
}

impl ColumnLayout {
    fn from_width(width: u16) -> Self {
        Self {
            show_branch: width >= 60,
            show_sync: width >= 80,
            show_commit: width >= 100,
        }
    }

    fn widths(&self) -> Vec<Constraint> {
        match (self.show_branch, self.show_sync, self.show_commit) {
            (true, true, true) => vec![
                Constraint::Percentage(35),
                Constraint::Percentage(18),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(17),
            ],
            (true, true, false) => vec![
                Constraint::Percentage(40),
                Constraint::Percentage(20),
                Constraint::Percentage(18),
                Constraint::Percentage(22),
            ],
            (true, false, false) => vec![
                Constraint::Percentage(50),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ],
            (false, false, false) => vec![
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ],
            // Edge cases — shouldn't happen but handle gracefully
            _ => vec![
                Constraint::Percentage(35),
                Constraint::Percentage(18),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(17),
            ],
        }
    }

    fn headers(&self) -> Vec<&'static str> {
        let mut h = vec!["  Repository"];
        if self.show_branch {
            h.push("Branch");
        }
        h.push("Status");
        if self.show_sync {
            h.push("Sync");
        }
        if self.show_commit {
            h.push("Last Commit");
        }
        h
    }
}

fn draw_table(f: &mut Frame, app: &AppState, area: Rect) {
    let t = &app.theme;
    let cols = ColumnLayout::from_width(area.width);

    if app.scanning {
        let msg = Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("{} Scanning for repositories...", app.spinner()),
                Style::default().fg(t.accent),
            ),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(t.border)));
        f.render_widget(msg, area);
        return;
    }

    if app.filtered_indices.is_empty() {
        let lines = if app.repos.is_empty() {
            vec![
                Line::from(vec![
                    Span::styled("  ✗ ", Style::default().fg(t.error)),
                    Span::styled("No git repositories found in this directory", Style::default().fg(t.dim)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("    Press ", Style::default().fg(t.dim)),
                    Span::styled("b", Style::default().fg(t.accent).bold()),
                    Span::styled(" to go back and pick another directory", Style::default().fg(t.dim)),
                ]),
            ]
        } else {
            vec![Line::from(vec![
                Span::styled("  ✗ ", Style::default().fg(t.error)),
                Span::styled("No repos match current filter", Style::default().fg(t.dim)),
            ])]
        };
        let widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(t.border)));
        f.render_widget(widget, area);
        return;
    }

    let header = Row::new(
        cols.headers()
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD))),
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

            let mut cells = Vec::new();

            // Repository name — always shown
            cells.push(
                Cell::from(format!("{}{}", prefix, repo.display_name)).style(
                    if selected {
                        Style::default().fg(Color::White).bold()
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
            );

            // Branch — hide on narrow
            if cols.show_branch {
                cells.push(Cell::from(repo.branch.clone()).style(Style::default().fg(t.accent)));
            }

            // Status — always shown
            let (status_text, status_color) = if repo.error.is_some() {
                ("✗ error", t.error)
            } else if repo.fetching {
                ("◌ ...", t.dim)
            } else {
                match repo.status {
                    RepoStatus::Clean => ("● clean", t.clean),
                    RepoStatus::Dirty => ("● dirty", t.dirty),
                    RepoStatus::Conflicts => ("✖ conflict", t.error),
                }
            };
            cells.push(Cell::from(status_text).style(Style::default().fg(status_color)));

            // Sync — hide on narrow
            if cols.show_sync {
                let sync = if repo.pulling {
                    Cell::from("↓ pulling...").style(Style::default().fg(t.dirty))
                } else if let Some(ref result) = repo.pull_result {
                    match result {
                        Ok(_) => Cell::from("✓ pulled").style(Style::default().fg(t.clean)),
                        Err(_) => Cell::from("✗ pull failed").style(Style::default().fg(t.error)),
                    }
                } else if repo.error.is_some() {
                    Cell::from("—").style(Style::default().fg(t.dim))
                } else if repo.fetching {
                    Cell::from("...").style(Style::default().fg(t.dim))
                } else {
                    sync_cell(repo.ahead, repo.behind, t)
                };
                cells.push(sync);
            }

            // Last Commit — hide on narrow
            if cols.show_commit {
                let age_color = if !repo.fetching && app.is_stale(repo) {
                    t.stale
                } else if repo.fetching {
                    t.dim
                } else {
                    Color::White
                };
                cells.push(
                    Cell::from(repo.last_commit_age.clone()).style(Style::default().fg(age_color)),
                );
            }

            let row = Row::new(cells);
            if selected {
                row.style(Style::default().bg(t.selected_bg))
            } else {
                row
            }
        })
        .collect();

    let table = Table::new(rows, cols.widths())
        .header(header)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(t.border)))
        .row_highlight_style(Style::default());

    let mut state = TableState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(table, area, &mut state);
}

fn sync_cell(ahead: usize, behind: usize, t: &Theme) -> Cell<'static> {
    if ahead == 0 && behind == 0 {
        return Cell::from("✓ synced").style(Style::default().fg(t.clean));
    }

    let mut spans = Vec::new();
    if ahead > 0 {
        spans.push(Span::styled(format!("↑{}", ahead), Style::default().fg(t.ahead).bold()));
    }
    if ahead > 0 && behind > 0 {
        spans.push(Span::raw(" "));
    }
    if behind > 0 {
        spans.push(Span::styled(format!("↓{}", behind), Style::default().fg(t.behind).bold()));
    }
    Cell::from(Line::from(spans))
}

fn draw_detail_pane(f: &mut Frame, app: &AppState, area: Rect) {
    let t = &app.theme;
    let detail = match &app.detail_pane {
        Some(d) => d,
        None => return,
    };

    let mut lines: Vec<Line> = Vec::new();

    // Commits
    lines.push(Line::from(Span::styled(
        " Recent Commits",
        Style::default().fg(t.accent).bold(),
    )));
    if detail.commits.is_empty() {
        lines.push(Line::from(Span::styled("   No commits", Style::default().fg(t.dim))));
    } else {
        for c in &detail.commits {
            lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(&c.date, Style::default().fg(t.dim)),
                Span::styled("  ", Style::default()),
                Span::styled(&c.author, Style::default().fg(t.dirty)),
                Span::styled("  ", Style::default()),
                Span::styled(&c.message, Style::default().fg(Color::White)),
            ]));
        }
    }

    lines.push(Line::from(""));

    // Changed Files
    lines.push(Line::from(Span::styled(
        " Changed Files",
        Style::default().fg(t.accent).bold(),
    )));
    if detail.changed_files.is_empty() {
        lines.push(Line::from(Span::styled(
            "   Working tree clean",
            Style::default().fg(t.clean),
        )));
    } else {
        for f_name in &detail.changed_files {
            let (prefix, color) = match f_name.chars().next() {
                Some('A') => ("A", t.clean),
                Some('M') => ("M", t.dirty),
                Some('D') => ("D", t.error),
                Some('R') => ("R", t.accent),
                _ => ("?", t.dim),
            };
            let path = f_name.get(2..).unwrap_or(f_name);
            lines.push(Line::from(vec![
                Span::styled(format!("   {} ", prefix), Style::default().fg(color)),
                Span::styled(path, Style::default().fg(Color::White)),
            ]));
        }
    }

    lines.push(Line::from(""));

    // Remotes
    lines.push(Line::from(Span::styled(
        " Remotes",
        Style::default().fg(t.accent).bold(),
    )));
    if detail.remote_urls.is_empty() {
        lines.push(Line::from(Span::styled("   No remotes", Style::default().fg(t.dim))));
    } else {
        for (name, url) in &detail.remote_urls {
            lines.push(Line::from(vec![
                Span::styled(format!("   {} ", name), Style::default().fg(t.dirty)),
                Span::styled(url, Style::default().fg(Color::White)),
            ]));
        }
    }

    lines.push(Line::from(""));

    // Branches
    lines.push(Line::from(Span::styled(
        " Branches",
        Style::default().fg(t.accent).bold(),
    )));
    if detail.branches.is_empty() {
        lines.push(Line::from(Span::styled("   No branches", Style::default().fg(t.dim))));
    } else {
        for b in &detail.branches {
            let mut spans = vec![Span::styled("   ", Style::default())];

            if b.is_head {
                spans.push(Span::styled("* ", Style::default().fg(t.clean).bold()));
            } else {
                spans.push(Span::styled("  ", Style::default()));
            }

            spans.push(Span::styled(&b.name, Style::default().fg(Color::White)));

            if b.upstream_gone {
                spans.push(Span::styled(" ⚠ upstream gone", Style::default().fg(t.dirty)));
            }
            if b.is_merged {
                spans.push(Span::styled(" (merged)", Style::default().fg(t.dim)));
            }

            lines.push(Line::from(spans));
        }
    }

    let title = format!(" {} ", detail.display_name);
    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(t.accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((detail.scroll as u16, 0));

    f.render_widget(paragraph, area);
}

fn draw_footer(f: &mut Frame, app: &AppState, area: Rect) {
    let t = &app.theme;
    let mut spans: Vec<Span> = Vec::new();

    // Search mode
    if app.input_mode == InputMode::Search {
        spans.push(Span::styled(
            format!("  /{}", app.search_query),
            Style::default().fg(t.accent).bold(),
        ));
        spans.push(Span::styled("▌ ", Style::default().fg(t.accent)));
        spans.push(Span::styled("│", Style::default().fg(t.border)));
    }

    // Counts
    let count_label = if app.filtered_indices.len() == app.total_count() {
        format!("  {} repos ", app.repo_count())
    } else {
        format!("  {}/{} repos ", app.repo_count(), app.total_count())
    };
    spans.push(Span::styled(count_label, Style::default().fg(Color::White).bold()));
    spans.push(Span::styled("│", Style::default().fg(t.border)));
    spans.push(Span::styled(format!(" ● {} ", app.clean_count()), Style::default().fg(t.clean)));
    spans.push(Span::styled(format!("● {} ", app.dirty_count()), Style::default().fg(t.dirty)));

    if app.error_count() > 0 {
        spans.push(Span::styled(format!("✗ {} ", app.error_count()), Style::default().fg(t.error)));
    }
    if app.stale_count() > 0 {
        spans.push(Span::styled(
            format!("⏳{} ", app.stale_count()),
            Style::default().fg(t.stale),
        ));
    }

    spans.push(Span::styled("│", Style::default().fg(t.border)));

    // Filter/sort indicators
    if app.status_filter != StatusFilter::All {
        spans.push(Span::styled(
            format!(" [{}]", app.status_filter.label()),
            Style::default().fg(t.dirty),
        ));
    }
    if app.sort_order != SortOrder::Name {
        spans.push(Span::styled(
            format!(" [↕{}]", app.sort_order.label()),
            Style::default().fg(t.accent),
        ));
    }
    if app.status_filter != StatusFilter::All || app.sort_order != SortOrder::Name {
        spans.push(Span::styled(" │", Style::default().fg(t.border)));
    }

    // Keybinds
    if app.input_mode == InputMode::Search {
        spans.push(Span::styled(" esc", Style::default().fg(t.accent).bold()));
        spans.push(Span::styled(" cancel ", Style::default().fg(t.dim)));
        spans.push(Span::styled("⏎", Style::default().fg(t.accent).bold()));
        spans.push(Span::styled(" confirm", Style::default().fg(t.dim)));
    } else {
        spans.push(Span::styled(" q", Style::default().fg(t.accent).bold()));
        spans.push(Span::styled(" quit ", Style::default().fg(t.dim)));
        spans.push(Span::styled("/", Style::default().fg(t.accent).bold()));
        spans.push(Span::styled(" search ", Style::default().fg(t.dim)));
        spans.push(Span::styled("f", Style::default().fg(t.accent).bold()));
        spans.push(Span::styled(" filter ", Style::default().fg(t.dim)));
        spans.push(Span::styled("p", Style::default().fg(t.accent).bold()));
        spans.push(Span::styled(" pull ", Style::default().fg(t.dim)));
        spans.push(Span::styled("⏎", Style::default().fg(t.accent).bold()));
        spans.push(Span::styled(" detail ", Style::default().fg(t.dim)));
        spans.push(Span::styled("?", Style::default().fg(t.accent).bold()));
        spans.push(Span::styled(" help", Style::default().fg(t.dim)));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border));

    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}

fn draw_theme_picker(f: &mut Frame, app: &AppState) {
    let t = &app.theme;
    let presets = Theme::presets();

    // Center the overlay
    let area = f.area();
    let picker_width = 40u16.min(area.width.saturating_sub(4));
    let picker_height = (presets.len() as u16 + 5).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(picker_width)) / 2;
    let y = (area.height.saturating_sub(picker_height)) / 2;
    let picker_area = Rect::new(x, y, picker_width, picker_height);

    // Clear the area behind the overlay
    f.render_widget(Clear, picker_area);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));

    for (i, preset) in presets.iter().enumerate() {
        let selected = i == app.theme_picker_index;
        let prefix = if selected { " ▸ " } else { "   " };

        let name_style = if selected {
            Style::default().fg(Color::White).bold()
        } else {
            Style::default().fg(Color::White)
        };

        // Color preview swatches
        let mut spans = vec![
            Span::styled(prefix, Style::default().fg(t.accent)),
            Span::styled(preset.name, name_style),
        ];

        // Pad to align swatches
        let pad = 18usize.saturating_sub(preset.name.len());
        spans.push(Span::raw(" ".repeat(pad)));

        // Color dots showing the theme's palette
        spans.push(Span::styled("●", Style::default().fg(preset.theme.accent)));
        spans.push(Span::styled("●", Style::default().fg(preset.theme.clean)));
        spans.push(Span::styled("●", Style::default().fg(preset.theme.dirty)));
        spans.push(Span::styled("●", Style::default().fg(preset.theme.error)));
        spans.push(Span::styled("●", Style::default().fg(preset.theme.behind)));

        let line = Line::from(spans);
        lines.push(if selected {
            line.style(Style::default().bg(t.selected_bg))
        } else {
            line
        });
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("   ⏎", Style::default().fg(t.accent).bold()),
        Span::styled(" select  ", Style::default().fg(t.dim)),
        Span::styled("esc", Style::default().fg(t.accent).bold()),
        Span::styled(" cancel", Style::default().fg(t.dim)),
    ]));

    let block = Block::default()
        .title(" Theme ")
        .title_style(Style::default().fg(t.accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));

    let widget = Paragraph::new(lines).block(block);
    f.render_widget(widget, picker_area);
}

fn draw_toast(f: &mut Frame, app: &AppState, toast: &Toast) {
    let t = &app.theme;
    let area = f.area();

    let (icon, border_color) = match toast.level {
        ToastLevel::Info => ("ℹ", t.accent),
        ToastLevel::Success => ("✓", t.clean),
        ToastLevel::Warning => ("⚠", t.dirty),
        ToastLevel::Error => ("✗", t.error),
    };

    let msg = format!(" {} {} ", icon, toast.message);
    let width = (msg.len() as u16 + 4).min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = area.height.saturating_sub(5); // just above the footer

    let toast_area = Rect::new(x, y, width, 3);

    f.render_widget(Clear, toast_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let line = Line::from(vec![
        Span::styled(format!(" {} ", icon), Style::default().fg(border_color).bold()),
        Span::styled(&toast.message, Style::default().fg(Color::White)),
    ]);

    f.render_widget(Paragraph::new(line).block(block), toast_area);
}

fn draw_help(f: &mut Frame, app: &AppState) {
    let t = &app.theme;
    let area = f.area();

    let sections: Vec<(&str, Vec<(&str, &str)>)> = vec![
        ("Navigation", vec![
            ("↑ / k", "Move up"),
            ("↓ / j", "Move down"),
            ("Enter", "Open / close detail pane"),
            ("b", "Back to directory picker"),
        ]),
        ("Actions", vec![
            ("p", "Pull selected repo (clean + behind only)"),
            ("P", "Pull all eligible repos"),
            ("r", "Force refresh all repos"),
            ("i", "Show error details for selected repo"),
        ]),
        ("Search & Filter", vec![
            ("/", "Search repos by name"),
            ("f", "Cycle filter (all/clean/dirty/behind/ahead/errors/stale)"),
            ("s", "Cycle sort (name/status/commit/behind)"),
            ("Esc", "Clear filter/search, close pane, or quit"),
        ]),
        ("Appearance", vec![
            ("t", "Open theme picker"),
        ]),
        ("General", vec![
            ("q", "Quit (or close active pane)"),
            ("Ctrl+C", "Force quit"),
            ("?", "Toggle this help screen"),
        ]),
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (section, keys) in &sections {
        lines.push(Line::from(Span::styled(
            format!("  {}", section),
            Style::default().fg(t.accent).bold(),
        )));
        lines.push(Line::from(""));

        for (key, desc) in keys {
            let pad = 14usize.saturating_sub(key.len());
            lines.push(Line::from(vec![
                Span::styled(format!("    {}", key), Style::default().fg(Color::White).bold()),
                Span::raw(" ".repeat(pad)),
                Span::styled(*desc, Style::default().fg(t.dim)),
            ]));
        }

        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("    Press ", Style::default().fg(t.dim)),
        Span::styled("?", Style::default().fg(t.accent).bold()),
        Span::styled(" or ", Style::default().fg(t.dim)),
        Span::styled("Esc", Style::default().fg(t.accent).bold()),
        Span::styled(" to close", Style::default().fg(t.dim)),
    ]));

    let picker_width = 60u16.min(area.width.saturating_sub(4));
    let picker_height = (lines.len() as u16 + 2).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(picker_width)) / 2;
    let y = (area.height.saturating_sub(picker_height)) / 2;
    let help_area = Rect::new(x, y, picker_width, picker_height);

    f.render_widget(Clear, help_area);

    let block = Block::default()
        .title(" Keyboard Shortcuts ")
        .title_style(Style::default().fg(t.accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));

    let widget = Paragraph::new(lines)
        .block(block)
        .scroll((app.help_scroll as u16, 0));

    f.render_widget(widget, help_area);
}

fn draw_error_info(f: &mut Frame, app: &AppState) {
    let t = &app.theme;
    let area = f.area();

    let error_text = match &app.error_info_text {
        Some(text) => text.clone(),
        None => return,
    };

    let repo_name = app
        .selected_repo()
        .map(|r| r.display_name.clone())
        .unwrap_or_default();

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Repo: ", Style::default().fg(t.dim)),
        Span::styled(&repo_name, Style::default().fg(Color::White).bold()),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Error:",
        Style::default().fg(t.error).bold(),
    )));
    lines.push(Line::from(""));

    // Wrap error text into lines
    for line in error_text.lines() {
        lines.push(Line::from(Span::styled(
            format!("  {}", line),
            Style::default().fg(Color::White),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Press ", Style::default().fg(t.dim)),
        Span::styled("Esc", Style::default().fg(t.accent).bold()),
        Span::styled(" to close", Style::default().fg(t.dim)),
    ]));

    let picker_width = 60u16.min(area.width.saturating_sub(4));
    let picker_height = (lines.len() as u16 + 2).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(picker_width)) / 2;
    let y = (area.height.saturating_sub(picker_height)) / 2;
    let error_area = Rect::new(x, y, picker_width, picker_height);

    f.render_widget(Clear, error_area);

    let block = Block::default()
        .title(" Error Info ")
        .title_style(Style::default().fg(t.error).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.error));

    let widget = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(widget, error_area);
}
