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
│  5 repos │ ● 3 ● 1 ⏳1 │ q quit / search f filter p pull ⏎ detail ? help│
└──────────────────────────────────────────────────────────────────────────┘
```

## Install

### From GitHub Releases (prebuilt binary)

Download the latest release for your platform from the [Releases page](https://github.com/mik-tf/gruth/releases), then:

```bash
# Extract the archive (the download is a .tar.gz)
tar xzf gruth-macos-arm64.tar.gz    # or gruth-linux-amd64, gruth-linux-arm64

# Make it executable and move to your PATH
chmod +x gruth
sudo mv gruth /usr/local/bin/
```

Available binaries:

| Platform | File |
|----------|------|
| macOS Apple Silicon | `gruth-macos-arm64.tar.gz` |
| Linux x86_64 | `gruth-linux-amd64.tar.gz` |
| Linux ARM64 | `gruth-linux-arm64.tar.gz` |

### From source (requires Rust 1.70+)

```bash
cargo install --path .
```

Or clone and build:

```bash
git clone https://github.com/mik-tf/gruth.git
cd gruth
cargo build --release
sudo cp target/release/gruth /usr/local/bin/
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

# Sync mode: fetch and pull all clean repos, then exit
gruth --sync

# Sync a specific directory
gruth --sync -p ~/projects

# Show version and commit hash
gruth version

# Self-update to the latest release
gruth update
```

## Commands

| Command | Description |
|---------|-------------|
| `gruth` | Launch TUI (with directory picker) |
| `gruth version` | Show version and git commit hash |
| `gruth update` | Self-update to the latest GitHub release |

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `-p, --path <DIR>` | Root directory to scan (opens picker if omitted) | picker |
| `-d, --depth <N>` | Max recursion depth for repo discovery | `10` |
| `-i, --interval <N>` | Refresh interval in seconds | `5` |
| `--stale-days <N>` | Days before a repo is considered stale | `30` |
| `--sync` | Fetch and pull all clean repos, then exit | |

## Keybindings

Press `?` inside gruth to see all shortcuts. Full reference below:

### Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up (or scroll detail/help pane) |
| `↓` / `j` | Move down (or scroll detail/help pane) |
| `Enter` | Open / close detail pane |
| `b` | Back to directory picker |

### Actions

| Key | Action |
|-----|--------|
| `p` | Pull selected repo (clean + behind only) |
| `P` | Pull all eligible repos (clean + behind) |
| `r` | Force refresh all repos |
| `i` | Show error details for selected repo |

### Search & Filter

| Key | Action |
|-----|--------|
| `/` | Search repos by name |
| `f` | Cycle filter (all → clean → dirty → behind → ahead → errors → stale) |
| `s` | Cycle sort (name → status → commit → behind) |
| `Esc` | Clear filter/search, close pane, or quit |

### Appearance

| Key | Action |
|-----|--------|
| `t` | Open theme picker with live preview |

### General

| Key | Action |
|-----|--------|
| `q` | Quit (or close active pane/overlay) |
| `Ctrl+C` | Force quit |
| `?` | Toggle keyboard shortcuts help |

### Search mode

| Key | Action |
|-----|--------|
| `Esc` | Cancel search |
| `Enter` | Confirm search |
| `Backspace` | Delete character |

### Theme picker

| Key | Action |
|-----|--------|
| `↑` / `k` | Previous theme |
| `↓` / `j` | Next theme |
| `Enter` | Confirm selection |
| `Esc` / `q` | Cancel (revert to previous theme) |

### Directory picker

| Key | Action |
|-----|--------|
| `→` / `l` / `Tab` | Open directory |
| `←` / `h` / `Backspace` / `Esc` | Go to parent |
| `Enter` / `Space` | Select directory and start monitoring |
| `↑↓` / `jk` | Navigate |
| `.` | Toggle hidden directories |
| `~` | Jump to home directory |
| `q` / `Ctrl+C` | Quit |

## Features

### Directory picker
Run `gruth` without `-p` to browse your filesystem and pick a directory. Git repos are highlighted in green with a `[git]` badge. The picker auto-refreshes every 2 seconds to reflect filesystem changes. Press `b` anytime in the main view to return to the picker.

### Live filesystem monitoring
The repo list stays in sync with your filesystem. Every refresh cycle (default 5s), gruth re-scans the directory tree — deleted repos are removed, newly cloned repos appear, and existing repo state is preserved. No restart needed.

### Pull from TUI
Press `p` to pull the selected repo, or `P` to pull all eligible repos at once. Only works on clean repos that are behind their remote. Dirty repos, conflicts, repos with errors, and already-synced repos show a toast explaining why the pull was skipped.

### Error info
Press `i` on a repo with errors to see the full error message in a centered overlay. Does nothing on repos without errors.

### Help screen
Press `?` to see all keyboard shortcuts organized by category. Scrollable with `j`/`k`.

### Self-update
Run `gruth update` to download and install the latest release from GitHub. The TUI also checks for updates on startup — if a newer version exists, you'll see `↑ 0.3.0 available` in the header bar and a toast notification prompting you to run `gruth update`.

Run `gruth version` to see the current version and git commit hash.

### Toast notifications
Actions show color-coded feedback messages that auto-dismiss after 5 seconds:
- **Info** (cyan): scan results, filter/sort changes, pull started, refresh started
- **Success** (green): pull completed, theme selected
- **Warning** (yellow): pull skipped (dirty repo), already pulling
- **Error** (red): pull failed, repo has conflicts/errors

### Desktop notifications
Get a system notification when a repo transitions from synced to behind (someone pushed to the remote). Disable in config with `notifications = false`.

### Stale detection
Repos with no commits in 30+ days (configurable via `--stale-days` or config) are highlighted. Use `f` to filter and show only stale repos.

### Search and filter
Press `/` to search repos by name (substring match). Press `f` to cycle through status filters: all, clean, dirty, behind, ahead, errors, stale. The footer shows the active filter and matching repo count.

### Sort
Press `s` to cycle sort order:
- **name**: alphabetical (default)
- **status**: errors first → conflicts → dirty → clean
- **commit**: most recent first
- **behind**: most behind first

### Detail pane
Press `Enter` on a repo to see:
- Recent commits (last 10 with date, author, and message)
- Changed files with status indicators (`A` added, `M` modified, `D` deleted, `R` renamed)
- Remote URLs
- Local branches with orphaned upstream (`⚠ upstream gone`) and merged status

Scroll the detail pane with `j`/`k`. Close with `Enter`, `q`, or `Esc`.

### Theme picker
Press `t` to open the theme picker. Themes preview live as you navigate. Press `Enter` to confirm or `Esc` to cancel (reverts to the previous theme). Your selection is cached at `~/.cache/gruth/theme` and persists across sessions.

**Built-in themes:** Default, Dracula, Nord, Monokai, Solarized, Gruvbox, Tokyo Night, Matrix

### Responsive columns
Columns auto-hide based on terminal width to keep the display readable:

| Width | Visible columns |
|-------|----------------|
| < 60 | Repository, Status |
| 60–79 | + Branch |
| 80–99 | + Sync |
| ≥ 100 | + Last Commit (all columns) |

### Sync mode
`gruth --sync` runs without a TUI. It fetches all remotes and fast-forward pulls repos that are clean and behind their upstream. Dirty repos, conflicts, and diverged histories are skipped. Prints a summary with counts of pulled, up-to-date, skipped, and errored repos.

```
 gruth sync │ scanning ~/projects...

  Found 12 repositories

  ↓ PULL   hero_lib      [development] (↓3 — Fast-forwarded)
  ✓ OK     hero_router   [main]
  ⊘ SKIP   hero_proc     [development] (dirty)
  ✗ PULL   hero_browser  [main] (Cannot fast-forward)

  ─────────────────────────────────────
  ↓ 1 pulled  ✓ 8 up-to-date  ⊘ 2 skipped  ✗ 1 errors
```

## Config

Create `~/.config/gruth/config.toml` to set defaults:

```toml
# Refresh interval in seconds
interval = 10

# Max directory scan depth
depth = 5

# Days without commits before stale highlighting
stale_days = 14

# Desktop notifications when repos go behind
notifications = true

# Directories to skip during scanning
excluded_paths = ["node_modules", "vendor", "target"]

# Default sort order: name, status, commit, behind
default_sort = "name"

# Custom theme colors (overrides preset themes)
# Supports named colors and #RRGGBB hex
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

CLI flags override config file values. The `[theme]` section overrides the cached preset theme.

### Theme resolution order

1. Config file `[theme]` section (if present)
2. Cached preset from theme picker (`~/.cache/gruth/theme`)
3. Default theme

## Status indicators

| Symbol | Meaning |
|--------|---------|
| `● clean` | No uncommitted changes |
| `● dirty` | Uncommitted changes present |
| `✖ conflict` | Merge conflicts |
| `✗ error` | Git operation failed (press `i` for details) |
| `◌ ...` | Fetching in progress |
| `✓ synced` | Up to date with remote |
| `↑N` | N commits ahead of remote |
| `↓N` | N commits behind remote |
| `↓ pulling...` | Pull in progress |
| `✓ pulled` | Pull succeeded |
| `✗ pull failed` | Pull failed |
| `⏳` | Stale repo (no recent commits) |
| `⚠ upstream gone` | Branch upstream no longer exists |

## Requirements

- Rust 1.70+
- Git repositories with configured remotes (for ahead/behind tracking)
- SSH agent running (for fetching private repos)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, project structure, and guidelines.

## License

MIT
