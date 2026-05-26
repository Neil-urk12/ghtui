use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, PanelKind};

pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let panel_hints = match app.active_panel {
        PanelKind::Repos => "Enter: PRs | s: sort",
        PanelKind::Pulls => "1:open 2:closed 3:all | o:browser",
        PanelKind::Issues => "1:open 2:closed 3:all | o:browser",
        PanelKind::Notifications => "a:all/unread",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, PanelKind};
    use crate::config::Config;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn test_app() -> App {
        App::new(Config::default(), Some("test-token".into()))
    }

    fn render_status_bar_to_string(app: &App) -> String {
        let backend = TestBackend::new(120, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_status_bar(f, f.area(), app);
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let mut content = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = &buffer[(x, y)];
                content.push_str(cell.symbol());
            }
            content.push('\n');
        }
        content
    }

    #[test]
    fn pulls_hint_contains_filter_format() {
        let mut app = test_app();
        app.active_panel = PanelKind::Pulls;
        let content = render_status_bar_to_string(&app);
        assert!(
            content.contains("1:open 2:closed 3:all"),
            "Pulls hint should contain '1:open 2:closed 3:all' but was:\n{}",
            content
        );
    }

    #[test]
    fn pulls_hint_contains_browser() {
        let mut app = test_app();
        app.active_panel = PanelKind::Pulls;
        let content = render_status_bar_to_string(&app);
        assert!(
            content.contains("o:browser"),
            "Pulls hint should contain 'o:browser' but was:\n{}",
            content
        );
    }

    #[test]
    fn issues_hint_contains_filter_format() {
        let mut app = test_app();
        app.active_panel = PanelKind::Issues;
        let content = render_status_bar_to_string(&app);
        assert!(
            content.contains("1:open 2:closed 3:all"),
            "Issues hint should contain '1:open 2:closed 3:all' but was:\n{}",
            content
        );
    }

    #[test]
    fn issues_hint_contains_browser() {
        let mut app = test_app();
        app.active_panel = PanelKind::Issues;
        let content = render_status_bar_to_string(&app);
        assert!(
            content.contains("o:browser"),
            "Issues hint should contain 'o:browser' but was:\n{}",
            content
        );
    }

    #[test]
    fn notifications_hint_contains_all_unread() {
        let mut app = test_app();
        app.active_panel = PanelKind::Notifications;
        let content = render_status_bar_to_string(&app);
        assert!(
            content.contains("a:all/unread"),
            "Notifications hint should contain 'a:all/unread' but was:\n{}",
            content
        );
    }
}
