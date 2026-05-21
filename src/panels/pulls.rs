use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Row, StatefulWidget, Table};

use crate::domain::PullRequest;
use crate::cmd::Cmd;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::util::format;
use crate::widgets::{detail_pane::DetailPane, empty_state::EmptyStateWidget, loading::LoadingWidget};
use super::panel::{Panel, handle_key_default};

#[derive(Debug)]
pub struct PullsPanel {
    pub pulls: Vec<PullRequest>,
    pub filtered_indices: Vec<usize>,
    pub list_state: ratatui::widgets::TableState,
    pub loading: bool,
    pub detail_pane: DetailPane,
    pub state_filter: String, // "open", "closed", "all"
    pub owner: Option<String>,
    pub repo: Option<String>,
}

impl Default for PullsPanel {
    fn default() -> Self {
        let mut list_state = ratatui::widgets::TableState::default();
        list_state.select(Some(0));
        Self {
            pulls: Vec::new(),
            filtered_indices: Vec::new(),
            list_state,
            loading: false,
            detail_pane: DetailPane::new(),
            state_filter: "open".to_string(),
            owner: None,
            repo: None,
        }
    }
}

impl PullsPanel {
    pub fn set_pulls(&mut self, pulls: Vec<PullRequest>) {
        self.pulls = pulls;
        self.filtered_indices = (0..self.pulls.len()).collect();
        self.list_state.select(if self.filtered_indices.is_empty() { None } else { Some(0) });
        self.detail_pane.reset_scroll();
    }

    pub fn set_repo(&mut self, owner: String, repo: String) {
        self.owner = Some(owner);
        self.repo = Some(repo);
    }

    pub fn selected_pr(&self) -> Option<&PullRequest> {
        let idx = self.list_state.selected()?;
        self.filtered_indices.get(idx).map(|&i| &self.pulls[i])
    }

    pub fn view(
        frame: &mut Frame,
        area: Rect,
        state: &PullsPanel,
        focused: bool,
        theme: &Theme,
        tick: u64,
    ) {
        let [list_area, detail_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(area);

        let border_style = if focused {
            theme.active_border
        } else {
            theme.inactive_border
        };

        let title = match (&state.owner, &state.repo) {
            (Some(o), Some(r)) => format!(" PRs: {o}/{r} "),
            _ => " Pull Requests ".to_string(),
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        if state.loading {
            LoadingWidget::new().render(frame, list_area, tick, block);
            return;
        }

        if state.pulls.is_empty() {
            let msg = if state.owner.is_some() {
                "No pull requests found"
            } else {
                "Select a repository first (Enter on a repo)"
            };
            EmptyStateWidget::render(frame, list_area, msg, block);
        } else {
            let rows: Vec<Row> = state
                .filtered_indices
                .iter()
                .map(|&i| {
                    let pr = &state.pulls[i];
                    let status_str = if pr.state == "closed" { "closed" } else { "open" };
                    let status_style = if pr.state == "closed" { theme.error } else { theme.success };
                    let updated = format::relative_time(&pr.updated_at);
                    Row::new(vec![
                        Cell::from(format!("#{}", pr.number)),
                        Cell::from(pr.title.clone()),
                        Cell::from(pr.author.clone()),
                        Cell::from(Span::styled(status_str, status_style)),
                        Cell::from(updated),
                    ])
                })
                .collect();

            let widths = [
                Constraint::Length(6),
                Constraint::Fill(3),
                Constraint::Fill(1),
                Constraint::Length(8),
                Constraint::Fill(1),
            ];
            let header =
                Row::new(["#", "Title", "Author", "Status", "Updated"]).style(theme.muted);
            let table = Table::new(rows, widths)
                .header(header)
                .block(block)
                .row_highlight_style(theme.highlight);
            StatefulWidget::render(
                table,
                list_area,
                frame.buffer_mut(),
                &mut state.list_state.clone(),
            );
        }

        // Detail pane
        let detail_block = Block::default()
            .title(" PR Details ")
            .borders(Borders::ALL)
            .border_style(border_style);

        if let Some(pr) = state.selected_pr() {
            let mut lines = vec![
                Line::from(Span::styled(
                    format!("#{} {}", pr.number, pr.title),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(format!("by {} | opened {} | {}", pr.author, format::relative_time(&pr.created_at), format::relative_time(&pr.updated_at))),
                Line::from(""),
            ];

            if !pr.labels.is_empty() {
                let labels: Vec<Span> = pr
                    .labels
                    .iter()
                    .map(|l| Span::styled(format!("[{l}]"), Style::default().fg(Color::Yellow)))
                    .collect();
                let mut label_line = vec![Span::raw("Labels: ")];
                label_line.extend(labels);
                lines.push(Line::from(label_line));
                lines.push(Line::from(""));
            }

            if let Some(body) = &pr.body {
                let rendered = crate::util::markdown::render_markdown(body);
                lines.extend(rendered);
            } else {
                lines.push(Line::from(Span::styled(
                    "(no description)",
                    theme.muted,
                )));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "1/2/3: filter | o: open | c: checkout | a: approve | m: merge",
                theme.muted,
            )));

            state
                .detail_pane
                .render(frame, detail_area, " PR Details ", lines, detail_block);
        } else {
            EmptyStateWidget::render(
                frame,
                detail_area,
                "Select a pull request",
                detail_block,
            );
        }
    }
}

impl Panel for PullsPanel {
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

    fn fetch_cmd(&self) -> Cmd {
        match (&self.owner, &self.repo) {
            (Some(owner), Some(repo)) => Cmd::FetchPulls {
                owner: owner.clone(),
                repo: repo.clone(),
                state: self.state_filter.clone(),
            },
            _ => Cmd::None,
        }
    }

    fn set_repo_context(&mut self, owner: &str, repo: &str) {
        self.set_repo(owner.to_string(), repo.to_string());
    }

    fn load_data(&mut self, payload: ApiPayload) {
        if let ApiPayload::Pulls(pulls) = payload {
            self.set_pulls(pulls);
        }
    }

    fn handle_key(&mut self, key: char) -> Option<Cmd> {
        match key {
            '1' => {
                self.state_filter = "open".to_string();
                if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                    Some(Cmd::FetchPulls { owner: owner.clone(), repo: repo.clone(), state: "open".into() })
                } else { None }
            }
            '2' => {
                self.state_filter = "closed".to_string();
                if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                    Some(Cmd::FetchPulls { owner: owner.clone(), repo: repo.clone(), state: "closed".into() })
                } else { None }
            }
            '3' => {
                self.state_filter = "all".to_string();
                if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                    Some(Cmd::FetchPulls { owner: owner.clone(), repo: repo.clone(), state: "all".into() })
                } else { None }
            }
            'o' => {
                if let Some(pr) = self.selected_pr() {
                    if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                        let _ = std::process::Command::new("gh")
                            .args(["browse", &format!("{}", pr.number), "--repo", &format!("{}/{}", owner, repo)])
                            .spawn();
                    }
                }
                None
            }
            'a' => {
                if let Some(pr) = self.selected_pr() {
                    if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                        return Some(Cmd::ApprovePr { owner: owner.clone(), repo: repo.clone(), pr_number: pr.number as u64 });
                    }
                }
                None
            }
            'm' => {
                if let Some(pr) = self.selected_pr() {
                    if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                        return Some(Cmd::MergePr { owner: owner.clone(), repo: repo.clone(), pr_number: pr.number as u64 });
                    }
                }
                None
            }
            _ => handle_key_default(self, key),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool, theme: &Theme, tick: u64) {
        Self::view(frame, area, self, focused, theme, tick);
    }
}
