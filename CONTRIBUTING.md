# Contributing to gruth

Thanks for your interest in contributing to gruth!

## Getting Started

```bash
git clone https://github.com/Mahmoud-Emad/gruth.git
cd gruth
cargo build
cargo run -- -p ~/your-projects-dir
```

## Project Structure

```
src/
├── main.rs        Entry point, event loop, key handling, background tasks
├── app.rs         Application state, data structures, state transitions
├── cli.rs         Command-line argument definitions (clap)
├── config.rs      Config loading, theme presets, theme caching, color parsing
├── dir_picker.rs  Interactive directory browser with auto-refresh
├── events.rs      Terminal event handling (crossterm → AppEvent)
├── git_ops.rs     All git operations via libgit2 (status, fetch, pull, details)
├── scanner.rs     Recursive git repository discovery
├── sync.rs        Headless sync mode (--sync)
└── ui.rs          TUI rendering — header, table, detail pane, footer, overlays
```

## Architecture

- **Event-driven**: tokio async runtime with crossterm input polling
- **Background tasks**: git operations run in `spawn_blocking` tasks, results sent back via mpsc channel
- **State machine**: `AppState` holds all data; `InputMode` enum tracks keyboard context (Normal, Search, ThemePicker, Help, ErrorInfo)
- **Pure rendering**: `ui.rs` draw functions take `&AppState` and render — no side effects
- **Live filesystem**: every refresh cycle re-scans the directory tree and reconciles the repo list (adds new, removes deleted, preserves existing state)

### Key data flow

1. `main.rs` spawns background scan → `RepoResult::ScanComplete`
2. On each tick, spawns re-scan → `RepoResult::RescanComplete` → reconciles repo list
3. Spawns fetch+status per repo → `RepoResult::RepoUpdated`
4. Pull actions → `RepoResult::PullComplete` → auto-refreshes repo
5. Detail pane → `RepoResult::DetailLoaded`

## Development

```bash
# Build (dev)
cargo build

# Run with a test directory
cargo run -- -p ~/projects -i 30

# Run sync mode
cargo run -- --sync -p ~/projects

# Release build
cargo build --release
```

## Pull Requests

1. Fork and create a feature branch
2. Keep changes focused — one feature or fix per PR
3. Ensure `cargo build` produces zero warnings
4. Test with real git repositories (both clean and dirty states)
5. Update README.md if adding user-facing features or keybindings
6. Update the help overlay in `ui.rs` (`draw_help`) if adding new shortcuts

## Code Style

- Follow existing patterns — look at how similar features are implemented
- No unnecessary abstractions — simple and direct
- Error handling: `anyhow::Result` for top-level, graceful fallbacks for per-repo operations
- UI feedback: use toast messages (`app.toast(...)`) to communicate actions to the user
- New overlays: add an `InputMode` variant, a key handler function, and a `draw_` function

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
