use ratatui::prelude::*;

use crate::config::ThemeConfig;

#[derive(Clone)]
pub struct Theme {
    pub active_border: Style,
    pub inactive_border: Style,
    pub highlight: Style,
    pub muted: Style,
    pub success: Style,
    pub error: Style,
    pub warning: Style,
    pub tab_active: Style,
    pub tab_inactive: Style,
    pub status_bar: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            active_border: Style::default().fg(Color::Cyan),
            inactive_border: Style::default().fg(Color::DarkGray),
            highlight: Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            muted: Style::default().fg(Color::Gray),
            success: Style::default().fg(Color::Green),
            error: Style::default().fg(Color::Red),
            warning: Style::default().fg(Color::Yellow),
            tab_active: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            tab_inactive: Style::default().fg(Color::Gray),
            status_bar: Style::default().fg(Color::White).bg(Color::DarkGray),
        }
    }
}

impl Theme {
    pub fn from_config(config: &ThemeConfig) -> Self {
        Self {
            active_border: Style::default().fg(parse_color(&config.active_border_color)),
            inactive_border: Style::default().fg(parse_color(&config.inactive_border_color)),
            highlight: Style::default()
                .fg(parse_color(&config.highlight_color))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(parse_color(&config.muted_color)),
            success: Style::default().fg(parse_color(&config.success_color)),
            error: Style::default().fg(parse_color(&config.error_color)),
            warning: Style::default().fg(parse_color(&config.warning_color)),
            tab_active: Style::default()
                .fg(parse_color(&config.active_border_color))
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            tab_inactive: Style::default().fg(parse_color(&config.muted_color)),
            status_bar: Style::default().fg(Color::White).bg(Color::DarkGray),
        }
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
