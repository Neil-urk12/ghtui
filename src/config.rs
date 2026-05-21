use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use tracing::warn;

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeConfig,
    pub keybindings: KeybindingsConfig,
    pub defaults: DefaultsConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ThemeConfig {
    pub active_border_color: String,
    pub inactive_border_color: String,
    pub highlight_color: String,
    pub muted_color: String,
    pub success_color: String,
    pub error_color: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct KeybindingsConfig {
    pub quit: String,
    pub refresh: String,
    pub next_tab: String,
    pub prev_tab: String,
    pub search: String,
    pub help: String,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct DefaultsConfig {
    pub pulls_state: String,
    pub issues_state: String,
    pub notifications_show_all: bool,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            active_border_color: "cyan".to_string(),
            inactive_border_color: "gray".to_string(),
            highlight_color: "blue".to_string(),
            muted_color: "gray".to_string(),
            success_color: "green".to_string(),
            error_color: "red".to_string(),
        }
    }
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            quit: "q".to_string(),
            refresh: "r".to_string(),
            next_tab: "Tab".to_string(),
            prev_tab: "Shift-Tab".to_string(),
            search: "/".to_string(),
            help: "?".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = config_path();
        if !config_path.exists() {
            return Config::default();
        }

        match fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(config) => config,
                Err(e) => {
                    warn!("Failed to parse config at {}: {}", config_path.display(), e);
                    Config::default()
                }
            },
            Err(e) => {
                warn!("Failed to read config at {}: {}", config_path.display(), e);
                Config::default()
            }
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ghtui")
        .join("config.toml")
}
