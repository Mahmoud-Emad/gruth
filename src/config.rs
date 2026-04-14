use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    pub interval: Option<u64>,
    pub depth: Option<usize>,
    pub stale_days: Option<u64>,
    pub excluded_paths: Option<Vec<String>>,
    pub default_sort: Option<String>,
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
