# gruth

**<ins>G</ins>it <ins>R</ins>epository <ins>UT</ins>ility <ins>H</ins>elper** — a TUI dashboard for monitoring and syncing multiple git repositories.

```
┌──────────────────────────────────────────────────────────────────────────┐
│ gruth │ Git Repository UTility Helper │ v0.2.0 │ ● idle                 │
├──────────────────────────────────────────────────────────────────────────┤
│  Repository          Branch       Status     Sync       Last Commit     │
│▸ hero_lib            development  ● clean    ✓ synced   2h ago          │
│  hero_router         main         ● dirty    ↑3 ↓1      15m ago         │
│  hero_proc           development  ● clean    ↓5          1d ago         │
│  hero_browser        main         ✖ conflict —           3d ago         │
│  hero_books          development  ● clean    ✓ synced   45d ago         │
├──────────────────────────────────────────────────────────────────────────┤
│  5 repos │ ● 3 ● 1 ⏳1 │ q quit b back / search f filter s sort p pull │
└──────────────────────────────────────────────────────────────────────────┘
```

## Install

```bash
cargo install --path .
```

Or build from source:

```bash
git clone https://github.com/mik-tf/gruth.git
cd gruth
cargo build --release
./target/release/gruth
```

## Usage

```bash
# Launch with directory picker
gruth

# Monitor a specific directory
gruth -p ~/projects

# Set refresh interval to 30 seconds
gruth -i 30

# Limit scan depth to 3 levels
gruth -d 3

# Mark repos stale after 14 days
gruth --stale-days 14

# Sync mode: fetch and pull all clean repos
gruth --sync
```

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `-p, --path <DIR>` | Root directory to scan (opens picker if omitted) | picker |
| `-d, --depth <N>` | Max recursion depth | `10` |
| `-i, --interval <N>` | Refresh interval in seconds | `5` |
| `--stale-days <N>` | Days before a repo is stale | `30` |
| `--sync` | Fetch and pull all clean repos, then exit | |

## Keybindings

### Main view

| Key | Action |
|-----|--------|
| `q` / `Esc` | Quit (or close detail pane / clear filter) |
| `b` | Back to directory picker |
| `r` | Force refresh all repos |
| `p` | Pull selected repo (clean + behind only) |
| `/` | Search repos by name |
| `f` | Cycle status filter (all/clean/dirty/behind/ahead/errors/stale) |
| `s` | Cycle sort order (name/status/commit/behind) |
| `t` | Open theme picker with live preview |
| `Enter` | Open/close repo detail pane |
| `↑↓` / `jk` | Navigate (or scroll detail pane) |
| `Ctrl+C` | Quit |

### Directory picker

| Key | Action |
|-----|--------|
| `→` / `l` / `Tab` | Open directory |
| `←` / `h` / `Backspace` | Go to parent |
| `Enter` / `Space` | Select directory and start monitoring |
| `.` | Toggle hidden directories |
| `~` | Jump to home directory |
| `q` | Quit |

## Features

### Directory picker
Run `gruth` without `-p` to browse your filesystem and pick a directory. Git repos are highlighted in green. Press `b` anytime to go back and pick a different directory.

### Live monitoring
Periodically fetches all remotes and checks branch status, ahead/behind counts, and working tree state for every discovered repo.

### Pull from TUI
Press `p` to pull the selected repo. Only works on clean repos that are behind — dirty repos, conflicts, and already-synced repos show a helpful message explaining why.

### Toast notifications
Actions show color-coded feedback messages that auto-dismiss after 5 seconds:
- Info (cyan), success (green), warning (yellow), error (red)

### Desktop notifications
Get a desktop notification when a repo goes from synced to behind (someone pushed). Disable in config with `notifications = false`.

### Stale detection
Repos with no commits in 30+ days (configurable) are highlighted in dim red. Use `f` to filter and show only stale repos.

### Search and filter
Press `/` to search by repo name. Press `f` to cycle through status filters: all, clean, dirty, behind, ahead, errors, stale.

### Sort
Press `s` to cycle sort order: name, status (errors first), last commit (newest first), behind count (most behind first).

### Detail pane
Press `Enter` on a repo to see:
- Recent commits (last 10 with author and date)
- Changed files with status indicators (A/M/D)
- Remote URLs
- Local branches with orphaned upstream warnings

### Theme picker
Press `t` to open the theme picker. Navigate with `j`/`k`, preview themes live, press `Enter` to confirm or `Esc` to cancel.

**Built-in themes:** Default, Dracula, Nord, Monokai, Solarized, Gruvbox, Tokyo Night, Matrix

### Responsive columns
Columns auto-hide on narrow terminals to keep the display readable.

### Sync mode
`gruth --sync` runs without a TUI. It fetches all remotes and fast-forward pulls repos that are clean and behind their upstream. Dirty repos and diverged histories are skipped.

### Config file
Create `~/.config/gruth/config.toml` to set defaults:

```toml
interval = 10
depth = 5
stale_days = 14
notifications = true
excluded_paths = ["node_modules", "vendor", "target"]
default_sort = "name"  # name, status, commit, behind

[theme]
accent = "cyan"
border = "dark_gray"
clean = "green"
dirty = "yellow"
error = "red"
ahead = "cyan"
behind = "magenta"
stale = "#B43C3C"
selected_bg = "#1E1E32"
```

CLI flags override config file values. Colors support named values and `#RRGGBB` hex.

## Status indicators

| Symbol | Meaning |
|--------|---------|
| `● clean` | No uncommitted changes |
| `● dirty` | Uncommitted changes present |
| `✖ conflict` | Merge conflicts |
| `✗ error` | Git operation failed |
| `✓ synced` | Up to date with remote |
| `↑N` | N commits ahead of remote |
| `↓N` | N commits behind remote |
| `⏳` | Stale repo (no recent commits) |
| `⚠ upstream gone` | Branch upstream no longer exists |

## Requirements

- Rust 1.70+
- Git repositories with configured remotes (for ahead/behind tracking)
- SSH agent running (for fetching private repos)

## License

MIT
