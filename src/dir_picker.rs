//! Interactive directory picker — browse the filesystem to select a root directory.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Cell, Table, TableState},
    Frame,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const MAX_ENTRIES: usize = 500;

const CYAN: Color = Color::Cyan;
const BORDER: Color = Color::DarkGray;
const GREEN: Color = Color::Green;
const YELLOW: Color = Color::Yellow;
const DIM: Color = Color::DarkGray;
const SELECTED_BG: Color = Color::Rgb(30, 30, 50);

struct DirEntry {
    name: String,
    path: PathBuf,
    is_git: bool,
    readable: bool,
    item_count: Option<usize>,
}

pub struct DirPicker {
    current_dir: PathBuf,
    entries: Vec<DirEntry>,
    selected: usize,
    show_hidden: bool,
}

impl DirPicker {
    pub fn new(start: PathBuf) -> Self {
        let mut picker = Self {
            current_dir: start,
            entries: Vec::new(),
            selected: 0,
            show_hidden: false,

        };
        picker.refresh_entries();
        picker
    }

    fn refresh_entries(&mut self) {
        // Remember currently selected name to restore position after refresh
        let selected_name = self.entries.get(self.selected).map(|e| e.name.clone());

        self.entries.clear();

        // Canonicalize to resolve any symlinks in the current path
        if let Ok(canonical) = self.current_dir.canonicalize() {
            self.current_dir = canonical;
        }

        // Check read permission before listing
        let entries = match std::fs::read_dir(&self.current_dir) {
            Ok(e) => e,
            Err(_) => {
                // Can't read this directory — go back up
                if let Some(parent) = self.current_dir.parent() {
                    self.current_dir = parent.to_path_buf();
                    let _ = self.try_read_current();
                }
                return;
            }
        };

        let mut dirs: Vec<DirEntry> = entries
            .filter_map(|e| e.ok())
            // Cap entries to prevent UI hang on huge directories
            .take(MAX_ENTRIES)
            .filter_map(|entry| {
                // Skip symlinks first — before resolving the path
                if let Ok(ft) = entry.file_type() {
                    if ft.is_symlink() {
                        return None;
                    }
                }

                let path = entry.path();
                if !path.is_dir() {
                    return None;
                }

                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden unless toggled
                if name.starts_with('.') && !self.show_hidden {
                    return None;
                }

                let readable = std::fs::read_dir(&path).is_ok();
                let is_git = readable && path.join(".git").exists();

                // Count subdirectories — cap iteration to avoid hanging on huge dirs
                let item_count = std::fs::read_dir(&path).ok().map(|rd| {
                    rd.filter_map(|e| e.ok())
                        .take(MAX_ENTRIES)
                        .filter(|e| {
                            e.file_type().map(|ft| ft.is_dir() && !ft.is_symlink()).unwrap_or(false)
                        })
                        .count()
                });

                Some(DirEntry {
                    name,
                    path,
                    is_git,
                    readable,
                    item_count,
                })
            })
            .collect();

        dirs.sort_by(|a, b| {
            b.is_git.cmp(&a.is_git).then(a.name.cmp(&b.name))
        });

        self.entries = dirs;

        // Restore selection to the previously selected entry, or clamp
        if let Some(ref name) = selected_name {
            if let Some(pos) = self.entries.iter().position(|e| &e.name == name) {
                self.selected = pos;
            } else {
                self.selected = self.selected.min(self.entries.len().saturating_sub(1));
            }
        } else {
            self.selected = 0;
        }
    }

    fn try_read_current(&mut self) -> bool {
        std::fs::read_dir(&self.current_dir).is_ok()
    }

    fn navigate_into(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            // Canonicalize to resolve symlinks and validate the path exists
            let target = match entry.path.canonicalize() {
                Ok(p) => p,
                Err(_) => return, // Can't resolve — don't navigate
            };

            // Verify we can read before navigating
            if std::fs::read_dir(&target).is_err() {
                return; // Permission denied — stay put
            }

            self.current_dir = target;
            self.refresh_entries();
        }
    }

    fn navigate_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            let target = parent.to_path_buf();
            // Only navigate up if we can read the parent
            if std::fs::read_dir(&target).is_ok() {
                self.current_dir = target;
                self.refresh_entries();
            }
        }
    }

    fn select_next(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1).min(self.entries.len() - 1);
        }
    }

    fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.refresh_entries();
    }

    fn home_dir() -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
    }

    fn display_path(&self) -> String {
        let path = self.current_dir.to_string_lossy();
        if let Some(home) = dirs::home_dir() {
            let home_str = home.to_string_lossy();
            if path.starts_with(home_str.as_ref()) {
                return format!("~{}", &path[home_str.len()..]);
            }
        }
        path.to_string()
    }
}

pub fn run_picker() -> Result<Option<PathBuf>> {
    let mut picker = DirPicker::new(DirPicker::home_dir());

    // Setup terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut last_refresh = Instant::now();

    let result = loop {
        terminal.draw(|f| draw_picker(f, &picker))?;

        // Auto-refresh directory listing every 2 seconds
        if last_refresh.elapsed() >= Duration::from_secs(2) {
            picker.refresh_entries();
            last_refresh = Instant::now();
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        break None;
                    }
                    (KeyCode::Enter, _) | (KeyCode::Char(' '), _) => {
                        // Select current directory
                        break Some(picker.current_dir.clone());
                    }
                    (KeyCode::Right | KeyCode::Char('l'), _) => {
                        picker.navigate_into();
                    }
                    (KeyCode::Left | KeyCode::Char('h'), _) | (KeyCode::Backspace, _) => {
                        picker.navigate_up();
                    }
                    (KeyCode::Down | KeyCode::Char('j'), _) => picker.select_next(),
                    (KeyCode::Up | KeyCode::Char('k'), _) => picker.select_prev(),
                    (KeyCode::Char('.'), _) => picker.toggle_hidden(),
                    (KeyCode::Char('~'), _) => {
                        picker.current_dir = DirPicker::home_dir();
                        picker.refresh_entries();
                    }
                    (KeyCode::Tab, _) => {
                        // Tab navigates into highlighted dir
                        picker.navigate_into();
                    }
                    (KeyCode::Esc, _) => {
                        picker.navigate_up();
                    }
                    _ => {}
                }
            }
        }
    };

    // Restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    Ok(result)
}

fn draw_picker(f: &mut Frame, picker: &DirPicker) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Length(3), // current path
        Constraint::Min(5),   // directory list
        Constraint::Length(3), // footer
    ])
    .split(f.area());

    // Header
    let header_line = Line::from(vec![
        Span::styled(" gruth", Style::default().fg(CYAN).bold()),
        Span::styled(" │ ", Style::default().fg(BORDER)),
        Span::styled("Git Repository UTility Helper", Style::default().fg(DIM)),
        Span::styled(" │ ", Style::default().fg(BORDER)),
        Span::styled(format!("v{}", env!("CARGO_PKG_VERSION")), Style::default().fg(DIM)),
    ]);
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER));
    f.render_widget(Paragraph::new(header_line).block(header_block), chunks[0]);

    // Current path bar
    let path_line = Line::from(vec![
        Span::styled("  📂 ", Style::default().fg(YELLOW)),
        Span::styled(
            picker.display_path(),
            Style::default().fg(Color::White).bold(),
        ),
        Span::styled(
            format!("  ({} dirs)", picker.entries.len()),
            Style::default().fg(DIM),
        ),
    ]);
    let path_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER));
    f.render_widget(Paragraph::new(path_line).block(path_block), chunks[1]);

    // Directory list
    if picker.entries.is_empty() {
        let empty = Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("Empty directory", Style::default().fg(DIM)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER)),
        );
        f.render_widget(empty, chunks[2]);
    } else {
        let rows: Vec<Row> = picker
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let selected = i == picker.selected;
                let prefix = if selected { "▸ " } else { "  " };

                let icon = if entry.is_git { " " } else { "  " };
                let icon_color = if entry.is_git { GREEN } else { DIM };

                let name_style = if !entry.readable {
                    Style::default().fg(Color::Red)
                } else if selected {
                    Style::default().fg(Color::White).bold()
                } else if entry.is_git {
                    Style::default().fg(GREEN)
                } else if entry.name.starts_with('.') {
                    Style::default().fg(DIM)
                } else {
                    Style::default().fg(Color::White)
                };

                let info_text = if !entry.readable {
                    "permission denied".to_string()
                } else if entry.is_git {
                    "git repo".to_string()
                } else {
                    String::new()
                };
                let info_color = if !entry.readable {
                    Color::Red
                } else {
                    GREEN
                };

                let count_text = if !entry.readable {
                    "🔒".to_string()
                } else {
                    match entry.item_count {
                        Some(0) => "empty".to_string(),
                        Some(n) if n >= MAX_ENTRIES => format!("{}+ dirs", n),
                        Some(n) => format!("{} dirs", n),
                        None => "—".to_string(),
                    }
                };

                let row = Row::new(vec![
                    Cell::from(format!("{}{}", prefix, icon))
                        .style(Style::default().fg(icon_color)),
                    Cell::from(entry.name.clone()).style(name_style),
                    Cell::from(info_text).style(Style::default().fg(info_color)),
                    Cell::from(count_text).style(Style::default().fg(DIM)),
                ]);

                if selected {
                    row.style(Style::default().bg(SELECTED_BG))
                } else {
                    row
                }
            })
            .collect();

        let widths = [
            Constraint::Length(5),
            Constraint::Percentage(50),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ];

        let table = Table::new(rows, widths)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BORDER)),
            )
            .row_highlight_style(Style::default());

        let mut state = TableState::default();
        state.select(Some(picker.selected));
        f.render_stateful_widget(table, chunks[2], &mut state);
    }

    // Footer
    let hidden_indicator = if picker.show_hidden {
        Span::styled(" [showing hidden]", Style::default().fg(YELLOW))
    } else {
        Span::raw("")
    };

    let footer_line = Line::from(vec![
        Span::styled("  ⏎/space", Style::default().fg(CYAN).bold()),
        Span::styled(" select ", Style::default().fg(DIM)),
        Span::styled("→/l", Style::default().fg(CYAN).bold()),
        Span::styled(" open ", Style::default().fg(DIM)),
        Span::styled("←/h", Style::default().fg(CYAN).bold()),
        Span::styled(" back ", Style::default().fg(DIM)),
        Span::styled(".", Style::default().fg(CYAN).bold()),
        Span::styled(" hidden ", Style::default().fg(DIM)),
        Span::styled("~", Style::default().fg(CYAN).bold()),
        Span::styled(" home ", Style::default().fg(DIM)),
        Span::styled("q", Style::default().fg(CYAN).bold()),
        Span::styled(" quit", Style::default().fg(DIM)),
        hidden_indicator,
    ]);
    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER));
    f.render_widget(Paragraph::new(footer_line).block(footer_block), chunks[3]);
}
