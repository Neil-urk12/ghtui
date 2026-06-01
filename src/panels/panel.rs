use ratatui::prelude::*;

use crate::cmd::Cmd;
use crate::event::ApiPayload;
use crate::domain::FilterState;
use crate::theme::Theme;
use crate::widgets::detail_pane::DetailPane;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::{render_list_view, LeftContent, ListViewConfig};

pub use crate::widgets::selection::{navigate_down, navigate_up, SelectionState};
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
        selection.select(if self.filtered_indices.is_empty() {
            None
        } else {
            Some(0)
        });
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

    /// Filter items by a predicate, updating filtered_indices and selection.
    pub fn filter_by(
        &mut self,
        query: &str,
        matches: impl Fn(&T, &str) -> bool,
        selection: &mut SelectionState,
    ) {
        if query.is_empty() {
            self.filtered_indices = (0..self.items.len()).collect();
        } else {
            let q = query.to_lowercase();
            self.filtered_indices = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| matches(item, &q))
                .map(|(i, _)| i)
                .collect();
        }
        selection.select(if self.filtered_indices.is_empty() {
            None
        } else {
            Some(0)
        });
        selection.set_len(self.filtered_indices.len());
    }

    /// Return (owner, repo) if both are set.
    pub fn owner_repo(&self) -> Option<(&str, &str)> {
        match (&self.owner, &self.repo) {
            (Some(o), Some(r)) => Some((o.as_str(), r.as_str())),
            _ => None,
        }
    }

    /// Return a `RepoContext` if both owner and repo are set.
    pub fn repo_context(&self) -> Option<crate::cmd::RepoContext> {
        let (owner, repo) = self.owner_repo()?;
        Some(crate::cmd::RepoContext::new(owner, repo))
    }

    /// Build an empty state that adapts its message based on whether a repo
    /// context is available.
    pub fn repo_aware_empty_state(
        &self,
        has_items_msg: &'static str,
        has_items_icon: &'static str,
        has_items_hint: &'static str,
    ) -> EmptyState {
        if self.owner.is_some() {
            EmptyState::new(has_items_msg)
                .icon(has_items_icon)
                .hint(has_items_hint)
        } else {
            EmptyState::new("Select a repository first")
                .icon(" ")
                .hint("Switch to Repos tab, press Enter on a repo")
        }
    }
}

// --- PanelItem trait + ListPanel<T> ---

/// Deeper panel trait that owns its own `ListPanelData`, `SelectionState`,
/// and `DetailPane` internally.  Implementors only need to describe content
/// and key-handling; boilerplate (j/k nav, loading, repo context, render)
/// is handled by the generic `ListPanel<T>` wrapper.
///
/// This trait pairs with the generic `ListPanel<T>` wrapper which handles
pub trait PanelItem: std::fmt::Debug + Default {
    type Item: std::fmt::Debug;

    // --- Required ----------------------------------------------------------

    /// Build the left-side content (list or table rows) from data.
    fn left_content<'a>(
        &'a self,
        data: &'a ListPanelData<Self::Item>,
        theme: &'a Theme,
    ) -> LeftContent<'a>;

    /// Build detail lines for the selected item.
    fn detail_lines(&self, item: &Self::Item, theme: &Theme) -> Vec<Line<'static>>;

    /// Handle a key press.  Return `Some(Cmd)` if consumed, `None` to fall
    /// through to default j/k navigation.
    fn handle_key(
        &mut self,
        key: char,
        selected: Option<&Self::Item>,
        data: &ListPanelData<Self::Item>,
    ) -> Option<Cmd>;

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
    fn split_ratio(&self) -> (u16, u16) {
        (50, 50)
    }

    /// Detail pane title.  Default `" Details "`.
    fn detail_title(&self, showing_diff: bool) -> String {
        let _ = showing_diff;
        " Details ".into()
    }

    /// Build diff lines for the selected item. Default returns empty vec.
    fn diff_lines(&self, _item: &Self::Item, _theme: &Theme) -> Vec<Line<'static>> {
        vec![]
    }

    /// Wrap full (block wraps entire area, split inside). Default `true`.
    fn wrap_full(&self) -> bool {
        true
    }

    /// Highlight symbol for selected row. Default `"▸ "`.
    fn highlight_symbol(&self) -> String {
        "▸ ".into()
    }

    /// Initial selection state. Override for table-based panels.
    /// Default: `SelectionState::list()`.
    fn initial_selection(&self) -> SelectionState {
        SelectionState::list()
    }

    /// Display title for the selected item. Used in confirmation dialogs.
    /// Default returns `None`.
    fn item_display(&self, _item: &Self::Item) -> Option<String> {
        None
    }

    /// Empty state for the detail pane when nothing is selected.
    fn detail_empty_state(&self) -> EmptyState {
        EmptyState::new("Select an item").hint("j/k to navigate")
    }

    /// Empty state for diff view when files aren't loaded.
    fn diff_empty_state(&self) -> EmptyState {
        EmptyState::new("No diff available").hint("Press Enter to load files")
    }

    /// Filter items in data based on search query. Default: no-op.
    /// Override in concrete panels to filter by searchable fields.
    fn apply_filter(
        &self,
        data: &mut ListPanelData<Self::Item>,
        query: &str,
        selection: &mut SelectionState,
    ) {
        let _ = (data, query, selection);
    }

    /// Called after selection changes (j/k navigation).
    /// Return `Some(Cmd)` to trigger an async fetch (e.g. comments).
    fn on_select(
        &self,
        _selected: Option<&Self::Item>,
        _data: &ListPanelData<Self::Item>,
    ) -> Option<crate::cmd::Cmd> {
        None
    }
}

/// Shared behavior for panels that filter by state (Open/Closed/All).
pub trait StateFilterable {
    /// Returns a reference to the current filter state.
    fn filter_state(&self) -> &FilterState;

    /// Returns a mutable reference to the filter state field.
    fn filter_state_mut(&mut self) -> &mut FilterState;

    /// Sets the filter to `new`. Returns `true` if it changed, `false` if already set.
    fn set_filter(&mut self, new: FilterState) -> bool {
        if *self.filter_state() == new {
            return false;
        }
        *self.filter_state_mut() = new;
        true
    }
}

/// Generic list panel that owns selection + detail pane internally.
/// Wraps any `PanelItem` implementation and delegates navigation and
/// rendering to the inner item's methods plus boilerplate.
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

impl<T: PanelItem + 'static> ListPanel<T> {
    pub fn set_loading(&mut self, loading: bool) {
        self.data.loading = loading;
    }

    pub fn set_repo_context(&mut self, owner: &str, repo: &str) {
        self.data.set_repo(owner.to_string(), repo.to_string());
    }

    pub fn load_data(&mut self, payload: ApiPayload) {
        if let Some(items) = T::extract_items(payload) {
            self.data.load_items(items, &mut self.selection);
            self.item.on_loaded(&mut self.data);
        }
    }

    pub fn handle_key(&mut self, key: char) -> Option<Cmd> {
        // Try the PanelItem's custom key handler first.
        let selected = self.data.selected(&self.selection);
        if let Some(cmd) = self.item.handle_key(key, selected, &self.data) {
            return Some(cmd);
        }
        // Fall through to default j/k navigation.
        match key {
            'j' => {
                navigate_down(&mut self.selection, &mut self.detail_pane);
                let selected = self.data.selected(&self.selection);
                self.item.on_select(selected, &self.data)
            }
            'k' => {
                navigate_up(&mut self.selection, &mut self.detail_pane);
                let selected = self.data.selected(&self.selection);
                self.item.on_select(selected, &self.data)
            }
            _ => None,
        }
    }

    pub fn fetch_cmd(&self) -> Cmd {
        self.item.fetch_cmd(&self.data)
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        focused: bool,
        theme: &Theme,
        tick: u64,
    ) {
        let left = self.item.left_content(&self.data, theme);
        let (sr_left, sr_right) = self.item.split_ratio();

        let mut detail_lines: Vec<Line<'static>> = Vec::new();
        if let Some(item) = self.data.selected(&self.selection) {
            if self.detail_pane.showing_diff {
                detail_lines = self.item.diff_lines(item, theme);
            } else {
                detail_lines = self.item.detail_lines(item, theme);
            }
        }

        let config = ListViewConfig {
            split_ratio: (sr_left, sr_right),
            margin: 1,
            focused,
            block_title: self.item.block_title(&self.data),
            wrap_full: self.item.wrap_full(),
            loading: self.data.loading,
            empty_state: self.item.empty_state(&self.data),
            detail_title: self.item.detail_title(self.detail_pane.showing_diff),
            highlight_symbol: self.item.highlight_symbol(),
            detail_empty_state: if self.detail_pane.showing_diff {
                self.item.diff_empty_state()
            } else {
                self.item.detail_empty_state()
            },
            detail_mode: self.detail_pane.mode,
            theme,
            tick,
        };

        render_list_view(
            frame,
            area,
            config,
            left,
            &mut self.selection,
            &self.detail_pane,
            detail_lines,
        );
    }

    pub fn scroll_detail_up(&mut self) {
        self.detail_pane.scroll_up();
    }

    pub fn scroll_detail_down(&mut self) {
        self.detail_pane.scroll_down();
    }

    pub fn selected_display(&self) -> Option<String> {
        let item = self.data.selected(&self.selection)?;
        self.item.item_display(item)
    }

    pub fn apply_filter(&mut self, query: &str) {
        self.item
            .apply_filter(&mut self.data, query, &mut self.selection);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::Repo;

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

        data.set_items(
            vec!["first".into(), "second".into(), "third".into()],
            &mut selection,
        );

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

        fn left_content<'a>(
            &'a self,
            _data: &'a ListPanelData<Self::Item>,
            _theme: &'a Theme,
        ) -> LeftContent<'a> {
            LeftContent::List { items: vec![] }
        }

        fn detail_lines(&self, _item: &Self::Item, _theme: &Theme) -> Vec<Line<'static>> {
            vec![]
        }

        fn handle_key(
            &mut self,
            _key: char,
            _selected: Option<&Self::Item>,
            _data: &ListPanelData<Self::Item>,
        ) -> Option<Cmd> {
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

        fn apply_filter(
            &self,
            data: &mut ListPanelData<Self::Item>,
            query: &str,
            selection: &mut SelectionState,
        ) {
            data.filter_by(query, |item, q| item.to_lowercase().contains(q), selection);
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
        panel.set_loading(true);
        assert!(panel.data.loading);
        panel.set_loading(false);
        assert!(!panel.data.loading);
    }

    #[test]
    fn list_panel_set_repo_context() {
        let mut panel: ListPanel<DummyItem> = ListPanel::default();
        panel.set_repo_context("owner", "repo");
        assert_eq!(panel.data.owner.as_deref(), Some("owner"));
        assert_eq!(panel.data.repo.as_deref(), Some("repo"));
    }

    #[test]
    fn list_panel_fetch_cmd_delegates() {
        let panel: ListPanel<DummyItem> = ListPanel::default();
        assert!(matches!(panel.fetch_cmd(), Cmd::None));
    }

    #[test]
    fn list_panel_handle_key_navigation() {
        let mut panel: ListPanel<DummyItem> = ListPanel::default();
        // Populate via set_items which also sets selection len.
        panel.data.set_items(
            vec!["a".into(), "b".into(), "c".into()],
            &mut panel.selection,
        );

        // j should move down.
        let result = panel.handle_key('j');
        assert!(result.is_none());

        // k should move up.
        let result = panel.handle_key('k');
        assert!(result.is_none());
    }

    // --- Search filter tests (failing: methods don't exist yet) ---

    #[test]
    fn panel_item_trait_has_apply_filter_method() {
        // DummyItem implements PanelItem, so if apply_filter exists on the
        // trait, this will compile. We also verify the impl does basic
        // substring matching.
        let item = DummyItem;
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();
        data.set_items(
            vec!["foo".into(), "bar".into(), "foobar".into()],
            &mut selection,
        );

        item.apply_filter(&mut data, "foo", &mut selection);

        assert_eq!(data.filtered_indices, vec![0, 2]);
    }

    #[test]
    fn apply_filter_matches_substring() {
        let item = DummyItem;
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();
        data.set_items(
            vec!["foo".into(), "bar".into(), "foobar".into()],
            &mut selection,
        );

        item.apply_filter(&mut data, "foo", &mut selection);

        assert_eq!(data.filtered_indices, vec![0, 2]);
    }

    #[test]
    fn apply_filter_is_case_insensitive() {
        let item = DummyItem;
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();
        data.set_items(
            vec!["Hello".into(), "world".into(), "HELLO".into()],
            &mut selection,
        );

        item.apply_filter(&mut data, "hello", &mut selection);

        assert_eq!(data.filtered_indices, vec![0, 2]);
    }

    #[test]
    fn apply_filter_empty_query_shows_all() {
        let item = DummyItem;
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();
        data.set_items(vec!["a".into(), "b".into(), "c".into()], &mut selection);

        item.apply_filter(&mut data, "", &mut selection);

        assert_eq!(data.filtered_indices, vec![0, 1, 2]);
    }

    #[test]
    fn apply_filter_no_match_empty_result() {
        let item = DummyItem;
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();
        data.set_items(vec!["foo".into(), "bar".into()], &mut selection);

        item.apply_filter(&mut data, "zzz", &mut selection);

        assert!(data.filtered_indices.is_empty());
    }

    #[test]
    fn apply_filter_resets_selection() {
        let item = DummyItem;
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();
        data.set_items(
            vec!["foo".into(), "bar".into(), "foobar".into()],
            &mut selection,
        );

        // Move selection down first.
        selection.select(Some(2));
        assert_eq!(selection.selected(), Some(2));

        // After filtering, selection should reset to 0.
        item.apply_filter(&mut data, "foo", &mut selection);

        assert_eq!(selection.selected(), Some(0));
        assert_eq!(selection.len(), 2);
    }

    #[test]
    fn apply_filter_resets_selection_to_none_when_empty() {
        let item = DummyItem;
        let mut data = ListPanelData::<String>::new();
        let mut selection = SelectionState::list();
        data.set_items(vec!["foo".into(), "bar".into()], &mut selection);

        item.apply_filter(&mut data, "zzz", &mut selection);

        assert_eq!(selection.selected(), None);
        assert_eq!(selection.len(), 0);
    }

    #[test]
    fn list_panel_apply_filter_delegates_to_item() {
        let mut panel: ListPanel<DummyItem> = ListPanel::default();
        panel.data.set_items(
            vec!["foo".into(), "bar".into(), "foobar".into()],
            &mut panel.selection,
        );

        // All 3 items visible initially
        assert_eq!(panel.data.filtered_indices.len(), 3);

        // Call apply_filter through the Panel trait
        panel.apply_filter("foo");

        // Should filter to items containing "foo"
        assert_eq!(panel.data.filtered_indices, vec![0, 2]);

        // Clear filter
        panel.apply_filter("");
        assert_eq!(panel.data.filtered_indices, vec![0, 1, 2]);
    }

    #[test]
    fn list_panel_data_owner_repo_returns_pair() {
        let mut data: ListPanelData<Repo> = ListPanelData::new();
        data.owner = Some("octocat".into());
        data.repo = Some("hello-world".into());
        assert_eq!(data.owner_repo(), Some(("octocat", "hello-world")));
    }

    #[test]
    fn list_panel_data_owner_repo_returns_none_without_both() {
        let mut data: ListPanelData<Repo> = ListPanelData::new();
        data.owner = Some("octocat".into());
        assert_eq!(data.owner_repo(), None);
    }

    #[test]
    fn list_panel_data_owner_repo_returns_none_when_neither() {
        let data: ListPanelData<Repo> = ListPanelData::new();
        assert_eq!(data.owner_repo(), None);
    }

    #[test]
    fn repo_aware_empty_state_with_owner() {
        let mut data: ListPanelData<Repo> = ListPanelData::new();
        data.owner = Some("octocat".into());
        data.repo = Some("hello-world".into());
        let es = data.repo_aware_empty_state("No pulls found", " ", "Hint");
        assert_eq!(es.message, "No pulls found");
    }

    #[test]
    fn repo_aware_empty_state_without_owner() {
        let data: ListPanelData<Repo> = ListPanelData::new();
        let es = data.repo_aware_empty_state("No pulls found", " ", "Hint");
        assert_eq!(es.message, "Select a repository first");
    }

    #[test]
    fn panel_item_default_diff_empty_state() {
        let item = DummyItem;
        let diff_es = item.diff_empty_state();
        assert_eq!(diff_es.message, "No diff available");
    }

    #[test]
    fn repo_context_returns_some_when_both_set() {
        let mut data: ListPanelData<Repo> = ListPanelData::new();
        data.owner = Some("octocat".into());
        data.repo = Some("hello-world".into());
        let ctx = data.repo_context().unwrap();
        assert_eq!(ctx.owner, "octocat");
        assert_eq!(ctx.repo, "hello-world");
    }

    #[test]
    fn repo_context_returns_none_when_owner_missing() {
        let mut data: ListPanelData<Repo> = ListPanelData::new();
        data.repo = Some("hello-world".into());
        assert!(data.repo_context().is_none());
    }

    #[test]
    fn repo_context_returns_none_when_repo_missing() {
        let mut data: ListPanelData<Repo> = ListPanelData::new();
        data.owner = Some("octocat".into());
        assert!(data.repo_context().is_none());
    }

}