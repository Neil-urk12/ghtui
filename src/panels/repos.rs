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
