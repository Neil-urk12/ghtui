use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};

pub struct LoadingWidget {
    frames: Vec<&'static str>,
}

impl LoadingWidget {
    pub fn new() -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, tick: u64, block: Block) {
        let idx = (tick as usize) % self.frames.len();
        let text = format!("{} Loading...", self.frames[idx]);
        frame.render_widget(Paragraph::new(text).block(block), area);
    }
}
