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
use super::panel::{Panel, SelectionState, handle_key_default, ListPanelData};

#[derive(Debug, Default)]
pub enum RepoSort {
    #[default]
    Name,
    Stars,
    Updated,
}

#[derive(Debug)]
pub struct ReposPanel {
    pub data: ListPanelData<Repo>,
    pub sort: RepoSort,
}

impl Default for ReposPanel {
    fn default() -> Self {
        Self {
            data: ListPanelData::new(),
            sort: RepoSort::Name,
        }
    }
}

impl ReposPanel {
    pub fn sort_indices(&mut self) {
        match self.sort {
            RepoSort::Name => self
                .data
                .filtered_indices
                .sort_by(|&a, &b| self.data.items[a].name.cmp(&self.data.items[b].name)),
            RepoSort::Stars => self.data.filtered_indices.sort_by(|&a, &b| {
                self.data.items[b]
                    .stars
                    .cmp(&self.data.items[a].stars)
                    .then(self.data.items[a].name.cmp(&self.data.items[b].name))
            }),
            RepoSort::Updated => self.data.filtered_indices.sort_by(|&a, &b| {
                self.data.items[b]
                    .updated_at
                    .cmp(&self.data.items[a].updated_at)
                    .then(self.data.items[a].name.cmp(&self.data.items[b].name))
            }),
        }
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
            .data
            .filtered_indices
            .iter()
            .map(|&i| {
                let repo = &state.data.items[i];
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

        let detail_lines = if let Some(repo) = state.data.selected(selection) {
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
            loading: state.data.loading,
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
            self.data.loading = false;
            self.data.set_items(repos, selection);
            self.sort_indices();
        }
    }

    fn set_loading(&mut self, loading: bool) {
        self.data.loading = loading;
    }

    fn handle_key(&mut self, key: char, selection: &mut SelectionState, detail_pane: &mut DetailPane) -> Option<Cmd> {
        match key {
            's' => { self.cycle_sort(); None }
            '\n' => {
                if let Some(repo) = self.data.selected(selection) {
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
