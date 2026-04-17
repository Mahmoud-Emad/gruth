# gruth

**<ins>G</ins>it <ins>R</ins>epository <ins>UT</ins>ility <ins>H</ins>elper** — a TUI dashboard for monitoring and syncing multiple git repositories.

```
┌──────────────────────────────────────────────────────────────────────────┐
│ gruth │ Git Repository UTility Helper │ v0.6.0 │ ● idle                 │
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

## Quick start

```bash
# Download and install (macOS ARM64)
curl -sL https://github.com/Mahmoud-Emad/gruth/releases/latest/download/gruth-macos-arm64.tar.gz | tar xz
sudo mv gruth /usr/local/bin/

# Launch
gruth
```

See [docs/install.md](docs/install.md) for all platforms and building from source.

## Usage

```bash
gruth                        # Launch with directory picker
gruth -p ~/projects          # Monitor a specific directory
gruth --sync -p ~/projects   # Headless fetch + pull, then exit
gruth version                # Show version and commit hash
gruth update                 # Self-update to latest release
```

| Flag | Description | Default |
|------|-------------|---------|
| `-p, --path <DIR>` | Root directory to scan | picker |
| `-d, --depth <N>` | Max scan depth | `10` |
| `-i, --interval <N>` | Refresh interval (seconds) | `5` |
| `--stale-days <N>` | Stale threshold (days) | `30` |
| `--sync` | Headless sync mode | |

## Key shortcuts

Press `?` inside gruth for the full list. Essentials:

| Key | Action |
|-----|--------|
| `p` / `P` | Pull selected / pull all |
| `/` | Search |
| `f` | Cycle filter |
| `s` | Cycle sort |
| `t` | Theme picker |
| `i` | Error details |
| `Enter` | Detail pane |
| `?` | Help |
| `q` | Quit |

Full reference: [docs/keybindings.md](docs/keybindings.md)

## Documentation

| Doc | Description |
|-----|-------------|
| [Installation](docs/install.md) | All install methods, platforms, self-update |
| [Features](docs/features.md) | Full feature descriptions and screenshots |
| [Keybindings](docs/keybindings.md) | Complete keyboard shortcut reference |
| [Configuration](docs/config.md) | Config file, themes, status indicators |
| [Contributing](CONTRIBUTING.md) | Development setup and guidelines |

## License

MIT
