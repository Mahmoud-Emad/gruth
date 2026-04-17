# Configuration

## Config file

Create `~/.config/gruth/config.toml`:

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

CLI flags override config file values.

## Themes

### Built-in themes

Default, Dracula, Nord, Monokai, Solarized, Gruvbox, Tokyo Night, Matrix

Press `t` in the TUI to open the theme picker with live preview. Your selection is cached at `~/.cache/gruth/theme` and persists across sessions.

### Theme resolution order

1. Config file `[theme]` section (if present)
2. Cached preset from theme picker (`~/.cache/gruth/theme`)
3. Default theme

### Custom colors

The `[theme]` section supports:

- **Named colors**: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, `gray`, `dark_gray`, `light_red`, `light_green`, `light_yellow`, `light_blue`, `light_magenta`, `light_cyan`
- **Hex colors**: `#RRGGBB` (e.g., `#B43C3C`)

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
