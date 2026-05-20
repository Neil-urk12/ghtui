use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use tracing::warn;

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeConfig,
    pub keybindings: KeybindingsConfig,
    pub defaults: DefaultsConfig,
    pub api: ApiConfig,
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
    pub warning_color: String,
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
    pub navigate_up: String,
    pub navigate_down: String,
    pub select: String,
    pub back: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DefaultsConfig {
    pub pulls_state: String,
    pub issues_state: String,
    pub notifications_show_all: bool,
    pub refresh_interval_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ApiConfig {
    pub base_url: String,
    pub cache_ttl_secs: CacheTtlConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct CacheTtlConfig {
    pub repos: u64,
    pub pulls: u64,
    pub issues: u64,
    pub notifications: u64,
    pub actions: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            keybindings: KeybindingsConfig::default(),
            defaults: DefaultsConfig::default(),
            api: ApiConfig::default(),
        }
    }
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
            warning_color: "yellow".to_string(),
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
            navigate_up: "k".to_string(),
            navigate_down: "j".to_string(),
            select: "Enter".to_string(),
            back: "Esc".to_string(),
        }
    }
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            pulls_state: "open".to_string(),
            issues_state: "open".to_string(),
            notifications_show_all: false,
            refresh_interval_secs: 60,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.github.com".to_string(),
            cache_ttl_secs: CacheTtlConfig::default(),
        }
    }
}

impl Default for CacheTtlConfig {
    fn default() -> Self {
        Self {
            repos: 300,
            pulls: 30,
            issues: 60,
            notifications: 15,
            actions: 10,
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
