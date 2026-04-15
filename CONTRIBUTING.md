# Contributing to gruth

Thanks for your interest in contributing to gruth!

## Getting Started

```bash
git clone https://github.com/mik-tf/gruth.git
cd gruth
cargo build
cargo run -- -p ~/your-projects-dir
```

## Project Structure

```
src/
├── main.rs        Entry point, event loop, key handling
├── app.rs         Application state, data structures, state transitions
├── cli.rs         Command-line argument definitions (clap)
├── config.rs      Config file loading, theme presets, theme caching
├── dir_picker.rs  Interactive directory browser
├── events.rs      Terminal event handling (crossterm → AppEvent)
├── git_ops.rs     All git operations via libgit2
├── scanner.rs     Recursive git repository discovery
├── sync.rs        Headless sync mode (--sync)
└── ui.rs          TUI rendering (ratatui)
```

## Architecture

- **Event-driven**: tokio async runtime with crossterm input polling
- **Background tasks**: git operations run in `spawn_blocking` tasks, results sent via mpsc channel
- **State machine**: `AppState` holds all data, `InputMode` enum tracks keyboard context (Normal/Search/ThemePicker)
- **Pure rendering**: `ui.rs` functions take `&AppState` and render — no side effects

## Development

```bash
# Build
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
4. Test with real git repositories
5. Update README.md if adding user-facing features

## Code Style

- Follow existing patterns — look at how similar features are implemented
- No unnecessary abstractions — simple and direct
- Error handling: `anyhow::Result` for top-level, graceful fallbacks for per-repo operations
- UI feedback: use toast messages to communicate actions to the user

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
