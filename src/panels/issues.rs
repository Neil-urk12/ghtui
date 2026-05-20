use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::Issue;
use crate::theme::Theme;
use crate::util::format;
use crate::util::format::relative_time;
use crate::util::markdown::render_markdown;
use crate::widgets::detail_pane::DetailPane;
use crate::widgets::empty_state::EmptyStateWidget;
use crate::widgets::loading::LoadingWidget;

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
    pub fn set_issues(&mut self, issues: Vec<Issue>) {
        self.issues = issues;
        self.filtered_indices = (0..self.issues.len()).collect();
        self.list_state.select(if self.filtered_indices.is_empty() { None } else { Some(0) });
        self.detail_pane.reset_scroll();
    }

    pub fn navigate_up(&mut self) {
        let i = self.list_state.selected().unwrap_or(0);
        if i > 0 {
            self.list_state.select(Some(i - 1));
            self.detail_pane.reset_scroll();
        }
    }

    pub fn navigate_down(&mut self) {
        let i = self.list_state.selected().unwrap_or(0);
        if i + 1 < self.filtered_indices.len() {
            self.list_state.select(Some(i + 1));
            self.detail_pane.reset_scroll();
        }
    }

    pub fn selected_item(&self) -> Option<&Issue> {
        let idx = self.list_state.selected()?;
        self.filtered_indices.get(idx).map(|&i| &self.issues[i])
    }

    pub fn view(frame: &mut Frame, area: Rect, state: &IssuesPanel, focused: bool, theme: &Theme, tick: u64) {
        let border_style = if focused {
            theme.active_border
        } else {
            theme.inactive_border
        };

        let block = Block::bordered()
            .title(format!(" Issues [{}] ", state.state_filter))
            .border_style(border_style);

        if state.loading {
            LoadingWidget::new().render(frame, area, tick, block);
            return;
        }

        if state.issues.is_empty() {
            EmptyStateWidget::render(frame, area, "No issues found", block);
            return;
        }

        let chunks = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
            .margin(1)
            .split(area);

        // Left: issue list
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

        let list = List::new(items)
            .highlight_style(theme.highlight)
            .highlight_symbol("▸ ");

        frame.render_stateful_widget(list, chunks[0], &mut state.list_state.clone());

        // Right: detail pane
        let detail_block = Block::bordered()
            .title(" Detail ")
            .border_style(if focused { theme.active_border } else { theme.inactive_border });

        if let Some(issue) = state.selected_item() {
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

            state
                .detail_pane
                .render(frame, chunks[1], "Detail", content, detail_block);
        } else {
            EmptyStateWidget::render(frame, chunks[1], "Select an issue to view details", detail_block);
        }
    }
}
