use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};

pub struct EmptyStateWidget;

impl EmptyStateWidget {
    pub fn render(frame: &mut Frame, area: Rect, message: &str, block: Block) {
        let text = format!("\n  {}", message);
        frame.render_widget(
            Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(Color::DarkGray)),
            area,
        );
    }
}
