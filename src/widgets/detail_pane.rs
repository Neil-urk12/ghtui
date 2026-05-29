use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};

#[derive(Debug, Clone, Default)]
pub struct DetailPane {
    pub scroll: u16,
    pub showing_diff: bool,
}


impl DetailPane {
    pub fn new() -> Self {
        Self { scroll: 0, showing_diff: false }
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn reset_scroll(&mut self) {
        self.scroll = 0;
    }

    pub fn toggle_diff(&mut self) {
        self.showing_diff = !self.showing_diff;
        self.scroll = 0;
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        content: Vec<Line>,
        block: Block,
    ) {
        let paragraph = Paragraph::new(content)
            .block(block.title(title))
            .scroll((self.scroll, 0));
        frame.render_widget(paragraph, area);
    }
}
