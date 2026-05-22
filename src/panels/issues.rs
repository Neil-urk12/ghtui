use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::domain::Issue;
use crate::cmd::Cmd;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::util::format;
use crate::util::format::relative_time;
use crate::util::markdown::render_markdown;
use crate::widgets::detail_pane::DetailPane;
use crate::widgets::list_view::{render_list_view, LeftContent, ListViewConfig};
use super::panel::{Panel, handle_key_default};

#[derive(Debug)]
pub struct IssuesPanel {
    pub issues: Vec<Issue>,
    pub filtered_indices: Vec<usize>,
    pub list_state: ListState,
    pub loading: bool,
    pub detail_pane: DetailPane,
    pub state_filter: String,
    pub owner: Option<String>,
    pub repo: Option<String>,
}

impl Default for IssuesPanel {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            issues: Vec::new(),
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

impl IssuesPanel {
    pub fn set_repo(&mut self, owner: String, repo: String) {
        self.owner = Some(owner);
        self.repo = Some(repo);
    }

    pub fn set_issues(&mut self, issues: Vec<Issue>) {
        self.issues = issues;
        self.filtered_indices = (0..self.issues.len()).collect();
        self.list_state.select(if self.filtered_indices.is_empty() { None } else { Some(0) });
        self.detail_pane.reset_scroll();
    }

    pub fn selected_item(&self) -> Option<&Issue> {
        let idx = self.list_state.selected()?;
        self.filtered_indices.get(idx).map(|&i| &self.issues[i])
    }

    pub fn view(frame: &mut Frame, area: Rect, state: &IssuesPanel, focused: bool, theme: &Theme, tick: u64) {
        let items: Vec<ListItem> = state
            .filtered_indices
            .iter()
            .map(|&i| {
                let issue = &state.issues[i];
                let state_icon = if issue.state == "open" {
                    Span::styled("● ", theme.success)
                } else {
                    Span::styled("● ", theme.error)
                };

                let labels: String = if issue.labels.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", issue.labels.join(", "))
                };

                let title_style = if issue.state == "open" {
                    Style::default()
                } else {
                    theme.muted
                };

                let line = Line::from(vec![
                    state_icon,
                    Span::styled(format!("#{}", issue.number), theme.muted),
                    Span::raw(" "),
                    Span::styled(&issue.title, title_style),
                    Span::styled(labels, theme.muted),
                    Span::raw(format!("  {}", relative_time(&issue.updated_at))),
                ]);
                ListItem::new(line)
            })
            .collect();

        let detail_lines: Vec<Line> = if let Some(issue) = state.selected_item() {
            let mut content: Vec<Line> = Vec::new();
            content.push(Line::from(vec![
                Span::styled(format!("#{} ", issue.number), theme.muted),
                Span::styled(&issue.title, Style::default().add_modifier(Modifier::BOLD)),
            ]));
            content.push(Line::from(vec![
                Span::styled("Author: ", theme.muted),
                Span::raw(&issue.author),
            ]));
            content.push(Line::from(vec![
                Span::styled("Opened: ", theme.muted),
                Span::raw(format::relative_time(&issue.created_at)),
            ]));
            content.push(Line::from(vec![
                Span::styled("State: ", theme.muted),
                if issue.state == "open" {
                    Span::styled("open", theme.success)
                } else {
                    Span::styled("closed", theme.error)
                },
            ]));
            if !issue.labels.is_empty() {
                content.push(Line::from(vec![
                    Span::styled("Labels: ", theme.muted),
                    Span::raw(issue.labels.join(", ")),
                ]));
            }
            if !issue.assignees.is_empty() {
                content.push(Line::from(vec![
                    Span::styled("Assignees: ", theme.muted),
                    Span::raw(issue.assignees.join(", ")),
                ]));
            }
            content.push(Line::from(""));

            if let Some(ref body) = issue.body {
                content.extend(render_markdown(body));
            } else {
                content.push(Line::from(Span::styled("No description", theme.muted)));
            }

            content
        } else {
            vec![]
        };

        let config = ListViewConfig {
            split_ratio: (45, 55),
            margin: 1,
            focused,
            block_title: format!(" Issues [{}] ", state.state_filter),
            wrap_full: true,
            loading: state.loading,
            empty_message: "No issues found".to_string(),
            detail_title: " Detail ".to_string(),
            highlight_symbol: "▸ ".to_string(),
            detail_empty_message: "Select an issue to view details".to_string(),
        };

        let left = LeftContent::List {
            items,
            state: &mut state.list_state.clone(),
        };

        render_list_view(frame, area, config, left, &mut state.detail_pane.clone(), detail_lines, theme, tick);
    }
}

impl Panel for IssuesPanel {
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
            (Some(owner), Some(repo)) => Cmd::FetchIssues {
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
        if let ApiPayload::Issues(issues) = payload {
            self.set_issues(issues);
        }
    }

    fn handle_key(&mut self, key: char) -> Option<Cmd> {
        match key {
            '1' => {
                self.state_filter = "open".to_string();
                if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                    Some(Cmd::FetchIssues { owner: owner.clone(), repo: repo.clone(), state: "open".into() })
                } else { None }
            }
            '2' => {
                self.state_filter = "closed".to_string();
                if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                    Some(Cmd::FetchIssues { owner: owner.clone(), repo: repo.clone(), state: "closed".into() })
                } else { None }
            }
            '3' => {
                self.state_filter = "all".to_string();
                if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                    Some(Cmd::FetchIssues { owner: owner.clone(), repo: repo.clone(), state: "all".into() })
                } else { None }
            }
            'o' => {
                if let Some(issue) = self.selected_item() {
                    if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                        let _ = std::process::Command::new("gh")
                            .args(["browse", &format!("{}", issue.number), "--repo", &format!("{}/{}", owner, repo)])
                            .spawn();
                    }
                }
                None
            }
            'c' => {
                if let Some(issue) = self.selected_item() {
                    if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                        let _ = std::process::Command::new("gh")
                            .args(["browse", &format!("{}", issue.number), "--repo", &format!("{}/{}", owner, repo)])
                            .spawn();
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
