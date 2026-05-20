use ratatui::prelude::*;
use crate::cmd::Cmd;
use crate::theme::Theme;
use crate::widgets::detail_pane::DetailPane;

/// Common interface for all panels.
///
/// Provided methods handle j/k navigation and detail pane scrolling.
/// Panels implement `handle_key` for their specific keys and `render` for drawing.
pub trait Panel: std::fmt::Debug {
    /// Mutable access to the detail pane (for scroll)
    fn detail_pane(&mut self) -> &mut DetailPane;

    /// Number of items in the list
    fn list_len(&self) -> usize;

    /// Currently selected index
    fn selected_index(&self) -> Option<usize>;

    /// Set selected index
    fn set_selected(&mut self, index: Option<usize>);

    /// Handle panel-specific key input. Return `Some(Cmd)` for side effects.
    /// Default impl handles `j` (down) and `k` (up).
    fn handle_key(&mut self, key: char) -> Option<Cmd> {
        match key {
            'j' => {
                self.navigate_down();
                None
            }
            'k' => {
                self.navigate_up();
                None
            }
            _ => None,
        }
    }

    /// Render the panel into the given area.
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool, theme: &Theme, tick: u64);

    // --- provided methods ---

    fn navigate_up(&mut self) {
        let i = self.selected_index().unwrap_or(0);
        if i > 0 {
            self.set_selected(Some(i - 1));
            self.detail_pane().reset_scroll();
        }
    }

    fn navigate_down(&mut self) {
        let i = self.selected_index().unwrap_or(0);
        if i + 1 < self.list_len() {
            self.set_selected(Some(i + 1));
            self.detail_pane().reset_scroll();
        }
    }

    fn scroll_detail_up(&mut self) {
        self.detail_pane().scroll_up();
    }

    fn scroll_detail_down(&mut self) {
        self.detail_pane().scroll_down();
    }
}
