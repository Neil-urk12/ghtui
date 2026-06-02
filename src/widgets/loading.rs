use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};

const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct LoadingWidget;

impl LoadingWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, tick: u64, block: Block) {
        let idx = (tick as usize) % SPINNER_FRAMES.len();
        let text = format!("{} Loading...", SPINNER_FRAMES[idx]);
        frame.render_widget(Paragraph::new(text).block(block), area);
    }
}
