use ratatui::prelude::*;
use ratatui::widgets::{ListState, TableState};

use crate::cmd::Cmd;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::widgets::detail_pane::DetailPane;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::{LeftContent, ListViewConfig, render_list_view};

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

    #[derive(Debug, Default)]
    struct DummyItem;

    impl PanelItem for DummyItem {
        type Item = String;

        fn left_content<'a>(&'a self, _data: &'a ListPanelData<Self::Item>, _theme: &'a Theme) -> LeftContent<'a> {
            LeftContent::List { items: vec![] }
        }

        fn detail_lines(&self, _item: &Self::Item, _theme: &Theme) -> Vec<Line<'static>> {
            vec![]
        }

        fn handle_key(&mut self, _key: char, _selected: Option<&Self::Item>, _data: &ListPanelData<Self::Item>) -> Option<Cmd> {
            None
        }

        fn fetch_cmd(&self, _data: &ListPanelData<Self::Item>) -> Cmd {
            Cmd::None
        }

        fn block_title(&self, _data: &ListPanelData<Self::Item>) -> String {
            "Dummy".into()
        }

        fn empty_state(&self, _data: &ListPanelData<Self::Item>) -> EmptyState {
            EmptyState::new("Nothing")
        }

        fn extract_items(_payload: ApiPayload) -> Option<Vec<Self::Item>> {
            None
        }
    }

    #[test]
    fn list_panel_default_construction() {
        let panel: ListPanel<DummyItem> = ListPanel::default();
        assert!(panel.data.items.is_empty());
        assert!(!panel.data.loading);
        assert!(panel.data.owner.is_none());
        assert!(panel.data.repo.is_none());
    }

    #[test]
    fn list_panel_set_loading() {
        let mut panel: ListPanel<DummyItem> = ListPanel::default();
        Panel::set_loading(&mut panel, true);
        assert!(panel.data.loading);
        Panel::set_loading(&mut panel, false);
        assert!(!panel.data.loading);
    }

    #[test]
    fn list_panel_set_repo_context() {
        let mut panel: ListPanel<DummyItem> = ListPanel::default();
        Panel::set_repo_context(&mut panel, "owner", "repo");
        assert_eq!(panel.data.owner.as_deref(), Some("owner"));
        assert_eq!(panel.data.repo.as_deref(), Some("repo"));
    }

    #[test]
    fn list_panel_fetch_cmd_delegates() {
        let panel: ListPanel<DummyItem> = ListPanel::default();
        assert!(matches!(Panel::fetch_cmd(&panel), Cmd::None));
    }

    #[test]
    fn list_panel_handle_key_navigation() {
        let mut panel: ListPanel<DummyItem> = ListPanel::default();
        // Populate via set_items which also sets selection len.
        panel.data.set_items(vec!["a".into(), "b".into(), "c".into()], &mut panel.selection);

        // j should move down.
        let result = Panel::handle_key(&mut panel, 'j');
        assert!(result.is_none());

        // k should move up.
        let result = Panel::handle_key(&mut panel, 'k');
        assert!(result.is_none());
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

// --- Panel trait ---

/// Common interface for all panels.
pub trait Panel: std::fmt::Debug {
    fn handle_key(&mut self, key: char) -> Option<Cmd>;
    #[allow(clippy::too_many_arguments)]
    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool, theme: &Theme, tick: u64);
    fn fetch_cmd(&self) -> Cmd { Cmd::None }
    fn set_loading(&mut self, _loading: bool) {}
    fn set_repo_context(&mut self, _owner: &str, _repo: &str) {}
    fn load_data(&mut self, payload: ApiPayload);
    fn scroll_detail_up(&mut self) {}
    fn scroll_detail_down(&mut self) {}
}

// --- PanelItem trait + ListPanel<T> ---

/// Deeper panel trait that owns its own `ListPanelData`, `SelectionState`,
/// and `DetailPane` internally.  Implementors only need to describe content
/// and key-handling; boilerplate (j/k nav, loading, repo context, render)
/// is handled by the generic `ListPanel<T>` wrapper.
///
/// During Phase 1 the old `Panel` trait and `ListPanel<T>` coexist.
pub trait PanelItem: std::fmt::Debug + Default {
    type Item: std::fmt::Debug;

    // --- Required ----------------------------------------------------------

    /// Build the left-side content (list or table rows) from data.
    fn left_content<'a>(&'a self, data: &'a ListPanelData<Self::Item>, theme: &'a Theme) -> LeftContent<'a>;

    /// Build detail lines for the selected item.
    fn detail_lines(&self, item: &Self::Item, theme: &Theme) -> Vec<Line<'static>>;

    /// Handle a key press.  Return `Some(Cmd)` if consumed, `None` to fall
    /// through to default j/k navigation.
    fn handle_key(&mut self, key: char, selected: Option<&Self::Item>, data: &ListPanelData<Self::Item>) -> Option<Cmd>;

    /// Return the fetch command for this panel (e.g. `Cmd::FetchIssues`).
    fn fetch_cmd(&self, data: &ListPanelData<Self::Item>) -> Cmd;

    /// Block title shown around the list area.
    fn block_title(&self, data: &ListPanelData<Self::Item>) -> String;

    /// Empty state shown when the item list is empty.
    fn empty_state(&self, data: &ListPanelData<Self::Item>) -> EmptyState;

    /// Try to extract items of this panel's type from an `ApiPayload`.
    /// Return `None` if the payload variant doesn't match.
    fn extract_items(payload: ApiPayload) -> Option<Vec<Self::Item>>;

    // --- Optional (with defaults) -----------------------------------------

    /// Called after items are loaded.  Override to post-process.
    fn on_loaded(&mut self, _data: &mut ListPanelData<Self::Item>) {}

    /// Left/right split ratio.  Default `(50, 50)`.
    fn split_ratio(&self) -> (u16, u16) { (50, 50) }

    /// Detail pane title.  Default `" Details "`.
    fn detail_title(&self) -> String { " Details ".into() }

    /// Wrap full (block wraps entire area, split inside). Default `true`.
    fn wrap_full(&self) -> bool { true }

    /// Highlight symbol for selected row. Default `"▸ "`.
    fn highlight_symbol(&self) -> String { "▸ ".into() }

    /// Initial selection state. Override for table-based panels.
    /// Default: `SelectionState::list()`.
    fn initial_selection(&self) -> SelectionState { SelectionState::list() }

    /// Empty state for the detail pane when nothing is selected.
    fn detail_empty_state(&self) -> EmptyState {
        EmptyState::new("Select an item")
            .hint("j/k to navigate")
    }
}

/// Generic list panel that owns selection + detail pane internally.
/// Wraps any `PanelItem` implementation and delegates the old `Panel` trait
/// to the inner item's methods plus boilerplate navigation.
pub struct ListPanel<T: PanelItem> {
    pub data: ListPanelData<T::Item>,
    pub selection: SelectionState,
    pub detail_pane: DetailPane,
    pub item: T,
}

impl<T: PanelItem> std::fmt::Debug for ListPanel<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListPanel")
            .field("data", &self.data)
            .field("item", &self.item)
            .field("loading", &self.data.loading)
            .finish_non_exhaustive()
    }
}

impl<T: PanelItem> Default for ListPanel<T> {
    fn default() -> Self {
        let item = T::default();
        let selection = item.initial_selection();
        Self {
            data: ListPanelData::default(),
            selection,
            detail_pane: DetailPane::new(),
            item,
        }
    }
}

impl<T: PanelItem + 'static> Panel for ListPanel<T> {
    fn set_loading(&mut self, loading: bool) {
        self.data.loading = loading;
    }

    fn set_repo_context(&mut self, owner: &str, repo: &str) {
        self.data.set_repo(owner.to_string(), repo.to_string());
    }

    fn load_data(&mut self, payload: ApiPayload) {
        if let Some(items) = T::extract_items(payload) {
            self.data.load_items(items, &mut self.selection);
            self.item.on_loaded(&mut self.data);
        }
    }

    fn handle_key(&mut self, key: char) -> Option<Cmd> {
        // Try the PanelItem's custom key handler first.
        let selected = self.data.selected(&self.selection);
        if let Some(cmd) = self.item.handle_key(key, selected, &self.data) {
            return Some(cmd);
        }
        // Fall through to default j/k navigation.
        match key {
            'j' => { navigate_down(&mut self.selection, &mut self.detail_pane); None }
            'k' => { navigate_up(&mut self.selection, &mut self.detail_pane); None }
            _ => None,
        }
    }

    fn fetch_cmd(&self) -> Cmd {
        self.item.fetch_cmd(&self.data)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool, theme: &Theme, tick: u64) {
        let left = self.item.left_content(&self.data, theme);
        let (sr_left, sr_right) = self.item.split_ratio();

        let mut detail_lines: Vec<Line<'static>> = Vec::new();
        if let Some(item) = self.data.selected(&self.selection) {
            detail_lines = self.item.detail_lines(item, theme);
        }

        let config = ListViewConfig {
            split_ratio: (sr_left, sr_right),
            margin: 1,
            focused,
            block_title: self.item.block_title(&self.data),
            wrap_full: self.item.wrap_full(),
            loading: self.data.loading,
            empty_state: self.item.empty_state(&self.data),
            detail_title: self.item.detail_title(),
            highlight_symbol: self.item.highlight_symbol(),
            detail_empty_state: self.item.detail_empty_state(),
        };

        render_list_view(frame, area, config, left, &mut self.selection, &self.detail_pane, detail_lines, theme, tick);
    }

    fn scroll_detail_up(&mut self) {
        self.detail_pane.scroll_up();
    }

    fn scroll_detail_down(&mut self) {
        self.detail_pane.scroll_down();
    }
}
