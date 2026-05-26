use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

pub const AUTO_DISMISS_TICKS: u64 = 16; // ~4 seconds at 250ms/tick

pub enum Severity {
    Error,
    Warn,
}

pub fn render_error_toast(frame: &mut Frame, area: Rect, message: &str, severity: Severity, tick: u64) {
    let (border_color, icon, title) = match severity {
        Severity::Error => (Color::Red, "✖", " Error "),
        Severity::Warn => (Color::Yellow, "⚠", " Warning "),
    };

    let remaining = AUTO_DISMISS_TICKS.saturating_sub(tick);
    let progress = if remaining > 0 {
        format!(" [{}/{}]", AUTO_DISMISS_TICKS - remaining, AUTO_DISMISS_TICKS)
    } else {
        String::new()
    };

    let toast_area = bottom_right_rect(60, 8, area);
    frame.render_widget(Clear, toast_area);
    let block = Block::default()
        .title(format!("{}{}", title, progress))
        .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let text = Paragraph::new(format!("{} {}", icon, message))
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(text, toast_area);
}

pub fn render_success_toast(frame: &mut Frame, area: Rect, message: &str, tick: u64) {
    let remaining = AUTO_DISMISS_TICKS.saturating_sub(tick);
    let progress = if remaining > 0 {
        format!(" [{}/{}]", AUTO_DISMISS_TICKS - remaining, AUTO_DISMISS_TICKS)
    } else {
        String::new()
    };

    let toast_area = bottom_right_rect(50, 6, area);
    frame.render_widget(Clear, toast_area);
    let block = Block::default()
        .title(format!(" Success{}", progress))
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));
    let text = Paragraph::new(format!("✔ {}", message))
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(text, toast_area);
}

pub fn classify_severity(message: &str) -> Severity {
    let lower = message.to_lowercase();
    if lower.contains("rate limit") || lower.contains("timeout") || lower.contains("network") {
        Severity::Warn
    } else {
        Severity::Error
    }
}

pub fn render_help(frame: &mut Frame, area: Rect, panel: &str) {
    let help_area = centered_rect(70, 60, area);
    frame.render_widget(Clear, help_area);
    let block = Block::default().title(" Help ").borders(Borders::ALL);
    let text = build_help_text(panel);
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
        help_area,
    );
}


pub fn render_confirmation(frame: &mut Frame, area: Rect, action_label: &str, detail: &str) {
    let modal_area = centered_rect(50, 20, area);
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(" Confirm ")
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let mut lines = vec![
        Line::from(action_label.to_string())
            .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ];

    if !detail.is_empty() {
        lines.push(Line::from(detail.to_string()).style(Style::default().fg(Color::Gray)));
    }

    lines.push(Line::from(""));
    lines.push(
        Line::from(vec![
            Span::styled(" y ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" Yes    "),
            Span::styled(" n ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" No"),
        ]),
    );

    let text = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(text, modal_area);
}

fn bottom_right_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Length(1),
        ])
        .split(r);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Percentage(percent_x),
            Constraint::Length(1),
        ])
        .split(popup_layout[1]);
    horizontal[1]
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1]);
    horizontal[1]
}

fn build_help_text(panel: &str) -> Text<'static> {
    let mut lines: Vec<Line> = vec![
        Line::from("Global").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Line::from("  Tab / Shift-Tab  Switch panel"),
        Line::from("  /                Search"),
        Line::from("  ?                Toggle help"),
        Line::from("  r                Refresh"),
        Line::from("  q                Quit"),
        Line::from(""),
    ];

    let panel_specific: Vec<Line> = match panel {
        "Repos" => vec![
            Line::from("Repos").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::from("  j / k            Navigate list"),
            Line::from("  Enter            View PRs for repo"),
            Line::from("  s                Toggle sort"),
        ],
        "PRs" => vec![
            Line::from("Pull Requests").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::from("  j / k            Navigate list"),
            Line::from("  1: open          Show open items"),
            Line::from("  2: closed        Show closed items"),
            Line::from("  3: all           Show all items"),
            Line::from("  o: browse        Open in browser"),
            Line::from("  c                Checkout branch"),
            Line::from("  a                Approve"),
            Line::from("  m                Merge"),
        ],
        "Issues" => vec![
            Line::from("Issues").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::from("  j / k            Navigate list"),
            Line::from("  1: open          Show open items"),
            Line::from("  2: closed        Show closed items"),
            Line::from("  3: all           Show all items"),
            Line::from("  o: browse        Open in browser"),
            Line::from("  c                Add comment"),
            Line::from("  a                Toggle assign"),
        ],
        "Notif" => vec![
            Line::from("Notifications").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::from("  j / k            Navigate list"),
            Line::from("  a: all/unread    Toggle all/unread view"),
            Line::from("  m                Mark as read"),
            Line::from("  M                Mark all as read"),
            Line::from("  Enter            Go to subject"),
        ],
        "Actions" => vec![
            Line::from("Actions").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::from("  j / k            Navigate list"),
            Line::from("  Enter            Expand jobs"),
            Line::from("  v                View logs"),
            Line::from("  r                Re-run"),
            Line::from("  c                Cancel"),
        ],
        _ => vec![],
    };

    lines.extend(panel_specific);
    lines.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render_help_to_string(panel: &str) -> String {
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_help(f, f.area(), panel);
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
    fn prs_help_contains_new_filter_format() {
        let content = render_help_to_string("PRs");
        assert!(
            content.contains("1: open"),
            "PRs help should contain '1: open' but was:\n{}",
            content
        );
        assert!(
            content.contains("2: closed"),
            "PRs help should contain '2: closed' but was:\n{}",
            content
        );
        assert!(
            content.contains("3: all"),
            "PRs help should contain '3: all' but was:\n{}",
            content
        );
        assert!(
            content.contains("o: browse"),
            "PRs help should contain 'o: browse' but was:\n{}",
            content
        );
    }

    #[test]
    fn issues_help_contains_new_filter_format() {
        let content = render_help_to_string("Issues");
        assert!(
            content.contains("1: open"),
            "Issues help should contain '1: open' but was:\n{}",
            content
        );
        assert!(
            content.contains("2: closed"),
            "Issues help should contain '2: closed' but was:\n{}",
            content
        );
        assert!(
            content.contains("3: all"),
            "Issues help should contain '3: all' but was:\n{}",
            content
        );
        assert!(
            content.contains("o: browse"),
            "Issues help should contain 'o: browse' but was:\n{}",
            content
        );
    }

    #[test]
    fn notifications_help_contains_correct_key() {
        let content = render_help_to_string("Notif");
        assert!(
            content.contains("a: all/unread"),
            "Notifications help should contain 'a: all/unread' but was:\n{}",
            content
        );
        assert!(
            content.contains("m                Mark as read"),
            "Notifications help should contain 'm' mark as read but was:\n{}",
            content
        );
        assert!(
            content.contains("M                Mark all as read"),
            "Notifications help should contain 'M' mark all as read but was:\n{}",
            content
        );
    }

    #[test]
    fn notifications_help_does_not_contain_x_binding() {
        let content = render_help_to_string("Notif");
        assert!(
            !content.contains("x                Mark as read"),
            "Notifications help should NOT contain old 'x' binding, got:\n{}",
            content
        );
    }
}
