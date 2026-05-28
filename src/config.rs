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
    pub diff_addition_color: String,
    pub diff_deletion_color: String,
    pub diff_header_color: String,
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

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DefaultsConfig {
    pub pulls_state: String,
    pub issues_state: String,
    pub notifications_show_all: bool,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            pulls_state: "open".to_string(),
            issues_state: "open".to_string(),
            notifications_show_all: false,
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
            diff_addition_color: "green".to_string(),
            diff_deletion_color: "red".to_string(),
            diff_header_color: "cyan".to_string(),
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
        Self::load_from_file(&config_path)
    }

    pub fn load_from_file(path: &std::path::Path) -> Self {
        if !path.exists() {
            return Config::default();
        }

        match fs::read_to_string(path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(config) => config,
                Err(e) => {
                    warn!("Failed to parse config at {}: {}", path.display(), e);
                    Config::default()
                }
            },
            Err(e) => {
                warn!("Failed to read config at {}: {}", path.display(), e);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert_eq!(config.theme.active_border_color, "cyan");
        assert_eq!(config.theme.inactive_border_color, "gray");
        assert_eq!(config.keybindings.quit, "q");
        assert_eq!(config.keybindings.next_tab, "Tab");
        assert_eq!(config.keybindings.prev_tab, "Shift-Tab");
        assert_eq!(config.defaults.pulls_state, "open");
        assert_eq!(config.defaults.issues_state, "open");
        assert!(!config.defaults.notifications_show_all);
    }

    #[test]
    fn parse_toml_overrides() {
        let toml_str = r#"
[keybindings]
quit = "x"

[defaults]
pulls_state = "closed"
notifications_show_all = true

[theme]
active_border_color = "red"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.keybindings.quit, "x");
        assert_eq!(config.defaults.pulls_state, "closed");
        assert!(config.defaults.notifications_show_all);
        assert_eq!(config.theme.active_border_color, "red");
        // Unspecified fields get defaults
        assert_eq!(config.keybindings.refresh, "r");
        assert_eq!(config.defaults.issues_state, "open");
    }

    #[test]
    fn parse_empty_toml() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.keybindings.quit, "q");
        assert_eq!(config.defaults.pulls_state, "open");
    }

    #[test]
    #[tracing_test::traced_test]
    fn invalid_toml_logs_warning() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        std::fs::write(&config_path, "invalid [[[").unwrap();

        let _config = Config::load_from_file(&config_path);

        assert!(logs_contain("Failed to parse config"));
    }
}
