use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};

pub struct EmptyState {
    pub message: String,
    pub hint: Option<String>,
    pub icon: &'static str,
}

impl EmptyState {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            hint: None,
            icon: "·",
        }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = icon;
        self
    }
}

pub struct EmptyStateWidget;

impl EmptyStateWidget {
    pub fn render(frame: &mut Frame, area: Rect, state: &EmptyState, block: Block) {
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 1 || inner.width < 4 {
            return;
        }

        let mut lines: Vec<Line> = Vec::new();

        // Vertical centering: pad top by ~1/3 of available space
        let content_lines = if state.hint.is_some() { 3 } else { 1 };
        // content_lines is always 1 or 3 — safe to cast
        let pad_top = (inner.height.saturating_sub(content_lines as u16)) / 3;
        for _ in 0..pad_top {
            lines.push(Line::from(""));
        }

        // Icon + message centered
        let msg_text = format!("{}  {}", state.icon, state.message);
        lines.push(Line::from(Span::styled(
            msg_text,
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )));

        // Hint line
        if let Some(hint) = &state.hint {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {}", hint),
                Style::default().fg(Color::DarkGray),
            )));
        }

        let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
        frame.render_widget(paragraph, inner);
    }
}
