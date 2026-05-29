use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailPaneMode {
    Normal,
    Fullscreen,
    Collapsed,
}

impl Default for DetailPaneMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Default)]
pub struct DetailPane {
    pub scroll: u16,
    pub showing_diff: bool,
    pub mode: DetailPaneMode,
}


impl DetailPane {
    pub fn new() -> Self {
        Self { scroll: 0, showing_diff: false, mode: DetailPaneMode::Normal }
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

    pub fn toggle_fullscreen(&mut self) {
        match self.mode {
            DetailPaneMode::Fullscreen => self.mode = DetailPaneMode::Normal,
            DetailPaneMode::Collapsed => self.mode = DetailPaneMode::Normal,
            DetailPaneMode::Normal => {
                self.mode = DetailPaneMode::Fullscreen;
                self.scroll = 0;
            }
        }
    }

    pub fn toggle_collapse(&mut self) {
        match self.mode {
            DetailPaneMode::Collapsed => self.mode = DetailPaneMode::Normal,
            DetailPaneMode::Fullscreen => self.mode = DetailPaneMode::Normal,
            DetailPaneMode::Normal => {
                self.mode = DetailPaneMode::Collapsed;
                self.scroll = 0;
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pane_with_mode(mode: DetailPaneMode) -> DetailPane {
        DetailPane { mode, ..DetailPane::default()
        }
    }

    #[test]
    fn default_mode_is_normal() {
        assert_eq!(DetailPaneMode::default(), DetailPaneMode::Normal);
    }

    #[test]
    fn toggle_fullscreen_from_normal() {
        let mut pane = pane_with_mode(DetailPaneMode::Normal);
        pane.toggle_fullscreen();
        assert_eq!(pane.mode, DetailPaneMode::Fullscreen);
        assert_eq!(pane.scroll, 0);
    }

    #[test]
    fn toggle_fullscreen_from_fullscreen() {
        let mut pane = pane_with_mode(DetailPaneMode::Fullscreen);
        pane.toggle_fullscreen();
        assert_eq!(pane.mode, DetailPaneMode::Normal);
    }

    #[test]
    fn toggle_fullscreen_from_collapsed() {
        let mut pane = pane_with_mode(DetailPaneMode::Collapsed);
        pane.toggle_fullscreen();
        assert_eq!(pane.mode, DetailPaneMode::Normal);
    }

    #[test]
    fn toggle_collapse_from_normal() {
        let mut pane = pane_with_mode(DetailPaneMode::Normal);
        pane.toggle_collapse();
        assert_eq!(pane.mode, DetailPaneMode::Collapsed);
        assert_eq!(pane.scroll, 0);
    }

    #[test]
    fn toggle_collapse_from_collapsed() {
        let mut pane = pane_with_mode(DetailPaneMode::Collapsed);
        pane.toggle_collapse();
        assert_eq!(pane.mode, DetailPaneMode::Normal);
    }

    #[test]
    fn toggle_collapse_from_fullscreen() {
        let mut pane = pane_with_mode(DetailPaneMode::Fullscreen);
        pane.toggle_collapse();
        assert_eq!(pane.mode, DetailPaneMode::Normal);
    }

    #[test]
    fn toggle_fullscreen_resets_scroll() {
        let mut pane = pane_with_mode(DetailPaneMode::Normal);
        pane.scroll = 5;
        pane.toggle_fullscreen();
        assert_eq!(pane.scroll, 0);
    }

    #[test]
    fn toggle_collapse_resets_scroll() {
        let mut pane = pane_with_mode(DetailPaneMode::Normal);
        pane.scroll = 5;
        pane.toggle_collapse();
        assert_eq!(pane.scroll, 0);
    }
}
