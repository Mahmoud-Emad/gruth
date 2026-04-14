# gruth

A terminal dashboard for monitoring and syncing multiple git repositories.

```
┌──────────────────────────────────────────────────────────────────────────┐
│ gruth │ git repo monitor │ v0.2.0 │ ● idle                               │
├──────────────────────────────────────────────────────────────────────────┤
│  Repository        Branch       Status     Sync     Commit     Br        │
│▸ hero_lib          development  ● clean    ✓ synced 2h ago     3 br      │
│  hero_router       main         ● dirty    ↑3 ↓1   15m ago    2 br       │
│  hero_proc         development  ● clean    ↓5       1d ago     5 br      │
│  hero_browser      main         ✖ conflict —        3d ago     1 br      │
│  hero_books        development  ● clean    ✓ synced 45d ago    2 br      │
├──────────────────────────────────────────────────────────────────────────┤
│  5 repos │ ● 3 ● 1 ⏳1 │ q quit / search f filter s sort ⏎ detail         │
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
# Monitor repos in current directory
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
| `-p, --path <DIR>` | Root directory to scan | `.` |
| `-d, --depth <N>` | Max recursion depth | `10` |
| `-i, --interval <N>` | Refresh interval in seconds | `5` |
| `--stale-days <N>` | Days before a repo is stale | `30` |
| `--sync` | Fetch and pull all clean repos, then exit | |

## Keybindings

| Key | Action |
|-----|--------|
| `q` / `Esc` | Quit (or close detail pane / clear filter) |
| `r` | Force refresh |
| `↑` / `k` | Move up (or scroll detail pane) |
| `↓` / `j` | Move down (or scroll detail pane) |
| `/` | Search repos by name |
| `f` | Cycle status filter (all/clean/dirty/behind/ahead/errors/stale) |
| `s` | Cycle sort order (name/status/commit/behind) |
| `Enter` | Open/close repo detail pane |
| `Ctrl+C` | Quit |

## Features

### Live monitoring
Periodically fetches all remotes and checks branch status, ahead/behind counts, and working tree state for every discovered repo.

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

### Branch overview
The "Br" column shows local branch count per repo. The detail pane shows full branch info including merged status and orphaned upstream detection.

### Sync mode
`gruth --sync` runs without a TUI. It fetches all remotes and fast-forward pulls repos that are clean and behind their upstream. Dirty repos and diverged histories are skipped.

### Config file
Create `~/.config/gruth/config.toml` to set defaults:

```toml
interval = 10
depth = 5
stale_days = 14
excluded_paths = ["node_modules", "vendor", "target"]
default_sort = "name"  # name, status, commit, behind
```

CLI flags override config file values.

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
