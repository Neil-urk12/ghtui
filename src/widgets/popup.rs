use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

pub fn render_error_toast(frame: &mut Frame, area: Rect, message: &str) {
    let toast_area = centered_rect(50, 10, area);
    frame.render_widget(Clear, toast_area);
    let block = Block::default()
        .title("Error")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));
    let text = Paragraph::new(message)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(text, toast_area);
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

fn build_help_text(panel: &str) -> Vec<Line<'static>> {
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
            Line::from("  1 / 2 / 3        Filter open/closed/all"),
            Line::from("  o                Open in browser"),
            Line::from("  c                Checkout branch"),
            Line::from("  a                Approve"),
            Line::from("  m                Merge"),
        ],
        "Issues" => vec![
            Line::from("Issues").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::from("  j / k            Navigate list"),
            Line::from("  1 / 2 / 3        Filter open/closed/all"),
            Line::from("  o                Open in browser"),
            Line::from("  c                Add comment"),
            Line::from("  a                Toggle assign"),
        ],
        "Notif" => vec![
            Line::from("Notifications").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::from("  j / k            Navigate list"),
            Line::from("  x                Mark as read"),
            Line::from("  X                Mark all as read"),
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
    lines
}
