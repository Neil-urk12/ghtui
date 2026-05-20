use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, PanelKind};

pub fn render_tab_bar(frame: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = PanelKind::ALL
        .iter()
        .map(|panel| {
            let style = if *panel == app.active_panel {
                app.theme.tab_active
            } else {
                app.theme.tab_inactive
            };
            Line::from(format!(" {} ", panel)).style(style)
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().style(app.theme.status_bar))
        .highlight_style(app.theme.tab_active)
        .select(
            PanelKind::ALL
                .iter()
                .position(|&p| p == app.active_panel)
                .unwrap_or(0),
        )
        .divider("|");

    frame.render_widget(tabs, area);
}
