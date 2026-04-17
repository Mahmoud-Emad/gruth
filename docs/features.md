# Features

## Directory picker

Run `gruth` without `-p` to browse your filesystem and pick a directory. Git repos are highlighted in green with a `[git]` badge. The picker auto-refreshes every 2 seconds to reflect filesystem changes. Press `b` anytime in the main view to return to the picker.

## Live filesystem monitoring

The repo list stays in sync with your filesystem. Every refresh cycle (default 5s), gruth re-scans the directory tree — deleted repos are removed, newly cloned repos appear, and existing repo state is preserved. No restart needed.

## Pull from TUI

Press `p` to pull the selected repo, or `P` to pull all eligible repos at once. Only works on clean repos that are behind their remote. Dirty repos, conflicts, repos with errors, and already-synced repos show a toast explaining why the pull was skipped.

## Error info

Press `i` on a repo with errors to see the full error message in a centered overlay. Does nothing on repos without errors.

## Help screen

Press `?` to see all keyboard shortcuts organized by category. Scrollable with `j`/`k`.

## Self-update

Run `gruth update` to download and install the latest release from GitHub. The TUI also checks for updates on startup — if a newer version exists, you'll see `↑ X.Y.Z available` in the header bar and a toast notification prompting you to run `gruth update`.

Run `gruth version` to see the current version and git commit hash.

## Toast notifications

Actions show color-coded feedback messages that auto-dismiss after 5 seconds:

- **Info** (cyan): scan results, filter/sort changes, pull started, refresh started
- **Success** (green): pull completed, theme selected
- **Warning** (yellow): pull skipped (dirty repo), already pulling
- **Error** (red): pull failed, repo has conflicts/errors

## Desktop notifications

Get a system notification when a repo transitions from synced to behind (someone pushed to the remote). Disable in config with `notifications = false`.

## Stale detection

Repos with no commits in 30+ days (configurable via `--stale-days` or config) are highlighted. Use `f` to filter and show only stale repos.

## Search and filter

Press `/` to search repos by name (substring match). Press `f` to cycle through status filters: all, clean, dirty, behind, ahead, errors, stale. The footer shows the active filter and matching repo count.

## Sort

Press `s` to cycle sort order:

- **name**: alphabetical (default)
- **status**: errors first → conflicts → dirty → clean
- **commit**: most recent first
- **behind**: most behind first

## Detail pane

Press `Enter` on a repo to see:

- Recent commits (last 10 with date, author, and message)
- Changed files with status indicators (`A` added, `M` modified, `D` deleted, `R` renamed)
- Remote URLs
- Local branches with orphaned upstream (`⚠ upstream gone`) and merged status

Scroll the detail pane with `j`/`k`. Close with `Enter`, `q`, or `Esc`.

## Responsive columns

Columns auto-hide based on terminal width to keep the display readable:

| Width | Visible columns |
|-------|----------------|
| < 60 | Repository, Status |
| 60–79 | + Branch |
| 80–99 | + Sync |
| >= 100 | + Last Commit (all columns) |

## Sync mode

`gruth --sync` runs without a TUI. It fetches all remotes and fast-forward pulls repos that are clean and behind their upstream. Dirty repos, conflicts, and diverged histories are skipped.

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
