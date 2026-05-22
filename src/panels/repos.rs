use ratatui::prelude::*;
use ratatui::widgets::{Cell, Row};

use crate::domain::Repo;
use crate::cmd::Cmd;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::util::format;
use crate::widgets::detail_pane::DetailPane;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::{render_list_view, LeftContent, ListViewConfig};
use super::panel::{Panel, SelectionState, handle_key_default};

#[derive(Debug, Default)]
pub enum RepoSort {
    #[default]
    Name,
    Stars,
    Updated,
}

#[derive(Debug)]
pub struct ReposPanel {
    pub repos: Vec<Repo>,
    pub filtered_indices: Vec<usize>,
    pub loading: bool,
    pub sort: RepoSort,
}

impl Default for ReposPanel {
    fn default() -> Self {
        Self {
            repos: Vec::new(),
            filtered_indices: Vec::new(),
            loading: false,
            sort: RepoSort::Name,
        }
    }
}

impl ReposPanel {
    pub fn set_repos(&mut self, repos: Vec<Repo>, selection: &mut SelectionState) {
        self.repos = repos;
        self.filtered_indices = (0..self.repos.len()).collect();
        self.sort_indices();
        selection.select(if self.filtered_indices.is_empty() { None } else { Some(0) });
        selection.set_len(self.filtered_indices.len());
    }

    fn sort_indices(&mut self) {
        match self.sort {
            RepoSort::Name => self
                .filtered_indices
                .sort_by(|&a, &b| self.repos[a].name.cmp(&self.repos[b].name)),
            RepoSort::Stars => self.filtered_indices.sort_by(|&a, &b| {
                self.repos[b]
                    .stars
                    .cmp(&self.repos[a].stars)
                    .then(self.repos[a].name.cmp(&self.repos[b].name))
            }),
            RepoSort::Updated => self.filtered_indices.sort_by(|&a, &b| {
                self.repos[b]
                    .updated_at
                    .cmp(&self.repos[a].updated_at)
                    .then(self.repos[a].name.cmp(&self.repos[b].name))
            }),
        }
    }

    pub fn selected_repo(&self, selection: &SelectionState) -> Option<&Repo> {
        let idx = selection.selected()?;
        self.filtered_indices.get(idx).map(|&i| &self.repos[i])
    }

    pub fn cycle_sort(&mut self) {
        self.sort = match self.sort {
            RepoSort::Name => RepoSort::Stars,
            RepoSort::Stars => RepoSort::Updated,
            RepoSort::Updated => RepoSort::Name,
        };
        self.sort_indices();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn view(
        frame: &mut Frame,
        area: Rect,
        state: &ReposPanel,
        selection: &mut SelectionState,
        detail_pane: &DetailPane,
        focused: bool,
        theme: &Theme,
        tick: u64,
    ) {
        let rows: Vec<Row> = state
            .filtered_indices
            .iter()
            .map(|&i| {
                let repo = &state.repos[i];
                let stars = format::abbreviate_number(repo.stars as u64);
                let lang = repo.language.as_deref().unwrap_or("-");
                let updated = format::relative_time(&repo.updated_at);
                Row::new(vec![
                    Cell::from(repo.name.clone()),
                    Cell::from(lang),
                    Cell::from(stars),
                    Cell::from(updated),
                ])
            })
            .collect();

        let detail_lines = if let Some(repo) = state.selected_repo(selection) {
            let mut lines = vec![
                Line::from(Span::styled(
                    repo.name.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
            ];
            if let Some(desc) = &repo.description {
                lines.push(Line::from(desc.as_str()));
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
        } else {
            vec![]
        };

        let config = ListViewConfig {
            split_ratio: (45, 55),
            margin: 0,
            focused,
            block_title: " Repositories ".to_string(),
            wrap_full: false,
            loading: state.loading,
            empty_state: EmptyState::new("No repositories found")
                .icon(" ")
                .hint("Press `r` to refresh"),
            detail_title: " Details ".to_string(),
            highlight_symbol: "▸ ".to_string(),
            detail_empty_state: EmptyState::new("Select a repository")
                .icon(" ")
                .hint("j/k to navigate, Enter to open"),
        };

        let left = LeftContent::Table {
            headers: vec!["Name", "Lang", "Stars", "Updated"],
            rows,
            widths: vec![
                Constraint::Fill(3),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ],
        };

        render_list_view(frame, area, config, left, selection, detail_pane, detail_lines, theme, tick);
    }
}

impl Panel for ReposPanel {
    fn fetch_cmd(&self) -> Cmd {
        Cmd::FetchRepos
    }

    fn load_data(&mut self, payload: ApiPayload, selection: &mut SelectionState) {
        if let ApiPayload::Repos(repos) = payload {
            self.loading = false;
            self.set_repos(repos, selection);
        }
    }

    fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    fn handle_key(&mut self, key: char, selection: &mut SelectionState, detail_pane: &mut DetailPane) -> Option<Cmd> {
        match key {
            's' => { self.cycle_sort(); None }
            '\n' => {
                if let Some(repo) = self.selected_repo(selection) {
                    let parts: Vec<&str> = repo.name.splitn(2, '/').collect();
                    if parts.len() == 2 {
                        return Some(Cmd::OpenRepoDetail {
                            owner: parts[0].to_string(),
                            repo: parts[1].to_string(),
                        });
                    }
                }
                None
            }
            _ => handle_key_default(self, key, selection, detail_pane),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, selection: &mut SelectionState, detail_pane: &DetailPane, focused: bool, theme: &Theme, tick: u64) {
        Self::view(frame, area, self, selection, detail_pane, focused, theme, tick);
    }
}
