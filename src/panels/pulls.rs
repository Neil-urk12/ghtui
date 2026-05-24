use ratatui::prelude::*;
use ratatui::widgets::{Cell, Row};

use crate::cmd::Cmd;
use crate::domain::PullRequest;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::util::format;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::LeftContent;
use super::panel::{ListPanelData, PanelItem, SelectionState};

fn pr_to_row(pr: &PullRequest, theme: &Theme) -> Row<'static> {
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
}

fn pr_to_detail(pr: &PullRequest, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            format!("#{} {}", pr.number, pr.title),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "by {} | opened {} | {}",
            pr.author,
            format::relative_time(&pr.created_at),
            format::relative_time(&pr.updated_at)
        )),
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
}

#[derive(Debug, Default)]
pub struct PullsPanelItem {
    pub state_filter: String, // "open", "closed", "all"
}

impl PanelItem for PullsPanelItem {
    type Item = PullRequest;

    fn left_content<'a>(
        &'a self,
        data: &'a ListPanelData<Self::Item>,
        theme: &'a Theme,
    ) -> LeftContent<'a> {
        let rows: Vec<Row> = data
            .filtered_indices
            .iter()
            .map(|&i| pr_to_row(&data.items[i], theme))
            .collect();
        LeftContent::Table {
            headers: vec!["#", "Title", "Author", "Status", "Updated"],
            rows,
            widths: vec![
                Constraint::Length(6),
                Constraint::Fill(3),
                Constraint::Fill(1),
                Constraint::Length(8),
                Constraint::Fill(1),
            ],
        }
    }

    fn detail_lines(&self, item: &Self::Item, theme: &Theme) -> Vec<Line<'static>> {
        pr_to_detail(item, theme)
    }

    fn handle_key(
        &mut self,
        key: char,
        selected: Option<&Self::Item>,
        data: &ListPanelData<Self::Item>,
    ) -> Option<Cmd> {
        match key {
            '1' => {
                self.state_filter = "open".to_string();
                if let (Some(owner), Some(repo)) = (&data.owner, &data.repo) {
                    Some(Cmd::FetchPulls {
                        owner: owner.clone(),
                        repo: repo.clone(),
                        state: "open".into(),
                    })
                } else {
                    None
                }
            }
            '2' => {
                self.state_filter = "closed".to_string();
                if let (Some(owner), Some(repo)) = (&data.owner, &data.repo) {
                    Some(Cmd::FetchPulls {
                        owner: owner.clone(),
                        repo: repo.clone(),
                        state: "closed".into(),
                    })
                } else {
                    None
                }
            }
            '3' => {
                self.state_filter = "all".to_string();
                if let (Some(owner), Some(repo)) = (&data.owner, &data.repo) {
                    Some(Cmd::FetchPulls {
                        owner: owner.clone(),
                        repo: repo.clone(),
                        state: "all".into(),
                    })
                } else {
                    None
                }
            }
            'o' => {
                if let Some(pr) = selected {
                    if let (Some(owner), Some(repo)) = (&data.owner, &data.repo) {
                        let _ = std::process::Command::new("gh")
                            .args(["browse", &format!("{}", pr.number), "--repo", &format!("{}/{}", owner, repo)])
                            .spawn();
                    }
                }
                None
            }
            'a' => {
                if let Some(pr) = selected {
                    if let (Some(owner), Some(repo)) = (&data.owner, &data.repo) {
                        return Some(Cmd::ApprovePr {
                            owner: owner.clone(),
                            repo: repo.clone(),
                            pr_number: pr.number as u64,
                        });
                    }
                }
                None
            }
            'm' => {
                if let Some(pr) = selected {
                    if let (Some(owner), Some(repo)) = (&data.owner, &data.repo) {
                        return Some(Cmd::MergePr {
                            owner: owner.clone(),
                            repo: repo.clone(),
                            pr_number: pr.number as u64,
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn fetch_cmd(&self, data: &ListPanelData<Self::Item>) -> Cmd {
        match (&data.owner, &data.repo) {
            (Some(owner), Some(repo)) => Cmd::FetchPulls {
                owner: owner.clone(),
                repo: repo.clone(),
                state: self.state_filter.clone(),
            },
            _ => Cmd::None,
        }
    }

    fn block_title(&self, data: &ListPanelData<Self::Item>) -> String {
        match (&data.owner, &data.repo) {
            (Some(o), Some(r)) => format!(" PRs: {o}/{r} ({}) ", self.state_filter),
            _ => format!(" Pull Requests ({}) ", self.state_filter),
        }
    }

    fn empty_state(&self, data: &ListPanelData<Self::Item>) -> EmptyState {
        if data.owner.is_some() {
            EmptyState::new("No pull requests found")
                .icon(" ")
                .hint("Press `r` to refresh, 1/2/3 to filter")
        } else {
            EmptyState::new("Select a repository first")
                .icon(" ")
                .hint("Switch to Repos tab, press Enter on a repo")
        }
    }

    fn extract_items(payload: ApiPayload) -> Option<Vec<Self::Item>> {
        if let ApiPayload::Pulls(pulls) = payload {
            Some(pulls)
        } else {
            None
        }
    }

    fn split_ratio(&self) -> (u16, u16) {
        (60, 40)
    }

    fn detail_title(&self) -> String {
        " PR Details ".to_string()
    }

    fn detail_empty_state(&self) -> EmptyState {
        EmptyState::new("Select a pull request")
            .icon(" ")
            .hint("j/k to navigate")
    }

    fn initial_selection(&self) -> SelectionState {
        SelectionState::table()
    }
}
