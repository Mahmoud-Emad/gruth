use ratatui::style::Color;
use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    pub interval: Option<u64>,
    pub depth: Option<usize>,
    pub stale_days: Option<u64>,
    pub excluded_paths: Option<Vec<String>>,
    pub default_sort: Option<String>,
    pub notifications: Option<bool>,
    pub theme: Option<ThemeConfig>,
}

#[derive(Deserialize, Default, Clone)]
pub struct ThemeConfig {
    pub accent: Option<String>,
    pub border: Option<String>,
    pub clean: Option<String>,
    pub dirty: Option<String>,
    pub error: Option<String>,
    pub ahead: Option<String>,
    pub behind: Option<String>,
    pub stale: Option<String>,
    pub selected_bg: Option<String>,
}

/// Resolved theme colors ready for rendering.
#[derive(Clone)]
pub struct Theme {
    pub accent: Color,
    pub border: Color,
    pub clean: Color,
    pub dirty: Color,
    pub error: Color,
    pub ahead: Color,
    pub behind: Color,
    pub stale: Color,
    pub selected_bg: Color,
    pub dim: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: Color::Cyan,
            border: Color::DarkGray,
            clean: Color::Green,
            dirty: Color::Yellow,
            error: Color::Red,
            ahead: Color::Cyan,
            behind: Color::Magenta,
            stale: Color::Rgb(180, 60, 60),
            selected_bg: Color::Rgb(30, 30, 50),
            dim: Color::DarkGray,
        }
    }
}

pub struct PresetTheme {
    pub name: &'static str,
    pub theme: Theme,
}

impl Theme {
    pub fn presets() -> Vec<PresetTheme> {
        vec![
            PresetTheme {
                name: "Default",
                theme: Theme::default(),
            },
            PresetTheme {
                name: "Dracula",
                theme: Theme {
                    accent: Color::Rgb(189, 147, 249),  // purple
                    border: Color::Rgb(68, 71, 90),
                    clean: Color::Rgb(80, 250, 123),    // green
                    dirty: Color::Rgb(241, 250, 140),   // yellow
                    error: Color::Rgb(255, 85, 85),     // red
                    ahead: Color::Rgb(139, 233, 253),   // cyan
                    behind: Color::Rgb(255, 121, 198),   // pink
                    stale: Color::Rgb(255, 85, 85),
                    selected_bg: Color::Rgb(68, 71, 90),
                    dim: Color::Rgb(98, 114, 164),
                },
            },
            PresetTheme {
                name: "Nord",
                theme: Theme {
                    accent: Color::Rgb(136, 192, 208),   // frost
                    border: Color::Rgb(76, 86, 106),
                    clean: Color::Rgb(163, 190, 140),    // green
                    dirty: Color::Rgb(235, 203, 139),    // yellow
                    error: Color::Rgb(191, 97, 106),     // red
                    ahead: Color::Rgb(129, 161, 193),    // blue
                    behind: Color::Rgb(180, 142, 173),   // purple
                    stale: Color::Rgb(191, 97, 106),
                    selected_bg: Color::Rgb(59, 66, 82),
                    dim: Color::Rgb(76, 86, 106),
                },
            },
            PresetTheme {
                name: "Monokai",
                theme: Theme {
                    accent: Color::Rgb(102, 217, 239),   // cyan
                    border: Color::Rgb(117, 113, 94),
                    clean: Color::Rgb(166, 226, 46),     // green
                    dirty: Color::Rgb(230, 219, 116),    // yellow
                    error: Color::Rgb(249, 38, 114),     // pink-red
                    ahead: Color::Rgb(102, 217, 239),
                    behind: Color::Rgb(174, 129, 255),   // purple
                    stale: Color::Rgb(249, 38, 114),
                    selected_bg: Color::Rgb(62, 61, 50),
                    dim: Color::Rgb(117, 113, 94),
                },
            },
            PresetTheme {
                name: "Solarized",
                theme: Theme {
                    accent: Color::Rgb(38, 139, 210),    // blue
                    border: Color::Rgb(88, 110, 117),
                    clean: Color::Rgb(133, 153, 0),      // green
                    dirty: Color::Rgb(181, 137, 0),      // yellow
                    error: Color::Rgb(220, 50, 47),      // red
                    ahead: Color::Rgb(42, 161, 152),     // cyan
                    behind: Color::Rgb(211, 54, 130),    // magenta
                    stale: Color::Rgb(220, 50, 47),
                    selected_bg: Color::Rgb(7, 54, 66),
                    dim: Color::Rgb(88, 110, 117),
                },
            },
            PresetTheme {
                name: "Gruvbox",
                theme: Theme {
                    accent: Color::Rgb(215, 153, 33),    // yellow
                    border: Color::Rgb(124, 111, 100),
                    clean: Color::Rgb(152, 151, 26),     // green
                    dirty: Color::Rgb(215, 153, 33),     // yellow
                    error: Color::Rgb(204, 36, 29),      // red
                    ahead: Color::Rgb(69, 133, 136),     // aqua
                    behind: Color::Rgb(177, 98, 134),    // purple
                    stale: Color::Rgb(204, 36, 29),
                    selected_bg: Color::Rgb(60, 56, 54),
                    dim: Color::Rgb(124, 111, 100),
                },
            },
            PresetTheme {
                name: "Tokyo Night",
                theme: Theme {
                    accent: Color::Rgb(122, 162, 247),   // blue
                    border: Color::Rgb(61, 89, 161),
                    clean: Color::Rgb(158, 206, 106),    // green
                    dirty: Color::Rgb(224, 175, 104),    // orange
                    error: Color::Rgb(247, 118, 142),    // red
                    ahead: Color::Rgb(125, 207, 255),    // cyan
                    behind: Color::Rgb(187, 154, 247),   // purple
                    stale: Color::Rgb(247, 118, 142),
                    selected_bg: Color::Rgb(41, 46, 66),
                    dim: Color::Rgb(61, 89, 161),
                },
            },
            PresetTheme {
                name: "Matrix",
                theme: Theme {
                    accent: Color::Rgb(0, 255, 0),
                    border: Color::Rgb(0, 80, 0),
                    clean: Color::Rgb(0, 255, 0),
                    dirty: Color::Rgb(0, 200, 0),
                    error: Color::Rgb(255, 0, 0),
                    ahead: Color::Rgb(0, 255, 100),
                    behind: Color::Rgb(0, 180, 0),
                    stale: Color::Rgb(100, 0, 0),
                    selected_bg: Color::Rgb(0, 30, 0),
                    dim: Color::Rgb(0, 80, 0),
                },
            },
        ]
    }

    pub fn from_config(tc: Option<&ThemeConfig>) -> Self {
        let mut theme = Self::default();
        let Some(tc) = tc else { return theme };

        if let Some(c) = tc.accent.as_deref().and_then(parse_color) {
            theme.accent = c;
            theme.ahead = c; // ahead defaults to accent
        }
        if let Some(c) = tc.border.as_deref().and_then(parse_color) {
            theme.border = c;
            theme.dim = c;
        }
        if let Some(c) = tc.clean.as_deref().and_then(parse_color) {
            theme.clean = c;
        }
        if let Some(c) = tc.dirty.as_deref().and_then(parse_color) {
            theme.dirty = c;
        }
        if let Some(c) = tc.error.as_deref().and_then(parse_color) {
            theme.error = c;
        }
        if let Some(c) = tc.ahead.as_deref().and_then(parse_color) {
            theme.ahead = c;
        }
        if let Some(c) = tc.behind.as_deref().and_then(parse_color) {
            theme.behind = c;
        }
        if let Some(c) = tc.stale.as_deref().and_then(parse_color) {
            theme.stale = c;
        }
        if let Some(c) = tc.selected_bg.as_deref().and_then(parse_color) {
            theme.selected_bg = c;
        }

        theme
    }
}

fn parse_color(s: &str) -> Option<Color> {
    // Named colors
    match s.to_lowercase().as_str() {
        "black" => return Some(Color::Black),
        "red" => return Some(Color::Red),
        "green" => return Some(Color::Green),
        "yellow" => return Some(Color::Yellow),
        "blue" => return Some(Color::Blue),
        "magenta" => return Some(Color::Magenta),
        "cyan" => return Some(Color::Cyan),
        "white" => return Some(Color::White),
        "darkgray" | "dark_gray" => return Some(Color::DarkGray),
        "lightred" | "light_red" => return Some(Color::LightRed),
        "lightgreen" | "light_green" => return Some(Color::LightGreen),
        "lightyellow" | "light_yellow" => return Some(Color::LightYellow),
        "lightblue" | "light_blue" => return Some(Color::LightBlue),
        "lightmagenta" | "light_magenta" => return Some(Color::LightMagenta),
        "lightcyan" | "light_cyan" => return Some(Color::LightCyan),
        "gray" => return Some(Color::Gray),
        _ => {}
    }

    // Hex colors: #RRGGBB
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

impl Config {
    pub fn load() -> Self {
        let Some(config_dir) = dirs::config_dir() else {
            return Self::default();
        };

        let path = config_dir.join("gruth").join("config.toml");
        let Ok(contents) = std::fs::read_to_string(&path) else {
            return Self::default();
        };

        toml::from_str(&contents).unwrap_or_default()
    }
}

/// Save the selected theme name to cache.
pub fn save_cached_theme(name: &str) {
    let Some(cache_dir) = dirs::cache_dir() else { return };
    let dir = cache_dir.join("gruth");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("theme"), name);
}

/// Load the cached theme name.
pub fn load_cached_theme() -> Option<String> {
    let cache_dir = dirs::cache_dir()?;
    let path = cache_dir.join("gruth").join("theme");
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// Resolve the theme: config [theme] section > cached preset > default.
pub fn resolve_theme(config: &Config) -> Theme {
    // If config has a [theme] section, use that
    if config.theme.is_some() {
        return Theme::from_config(config.theme.as_ref());
    }

    // Check for a cached preset theme
    if let Some(name) = load_cached_theme() {
        let presets = Theme::presets();
        if let Some(preset) = presets.iter().find(|p| p.name == name) {
            return preset.theme.clone();
        }
    }

    Theme::default()
}
