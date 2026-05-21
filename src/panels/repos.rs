use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Row, StatefulWidget, Table};

use crate::domain::Repo;
use crate::cmd::Cmd;
use crate::theme::Theme;
use crate::util::format;
use crate::widgets::{detail_pane::DetailPane, empty_state::EmptyStateWidget, loading::LoadingWidget};
use super::panel::Panel;

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
    pub list_state: ratatui::widgets::TableState,
    pub loading: bool,
    pub detail_pane: DetailPane,
    pub sort: RepoSort,
}

impl Default for ReposPanel {
    fn default() -> Self {
        let mut list_state = ratatui::widgets::TableState::default();
        list_state.select(Some(0));
        Self {
            repos: Vec::new(),
            filtered_indices: Vec::new(),
            list_state,
            loading: false,
            detail_pane: DetailPane::new(),
            sort: RepoSort::Name,
        }
    }
}

impl ReposPanel {
    pub fn set_repos(&mut self, repos: Vec<Repo>) {
        self.repos = repos;
        self.filtered_indices = (0..self.repos.len()).collect();
        self.sort_indices();
        self.list_state.select(if self.filtered_indices.is_empty() { None } else { Some(0) });
        self.detail_pane.reset_scroll();
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

    pub fn selected_repo(&self) -> Option<&Repo> {
        let idx = self.list_state.selected()?;
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

    pub fn view(
        frame: &mut Frame,
        area: Rect,
        state: &ReposPanel,
        focused: bool,
        theme: &Theme,
        tick: u64,
    ) {
        let [list_area, detail_area] =
            Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
                .areas(area);

        let border_style = if focused {
            theme.active_border
        } else {
            theme.inactive_border
        };
        let block = Block::default()
            .title(" Repositories ")
            .borders(Borders::ALL)
            .border_style(border_style);

        if state.loading {
            LoadingWidget::new().render(frame, list_area, tick, block);
            return;
        }

        if state.repos.is_empty() {
            EmptyStateWidget::render(frame, list_area, "No repositories found", block);
        } else {
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

            let widths = [
                Constraint::Fill(3),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ];
            let header = Row::new(["Name", "Lang", "Stars", "Updated"]).style(theme.muted);
            let table = Table::new(rows, widths)
                .header(header)
                .block(block)
                .row_highlight_style(theme.highlight);
            StatefulWidget::render(table, list_area, frame.buffer_mut(), &mut state.list_state.clone());
        }

        // Detail pane
        let detail_block = Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(border_style);

        if let Some(repo) = state.selected_repo() {
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

            state
                .detail_pane
                .render(frame, detail_area, " Details ", lines, detail_block);
        } else {
            EmptyStateWidget::render(
                frame,
                detail_area,
                "Select a repository",
                detail_block,
            );
        }
    }
}

impl Panel for ReposPanel {
    fn detail_pane(&mut self) -> &mut DetailPane {
        &mut self.detail_pane
    }

    fn list_len(&self) -> usize {
        self.filtered_indices.len()
    }

    fn selected_index(&self) -> Option<usize> {
        self.list_state.selected()
    }

    fn set_selected(&mut self, index: Option<usize>) {
        self.list_state.select(index);
    }

    fn handle_key(&mut self, key: char) -> Option<Cmd> {
        match key {
            's' => { self.cycle_sort(); None }
            '\n' => {
                if let Some(repo) = self.selected_repo() {
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
            _ => Panel::handle_key(self, key),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool, theme: &Theme, tick: u64) {
        Self::view(frame, area, self, focused, theme, tick);
    }
}
