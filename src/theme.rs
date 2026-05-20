use ratatui::prelude::*;

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
