use ratatui::prelude::*;

use crate::config::ThemeConfig;

#[derive(Clone)]
pub struct Theme {
    pub active_border: Style,
    pub highlight: Style,
    pub muted: Style,
    pub success: Style,
    pub error: Style,
    pub merged: Style,
    pub tab_active: Style,
    pub tab_inactive: Style,
    pub status_bar: Style,
    pub diff_addition: Style,
    pub diff_deletion: Style,
    pub diff_header: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            active_border: Style::default().fg(Color::Cyan),
            highlight: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(Color::Gray),
            success: Style::default().fg(Color::Green),
            error: Style::default().fg(Color::Red),
            merged: Style::default().fg(Color::Magenta),
            tab_active: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            tab_inactive: Style::default().fg(Color::Gray),
            status_bar: Style::default().fg(Color::White).bg(Color::DarkGray),
            diff_addition: Style::default().fg(Color::Green),
            diff_deletion: Style::default().fg(Color::Red),
            diff_header: Style::default().fg(Color::Cyan),
        }
    }
}

impl Theme {
    pub fn from_config(config: &ThemeConfig) -> Self {
        Self {
            active_border: Style::default().fg(parse_color(&config.active_border_color)),
            highlight: Style::default()
                .fg(parse_color(&config.highlight_color))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(parse_color(&config.muted_color)),
            success: Style::default().fg(parse_color(&config.success_color)),
            error: Style::default().fg(parse_color(&config.error_color)),
            merged: Style::default().fg(parse_color(&config.merged_color)),
            tab_active: Style::default()
                .fg(parse_color(&config.active_border_color))
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            tab_inactive: Style::default().fg(parse_color(&config.muted_color)),
            status_bar: Style::default().fg(Color::White).bg(Color::DarkGray),
            diff_addition: Style::default().fg(parse_color(&config.diff_addition_color)),
            diff_deletion: Style::default().fg(parse_color(&config.diff_deletion_color)),
            diff_header: Style::default().fg(parse_color(&config.diff_header_color)),
        }
    }
}

/// Returns the style for an open/closed/merged state.
pub fn status_style(state: &str, theme: &Theme) -> Style {
    match state {
        "open" => theme.success,
        "merged" => theme.merged,
        "closed" => theme.error,
        _ => theme.error,
    }
}

fn parse_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" | "purple" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "white" => Color::White,
        _ => Color::Cyan,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_style_open_returns_success() {
        let theme = Theme::default();
        assert_eq!(status_style("open", &theme), theme.success);
    }

    #[test]
    fn status_style_closed_returns_error() {
        let theme = Theme::default();
        assert_eq!(status_style("closed", &theme), theme.error);
    }

    #[test]
    fn status_style_merged_returns_merged_style() {
        let theme = Theme::default();
        assert_eq!(status_style("merged", &theme), theme.merged);
    }

    #[test]
    fn status_style_unknown_returns_error() {
        let theme = Theme::default();
        assert_eq!(status_style("unknown", &theme), theme.error);
    }

    #[test]
    fn merged_color_default_is_magenta() {
        let theme = Theme::default();
        assert_eq!(theme.merged.fg, Some(Color::Magenta));
    }

    #[test]
    fn from_config_merged_color_is_configurable() {
        let config = ThemeConfig {
            merged_color: "blue".to_string(),
            ..ThemeConfig::default()
        };
        let theme = Theme::from_config(&config);
        assert_eq!(theme.merged.fg, Some(Color::Blue));
    }

    #[test]
    fn from_config_merged_color_default_is_magenta() {
        let config = ThemeConfig::default();
        let theme = Theme::from_config(&config);
        assert_eq!(theme.merged.fg, Some(Color::Magenta));
    }
}