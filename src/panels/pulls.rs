use ratatui::prelude::*;
use ratatui::widgets::{Cell, Row};

use crate::domain::PullRequest;
use crate::cmd::Cmd;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::util::format;
use crate::widgets::detail_pane::DetailPane;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::{render_list_view, LeftContent, ListViewConfig};
use super::panel::{Panel, SelectionState, handle_key_default, ListPanelData};

#[derive(Debug)]
pub struct PullsPanel {
    pub data: ListPanelData<PullRequest>,
    pub state_filter: String, // "open", "closed", "all"
}

impl Default for PullsPanel {
    fn default() -> Self {
        Self {
            data: ListPanelData::new(),
            state_filter: "open".to_string(),
        }
    }
}

impl PullsPanel {
    #[allow(clippy::too_many_arguments)]
    pub fn view(
        frame: &mut Frame,
        area: Rect,
        state: &PullsPanel,
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
                let pr = &state.data.items[i];
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

        let detail_lines = if let Some(pr) = state.data.selected(selection) {
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

            lines
        } else {
            vec![]
        };

        let block_title = match (&state.data.owner, &state.data.repo) {
            (Some(o), Some(r)) => format!(" PRs: {o}/{r} "),
            _ => " Pull Requests ".to_string(),
        };

        let empty_state = if state.data.owner.is_some() {
            EmptyState::new("No pull requests found")
                .icon(" ")
                .hint("Press `r` to refresh, 1/2/3 to filter")
        } else {
            EmptyState::new("Select a repository first")
                .icon(" ")
                .hint("Switch to Repos tab, press Enter on a repo")
        };

        let config = ListViewConfig {
            split_ratio: (50, 50),
            margin: 0,
            focused,
            block_title,
            wrap_full: false,
            loading: state.data.loading,
            empty_state,
            detail_title: " PR Details ".to_string(),
            highlight_symbol: "▸ ".to_string(),
            detail_empty_state: EmptyState::new("Select a pull request")
                .icon(" ")
                .hint("j/k to navigate"),
        };

        let left = LeftContent::Table {
            headers: vec!["#", "Title", "Author", "Status", "Updated"],
            rows,
            widths: vec![
                Constraint::Length(6),
                Constraint::Fill(3),
                Constraint::Fill(1),
                Constraint::Length(8),
                Constraint::Fill(1),
            ],
        };

        render_list_view(frame, area, config, left, selection, detail_pane, detail_lines, theme, tick);
    }
}

impl Panel for PullsPanel {
    fn fetch_cmd(&self) -> Cmd {
        match (&self.data.owner, &self.data.repo) {
            (Some(owner), Some(repo)) => Cmd::FetchPulls {
                owner: owner.clone(),
                repo: repo.clone(),
                state: self.state_filter.clone(),
            },
            _ => Cmd::None,
        }
    }

    fn set_repo_context(&mut self, owner: &str, repo: &str) {
        self.data.set_repo(owner.to_string(), repo.to_string());
    }

    fn load_data(&mut self, payload: ApiPayload, selection: &mut SelectionState) {
        if let ApiPayload::Pulls(pulls) = payload {
            self.data.load_items(pulls, selection);
        }
    }

    fn set_loading(&mut self, loading: bool) {
        self.data.loading = loading;
    }

    fn handle_key(&mut self, key: char, selection: &mut SelectionState, detail_pane: &mut DetailPane) -> Option<Cmd> {
        match key {
            '1' => {
                self.state_filter = "open".to_string();
                if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                    Some(Cmd::FetchPulls { owner: owner.clone(), repo: repo.clone(), state: "open".into() })
                } else { None }
            }
            '2' => {
                self.state_filter = "closed".to_string();
                if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                    Some(Cmd::FetchPulls { owner: owner.clone(), repo: repo.clone(), state: "closed".into() })
                } else { None }
            }
            '3' => {
                self.state_filter = "all".to_string();
                if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                    Some(Cmd::FetchPulls { owner: owner.clone(), repo: repo.clone(), state: "all".into() })
                } else { None }
            }
            'o' => {
                if let Some(pr) = self.data.selected(selection) {
                    if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                        let _ = std::process::Command::new("gh")
                            .args(["browse", &format!("{}", pr.number), "--repo", &format!("{}/{}", owner, repo)])
                            .spawn();
                    }
                }
                None
            }
            'a' => {
                if let Some(pr) = self.data.selected(selection) {
                    if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                        return Some(Cmd::ApprovePr { owner: owner.clone(), repo: repo.clone(), pr_number: pr.number as u64 });
                    }
                }
                None
            }
            'm' => {
                if let Some(pr) = self.data.selected(selection) {
                    if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                        return Some(Cmd::MergePr { owner: owner.clone(), repo: repo.clone(), pr_number: pr.number as u64 });
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
