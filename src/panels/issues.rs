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
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::{render_list_view, LeftContent, ListViewConfig};
use super::panel::{Panel, SelectionState, handle_key_default, ListPanelData};

#[derive(Debug)]
pub struct IssuesPanel {
    pub data: ListPanelData<Issue>,
    pub state_filter: String,
}

impl Default for IssuesPanel {
    fn default() -> Self {
        Self {
            data: ListPanelData::new(),
            state_filter: "open".to_string(),
        }
    }
}

impl IssuesPanel {
    #[allow(clippy::too_many_arguments)]
    pub fn view(
        frame: &mut Frame,
        area: Rect,
        state: &IssuesPanel,
        selection: &mut SelectionState,
        detail_pane: &DetailPane,
        focused: bool,
        theme: &Theme,
        tick: u64,
    ) {
        let items: Vec<ListItem> = state
            .data
            .filtered_indices
            .iter()
            .map(|&i| {
                let issue = &state.data.items[i];
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

        let detail_lines = if let Some(issue) = state.data.selected(selection) {
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
            loading: state.data.loading,
            empty_state: EmptyState::new("No issues found")
                .icon(" ")
                .hint("Press `r` to refresh, 1/2/3 to filter"),
            detail_title: " Detail ".to_string(),
            highlight_symbol: "▸ ".to_string(),
            detail_empty_state: EmptyState::new("Select an issue")
                .icon(" ")
                .hint("j/k to navigate"),
        };

        let left = LeftContent::List { items };

        render_list_view(frame, area, config, left, selection, detail_pane, detail_lines, theme, tick);
    }
}

impl Panel for IssuesPanel {
    fn fetch_cmd(&self) -> Cmd {
        match (&self.data.owner, &self.data.repo) {
            (Some(owner), Some(repo)) => Cmd::FetchIssues {
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
        if let ApiPayload::Issues(issues) = payload {
            self.data.load_items(issues, selection);
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
                    Some(Cmd::FetchIssues { owner: owner.clone(), repo: repo.clone(), state: "open".into() })
                } else { None }
            }
            '2' => {
                self.state_filter = "closed".to_string();
                if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                    Some(Cmd::FetchIssues { owner: owner.clone(), repo: repo.clone(), state: "closed".into() })
                } else { None }
            }
            '3' => {
                self.state_filter = "all".to_string();
                if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                    Some(Cmd::FetchIssues { owner: owner.clone(), repo: repo.clone(), state: "all".into() })
                } else { None }
            }
            'o' => {
                if let Some(issue) = self.data.selected(selection) {
                    if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                        let _ = std::process::Command::new("gh")
                            .args(["browse", &format!("{}", issue.number), "--repo", &format!("{}/{}", owner, repo)])
                            .spawn();
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
