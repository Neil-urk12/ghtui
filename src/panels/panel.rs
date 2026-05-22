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

// --- ListPanelData<T> ---

/// Generic data container for list-based panels. Holds items, filtered indices,
/// loading state, and optional repo context. Panels compose with this via field
/// ownership and delegate Panel trait boilerplate to it.
#[derive(Debug)]
pub struct ListPanelData<T> {
    pub items: Vec<T>,
    pub filtered_indices: Vec<usize>,
    pub loading: bool,
    pub owner: Option<String>,
    pub repo: Option<String>,
}

impl<T> Default for ListPanelData<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            filtered_indices: Vec::new(),
            loading: false,
            owner: None,
            repo: None,
        }
    }
}

impl<T> ListPanelData<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace items, reset selection to 0 (or None if empty).
    pub fn set_items(&mut self, items: Vec<T>, selection: &mut SelectionState) {
        self.items = items;
        self.filtered_indices = (0..self.items.len()).collect();
        selection.select(if self.filtered_indices.is_empty() { None } else { Some(0) });
        selection.set_len(self.filtered_indices.len());
    }

    /// Load items and clear loading flag. Convenience wrapper over set_items.
    pub fn load_items(&mut self, items: Vec<T>, selection: &mut SelectionState) {
        self.loading = false;
        self.set_items(items, selection);
    }

    /// Get the selected item through filtered_indices.
    pub fn selected(&self, selection: &SelectionState) -> Option<&T> {
        let idx = selection.selected()?;
        self.filtered_indices.get(idx).map(|&i| &self.items[i])
    }

    /// Set repo context (owner/repo).
    pub fn set_repo(&mut self, owner: String, repo: String) {
        self.owner = Some(owner);
        self.repo = Some(repo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_items_populates_and_resets_selection() {
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();

        data.set_items(vec!["a".into(), "b".into(), "c".into()], &mut selection);

        assert_eq!(data.items.len(), 3);
        assert_eq!(data.filtered_indices, vec![0, 1, 2]);
        assert_eq!(selection.selected(), Some(0));
        assert_eq!(selection.len(), 3);
    }

    #[test]
    fn set_items_empty_clears_selection() {
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();

        data.set_items(vec!["a".into()], &mut selection);
        assert_eq!(selection.selected(), Some(0));

        data.set_items(vec![], &mut selection);
        assert_eq!(selection.selected(), None);
        assert_eq!(selection.len(), 0);
    }

    #[test]
    fn selected_returns_correct_item() {
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();

        data.set_items(vec!["first".into(), "second".into(), "third".into()], &mut selection);

        assert_eq!(data.selected(&selection), Some(&"first".to_string()));

        selection.select(Some(2));
        assert_eq!(data.selected(&selection), Some(&"third".to_string()));
    }

    #[test]
    fn selected_returns_none_out_of_bounds() {
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();

        data.set_items(vec![], &mut selection);
        assert_eq!(data.selected(&selection), None);
    }

    #[test]
    fn selected_through_filtered_indices() {
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();

        data.set_items(vec!["a".into(), "b".into(), "c".into()], &mut selection);
        // Manually set filtered_indices to only show items 0 and 2
        data.filtered_indices = vec![0, 2];
        selection.set_len(2);
        selection.select(Some(1));

        // Should return "c" (index 2 in items)
        assert_eq!(data.selected(&selection), Some(&"c".to_string()));
    }

    #[test]
    fn load_items_sets_loading_false() {
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();
        data.loading = true;

        data.load_items(vec!["a".into()], &mut selection);

        assert!(!data.loading);
        assert_eq!(data.items.len(), 1);
    }

    #[test]
    fn set_repo_stores_context() {
        let mut data = ListPanelData::<String>::new();
        assert!(data.owner.is_none());
        assert!(data.repo.is_none());

        data.set_repo("octocat".to_string(), "hello-world".to_string());
        assert_eq!(data.owner.as_deref(), Some("octocat"));
        assert_eq!(data.repo.as_deref(), Some("hello-world"));
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
