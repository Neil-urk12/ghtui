use ratatui::prelude::*;
use ratatui::widgets::{Cell, Row};

use crate::cmd::Cmd;
use crate::domain::Repo;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::util::format;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::LeftContent;
use super::panel::{ListPanelData, PanelItem, SelectionState};

#[derive(Debug, Default)]
pub enum RepoSort {
    #[default]
    Name,
    Stars,
    Updated,
}

impl RepoSort {
    pub fn sort_indices(&self, items: &[Repo], filtered_indices: &mut [usize]) {
        match self {
            RepoSort::Name => {
                filtered_indices.sort_by(|&a, &b| items[a].name.cmp(&items[b].name))
            }
            RepoSort::Stars => filtered_indices.sort_by(|&a, &b| {
                items[b]
                    .stars
                    .cmp(&items[a].stars)
                    .then(items[a].name.cmp(&items[b].name))
            }),
            RepoSort::Updated => filtered_indices.sort_by(|&a, &b| {
                items[b]
                    .updated_at
                    .cmp(&items[a].updated_at)
                    .then(items[a].name.cmp(&items[b].name))
            }),
        }
    }
}

fn repo_to_row(repo: &Repo) -> Row<'static> {
    let stars = format::abbreviate_number(repo.stars as u64);
    let updated = format::relative_time(&repo.updated_at);
    Row::new(vec![
        Cell::from(repo.name.clone()),
        Cell::from(stars),
        Cell::from(updated),
    ])
}

fn repo_to_detail(repo: &Repo, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            repo.name.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    if let Some(desc) = &repo.description {
        lines.push(Line::from(desc.clone()));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(format!(
        "Language: {}",
        repo.language.as_deref().unwrap_or("N/A")
    )));
    lines.push(Line::from(format!("Stars: {}", repo.stars)));
    lines.push(Line::from(format!("Forks: {}", repo.forks)));
    lines.push(Line::from(format!(
        "Default branch: {}",
        repo.default_branch
    )));
    lines.push(Line::from(format!("Visibility: {}", repo.visibility)));
    lines.push(Line::from(format!(
        "Updated: {}",
        format::relative_time(&repo.updated_at)
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Enter: view PRs | r: refresh | s: sort",
        theme.muted,
    )));
    lines
}

#[derive(Debug, Default)]
pub struct ReposPanelItem {
    pub sort: RepoSort,
}

impl ReposPanelItem {
    pub fn cycle_sort(&mut self, data: &mut ListPanelData<Repo>, selection: &mut SelectionState) {
        self.sort = match self.sort {
            RepoSort::Name => RepoSort::Stars,
            RepoSort::Stars => RepoSort::Updated,
            RepoSort::Updated => RepoSort::Name,
        };
        self.sort.sort_indices(&data.items, &mut data.filtered_indices);
        selection.set_len(data.filtered_indices.len());
    }
}

impl PanelItem for ReposPanelItem {
    type Item = Repo;

    fn left_content<'a>(
        &'a self,
        data: &'a ListPanelData<Self::Item>,
        _theme: &'a Theme,
    ) -> LeftContent<'a> {
        let rows: Vec<Row> = data
            .filtered_indices
            .iter()
            .map(|&i| repo_to_row(&data.items[i]))
            .collect();
        LeftContent::Table {
            headers: vec!["Name", "Stars", "Updated"],
            rows,
            widths: vec![
                Constraint::Percentage(50),
                Constraint::Length(10),
                Constraint::Percentage(20),
            ],
        }
    }

    fn detail_lines(&self, item: &Self::Item, theme: &Theme) -> Vec<Line<'static>> {
        repo_to_detail(item, theme)
    }

    fn handle_key(
        &mut self,
        key: char,
        selected: Option<&Self::Item>,
        _data: &ListPanelData<Self::Item>,
    ) -> Option<Cmd> {
        match key {
            's' => Some(Cmd::CycleRepoSort),
            '\n' => {
                if let Some(repo) = selected {
                    if let Some((owner, repo_name)) = repo.name.split_once('/') {
                        return Some(Cmd::OpenRepoDetail {
                            owner: owner.to_string(),
                            repo: repo_name.to_string(),
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn fetch_cmd(&self, _data: &ListPanelData<Self::Item>) -> Cmd {
        Cmd::FetchRepos
    }

    fn initial_selection(&self) -> SelectionState {
        SelectionState::table()
    }

    fn block_title(&self, _data: &ListPanelData<Self::Item>) -> String {
        format!(" Repos ({:?}) ", self.sort)
    }

    fn empty_state(&self, _data: &ListPanelData<Self::Item>) -> EmptyState {
        EmptyState::new("No Repos Found")
            .icon(" ")
            .hint("Press 'r' to refresh")
    }

    fn extract_items(payload: ApiPayload) -> Option<Vec<Self::Item>> {
        if let ApiPayload::Repos(repos) = payload {
            Some(repos)
        } else {
            None
        }
    }

    fn on_loaded(&mut self, data: &mut ListPanelData<Self::Item>) {
        self.sort.sort_indices(&data.items, &mut data.filtered_indices);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::Cmd;
    use crate::domain::Repo;
    use crate::event::ApiPayload;
    use crate::theme::Theme;
    use crate::widgets::list_view::LeftContent;
    use crate::panels::panel::ListPanelData;

    fn test_repo() -> Repo {
        Repo {
            name: "octocat/hello-world".to_string(),
            description: Some("A test repository".to_string()),
            language: Some("Rust".to_string()),
            stars: 42,
            forks: 10,
            updated_at: "2024-01-15T10:30:00Z".to_string(),
            visibility: "public".to_string(),
            default_branch: "main".to_string(),
        }
    }

    #[test]
    fn left_content_builds_table_structure() {
        let item = ReposPanelItem::default();
        let theme = Theme::default();
        let mut data = ListPanelData::<Repo>::new();
        data.items = vec![
            test_repo(),
            Repo {
                name: "other/lib".to_string(),
                stars: 1500,
                updated_at: "2024-06-01T00:00:00Z".to_string(),
                ..Default::default()
            },
        ];
        data.filtered_indices = vec![0, 1];

        let content = item.left_content(&data, &theme);

        match content {
            LeftContent::Table { headers, rows, widths } => {
                assert_eq!(headers, vec!["Name", "Stars", "Updated"]);
                assert_eq!(rows.len(), 2);
                assert_eq!(widths.len(), 3);
            }
            _ => panic!("Expected LeftContent::Table"),
        }
    }

    #[test]
    fn detail_lines_includes_all_fields() {
        let item = ReposPanelItem::default();
        let repo = test_repo();
        let theme = Theme::default();

        let lines = item.detail_lines(&repo, &theme);

        // Collect all text content from all spans in all lines
        let all_text: String = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
            .collect::<Vec<&str>>()
            .join(" ");

        assert!(all_text.contains("octocat/hello-world"), "should contain name");
        assert!(all_text.contains("A test repository"), "should contain description");
        assert!(all_text.contains("Rust"), "should contain language");
        assert!(all_text.contains("42"), "should contain stars");
        assert!(all_text.contains("Forks: 10"), "should contain forks");
        assert!(all_text.contains("main"), "should contain default branch");
        assert!(all_text.contains("public"), "should contain visibility");
    }

    #[test]
    fn handle_key_s_returns_cycle_repo_sort() {
        let mut item = ReposPanelItem::default();
        let data = ListPanelData::<Repo>::new();

        let cmd = item.handle_key('s', None, &data);

        assert!(matches!(cmd, Some(Cmd::CycleRepoSort)));
    }

    #[test]
    fn handle_key_enter_with_owner_repo_returns_open_repo_detail() {
        let mut item = ReposPanelItem::default();
        let data = ListPanelData::<Repo>::new();
        let repo = test_repo();

        let cmd = item.handle_key('\n', Some(&repo), &data);

        match cmd {
            Some(Cmd::OpenRepoDetail { owner, repo: repo_name }) => {
                assert_eq!(owner, "octocat");
                assert_eq!(repo_name, "hello-world");
            }
            other => panic!("Expected OpenRepoDetail, got {:?}", other),
        }
    }

    #[test]
    fn handle_key_enter_with_malformed_name_returns_none() {
        let mut item = ReposPanelItem::default();
        let data = ListPanelData::<Repo>::new();
        let repo = Repo {
            name: "no-slash-here".to_string(),
            ..Default::default()
        };

        let cmd = item.handle_key('\n', Some(&repo), &data);

        assert!(cmd.is_none());
    }

    #[test]
    fn on_loaded_sorts_by_stars_descending() {
        let mut item = ReposPanelItem { sort: RepoSort::Stars };

        let mut data = ListPanelData::<Repo>::new();
        data.items = vec![
            Repo { name: "a/repo-a".to_string(), stars: 10, ..Default::default() },
            Repo { name: "b/repo-b".to_string(), stars: 50, ..Default::default() },
            Repo { name: "c/repo-c".to_string(), stars: 30, ..Default::default() },
        ];
        data.filtered_indices = vec![0, 1, 2];

        item.on_loaded(&mut data);

        // Should be sorted by stars descending: repo-b (50), repo-c (30), repo-a (10)
        assert_eq!(data.filtered_indices.len(), 3);
        assert_eq!(data.items[data.filtered_indices[0]].name, "b/repo-b");
        assert_eq!(data.items[data.filtered_indices[1]].name, "c/repo-c");
        assert_eq!(data.items[data.filtered_indices[2]].name, "a/repo-a");
    }

    #[test]
    fn extract_items_repos_returns_some() {
        let repos = vec![test_repo()];
        let payload = ApiPayload::Repos(repos);

        let result = ReposPanelItem::extract_items(payload);

        assert!(result.is_some());
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "octocat/hello-world");
    }

    #[test]
    fn extract_items_pulls_returns_none() {
        let payload = ApiPayload::Pulls(vec![]);

        let result = ReposPanelItem::extract_items(payload);

        assert!(result.is_none());
    }

    #[test]
    fn extract_items_empty_repos_returns_some_empty() {
        let result = ReposPanelItem::extract_items(ApiPayload::Repos(vec![]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn left_content_empty_items() {
        let data = ListPanelData::<Repo>::new();
        let item: ReposPanelItem = Default::default();
        let theme = Theme::default();
        let content = item.left_content(&data, &theme);
        match content {
            LeftContent::Table { rows, .. } => assert!(rows.is_empty()),
            LeftContent::List { items, .. } => assert!(items.is_empty()),
        }
    }
}
