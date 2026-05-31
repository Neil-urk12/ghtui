use ratatui::widgets::{ListState, TableState};

use crate::widgets::detail_pane::DetailPane;

// --- SelectionState ---

pub struct SelectionStateInner<S> {
    pub state: S,
    len: usize,
}

pub enum SelectionState {
    List(SelectionStateInner<ListState>),
    Table(SelectionStateInner<TableState>),
}

impl SelectionState {
    pub fn list() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        SelectionState::List(SelectionStateInner { state, len: 0 })
    }

    pub fn table() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        SelectionState::Table(SelectionStateInner { state, len: 0 })
    }

    pub fn selected(&self) -> Option<usize> {
        match self {
            SelectionState::List(inner) => inner.state.selected(),
            SelectionState::Table(inner) => inner.state.selected(),
        }
    }

    pub fn select(&mut self, index: Option<usize>) {
        match self {
            SelectionState::List(inner) => inner.state.select(index),
            SelectionState::Table(inner) => inner.state.select(index),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            SelectionState::List(inner) => inner.len,
            SelectionState::Table(inner) => inner.len,
        }
    }

    pub fn set_len(&mut self, new_len: usize) {
        match self {
            SelectionState::List(inner) => inner.len = new_len,
            SelectionState::Table(inner) => inner.len = new_len,
        }
    }
}

// --- Free navigation functions ---

pub fn navigate_up(selection: &mut SelectionState, detail_pane: &mut DetailPane) {
    let i = selection.selected().unwrap_or(0);
    if i > 0 {
        selection.select(Some(i - 1));
        detail_pane.reset_scroll();
    }
}

pub fn navigate_down(selection: &mut SelectionState, detail_pane: &mut DetailPane) {
    let i = selection.selected().unwrap_or(0);
    if i + 1 < selection.len() {
        selection.select(Some(i + 1));
        detail_pane.reset_scroll();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_starts_at_zero() {
        let sel = SelectionState::list();
        assert_eq!(sel.selected(), Some(0));
        assert_eq!(sel.len(), 0);
    }

    #[test]
    fn table_starts_at_zero() {
        let sel = SelectionState::table();
        assert_eq!(sel.selected(), Some(0));
        assert_eq!(sel.len(), 0);
    }

    #[test]
    fn select_and_len_roundtrip() {
        let mut sel = SelectionState::list();
        sel.set_len(5);
        assert_eq!(sel.len(), 5);
        sel.select(Some(3));
        assert_eq!(sel.selected(), Some(3));
    }

    #[test]
    fn navigate_down_advances() {
        let mut sel = SelectionState::list();
        let mut dp = DetailPane::default();
        sel.set_len(3);
        navigate_down(&mut sel, &mut dp);
        assert_eq!(sel.selected(), Some(1));
    }

    #[test]
    fn navigate_down_stops_at_end() {
        let mut sel = SelectionState::list();
        let mut dp = DetailPane::default();
        sel.set_len(2);
        sel.select(Some(1));
        navigate_down(&mut sel, &mut dp);
        assert_eq!(sel.selected(), Some(1)); // didn't move
    }

    #[test]
    fn navigate_up_decrements() {
        let mut sel = SelectionState::list();
        let mut dp = DetailPane::default();
        sel.set_len(3);
        sel.select(Some(2));
        navigate_up(&mut sel, &mut dp);
        assert_eq!(sel.selected(), Some(1));
    }

    #[test]
    fn navigate_up_stops_at_zero() {
        let mut sel = SelectionState::list();
        let mut dp = DetailPane::default();
        sel.set_len(3);
        navigate_up(&mut sel, &mut dp);
        assert_eq!(sel.selected(), Some(0)); // didn't move
    }
}
