use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, PanelKind};

pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let panel_hints = match app.active_panel {
        PanelKind::Repos => "Enter: PRs | s: sort",
        PanelKind::Pulls => "1/2/3: filter | o: open",
        PanelKind::Issues => "1/2/3: filter | o: open",
        PanelKind::Notifications => "x: read | X: all read",
        PanelKind::Actions => "Enter: expand | v: logs",
    };
    let global_hints = "j/k: nav | Tab: switch | /: search | ?: help | q: quit";

    let rate_str = match (app.rate_remaining, app.rate_reset_secs) {
        (Some(0), _) => {
            if let Some(reset) = app.rate_reset_secs {
                format!(" | API: rate limited ({}s)", reset)
            } else {
                " | API: rate limited".to_string()
            }
        }
        (Some(remaining), _) => format!(" | API: {}", remaining),
        _ => String::new(),
    };

    let hints = format!("{} | {}{}", panel_hints, global_hints, rate_str);

    let status = Paragraph::new(hints)
        .style(app.theme.status_bar)
        .alignment(Alignment::Left);

    frame.render_widget(status, area);
}
