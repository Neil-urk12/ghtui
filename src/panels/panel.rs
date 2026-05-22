use ratatui::prelude::*;
use ratatui::widgets::{ListState, TableState};

use crate::cmd::Cmd;
use crate::event::ApiPayload;
use crate::theme::Theme;
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

pub fn scroll_detail_up(detail_pane: &mut DetailPane) {
    detail_pane.scroll_up();
}

pub fn scroll_detail_down(detail_pane: &mut DetailPane) {
    detail_pane.scroll_down();
}

// --- Panel trait ---

/// Common interface for all panels.
pub trait Panel: std::fmt::Debug {
    fn handle_key(&mut self, key: char, selection: &mut SelectionState, detail_pane: &mut DetailPane) -> Option<Cmd>;
    #[allow(clippy::too_many_arguments)]
    fn render(&self, frame: &mut Frame, area: Rect, selection: &mut SelectionState, detail_pane: &DetailPane, focused: bool, theme: &Theme, tick: u64);
    fn fetch_cmd(&self) -> Cmd { Cmd::None }
    fn set_loading(&mut self, _loading: bool) {}
    fn set_repo_context(&mut self, _owner: &str, _repo: &str) {}
    fn load_data(&mut self, payload: ApiPayload, selection: &mut SelectionState);
}

/// Default key handling for panels. Call this from panel `handle_key` impls
/// to delegate unhandled keys (j/k navigation). Avoids UFCS recursion.
pub fn handle_key_default(_panel: &mut dyn Panel, key: char, selection: &mut SelectionState, detail_pane: &mut DetailPane) -> Option<Cmd> {
    match key {
        'j' => {
            navigate_down(selection, detail_pane);
            None
        }
        'k' => {
            navigate_up(selection, detail_pane);
            None
        }
        _ => None,
    }
}
