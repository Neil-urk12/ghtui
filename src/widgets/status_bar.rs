use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;

pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let hints = match app.active_panel {
        _ => "j/k: navigate | Tab: switch panel | r: refresh | q: quit",
    };

    let status = Paragraph::new(hints)
        .style(app.theme.status_bar)
        .alignment(Alignment::Left);

    frame.render_widget(status, area);
}
