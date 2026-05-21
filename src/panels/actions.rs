use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::domain::WorkflowRun;
use crate::cmd::Cmd;
use crate::theme::Theme;
use crate::util::format::relative_time;
use crate::widgets::detail_pane::DetailPane;
use crate::widgets::empty_state::EmptyStateWidget;
use crate::widgets::loading::LoadingWidget;
use super::panel::{Panel, handle_key_default};

#[derive(Debug)]
pub struct ActionsPanel {
    pub runs: Vec<WorkflowRun>,
    pub list_state: ListState,
    pub loading: bool,
    pub detail_pane: DetailPane,
    pub expanded_run: Option<usize>,
    pub owner: Option<String>,
    pub repo: Option<String>,
}

impl Default for ActionsPanel {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            runs: Vec::new(),
            list_state,
            loading: false,
            detail_pane: DetailPane::new(),
            expanded_run: None,
            owner: None,
            repo: None,
        }
    }
}

impl ActionsPanel {
    pub fn selected_item(&self) -> Option<&WorkflowRun> {
        self.list_state.selected().and_then(|i| self.runs.get(i))
    }

    pub fn toggle_expand(&mut self) {
        let selected = self.list_state.selected();
        if let Some(idx) = selected {
            if self.expanded_run == Some(idx) {
                self.expanded_run = None;
            } else {
                self.expanded_run = Some(idx);
            }
        }
    }

    fn status_icon(run: &WorkflowRun) -> (Span<'static>, Style) {
        if run.status == "in_progress" {
            return (
                Span::styled("● ", Color::Yellow),
                Style::default().fg(Color::Yellow),
            );
        }
        if run.status == "queued" {
            return (
                Span::styled("○ ", Color::DarkGray),
                Style::default().fg(Color::DarkGray),
            );
        }
        match run.conclusion.as_deref() {
            Some("success") => (
                Span::styled("✓ ", Color::Green),
                Style::default().fg(Color::Green),
            ),
            Some("failure") => (
                Span::styled("✗ ", Color::Red),
                Style::default().fg(Color::Red),
            ),
            Some("cancelled") => (
                Span::styled("– ", Color::DarkGray),
                Style::default().fg(Color::DarkGray),
            ),
            _ => (
                Span::styled("○ ", Color::DarkGray),
                Style::default().fg(Color::DarkGray),
            ),
        }
    }

    pub fn view(
        frame: &mut Frame,
        area: Rect,
        state: &ActionsPanel,
        focused: bool,
        theme: &Theme,
        tick: u64,
    ) {
        let border_style = if focused {
            theme.active_border
        } else {
            theme.inactive_border
        };

        let block = Block::bordered()
            .title(" Actions ")
            .border_style(border_style);

        if state.loading {
            LoadingWidget::new().render(frame, area, tick, block);
            return;
        }

        if state.runs.is_empty() {
            EmptyStateWidget::render(frame, area, "No workflow runs found", block);
            return;
        }

        let chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .margin(1)
            .split(area);

        // Left: run list
        let items: Vec<ListItem> = state
            .runs
            .iter()
            .map(|run| {
                let (icon, style) = Self::status_icon(run);
                let is_expanded = state.expanded_run == state.list_state.selected();

                let status_text = if run.status == "in_progress" {
                    "in_progress"
                } else {
                    run.conclusion.as_deref().unwrap_or(&run.status)
                };

                let expand_marker = if is_expanded { "▾ " } else { "▸ " };

                let line = Line::from(vec![
                    Span::raw(expand_marker),
                    icon,
                    Span::styled(&run.name, style),
                    Span::raw(format!(" #{}", run.run_number)),
                    Span::styled(format!("  {}  ", status_text), theme.muted),
                    Span::styled(&run.branch, Style::default().fg(Color::Blue)),
                    Span::raw(format!("  {}", relative_time(&run.created_at))),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme.highlight)
            .highlight_symbol("  ");

        frame.render_stateful_widget(list, chunks[0], &mut state.list_state.clone());

        // Right: detail pane
        let detail_block = Block::bordered()
            .title(" Detail ")
            .border_style(if focused {
                theme.active_border
            } else {
                theme.inactive_border
            });

        if let Some(run) = state.selected_item() {
            let mut content: Vec<Line> = Vec::new();

            let (icon, status_style) = Self::status_icon(run);
            content.push(Line::from(vec![
                icon,
                Span::styled(&run.name, Style::default().add_modifier(Modifier::BOLD)),
            ]));
            content.push(Line::from(""));

            let status_text = if run.status == "in_progress" {
                "in_progress".to_string()
            } else {
                run.conclusion
                    .clone()
                    .unwrap_or_else(|| run.status.clone())
            };

            content.push(Line::from(vec![
                Span::styled("Status: ", theme.muted),
                Span::styled(status_text, status_style),
            ]));
            content.push(Line::from(vec![
                Span::styled("Branch: ", theme.muted),
                Span::styled(&run.branch, Style::default().fg(Color::Blue)),
            ]));
            content.push(Line::from(vec![
                Span::styled("Event: ", theme.muted),
                Span::raw(&run.event),
            ]));
            content.push(Line::from(vec![
                Span::styled("Run: ", theme.muted),
                Span::raw(format!("#{}", run.run_number)),
            ]));
            content.push(Line::from(vec![
                Span::styled("Started: ", theme.muted),
                Span::raw(relative_time(&run.created_at)),
            ]));

            if state.expanded_run == state.list_state.selected() {
                content.push(Line::from(""));
                content.push(Line::from(Span::styled(
                    "Jobs",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )));
                content.push(Line::from(Span::styled(
                    "  (jobs require API fetch - press 'v' for logs)",
                    theme.muted,
                )));
            }

            state
                .detail_pane
                .render(frame, chunks[1], "Detail", content, detail_block);
        } else {
            EmptyStateWidget::render(
                frame,
                chunks[1],
                "Select a run to view details",
                detail_block,
            );
        }
    }
}

impl Panel for ActionsPanel {
    fn detail_pane(&mut self) -> &mut DetailPane {
        &mut self.detail_pane
    }

    fn list_len(&self) -> usize {
        self.runs.len()
    }

    fn selected_index(&self) -> Option<usize> {
        self.list_state.selected()
    }

    fn set_selected(&mut self, index: Option<usize>) {
        self.list_state.select(index);
    }

    fn handle_key(&mut self, key: char) -> Option<Cmd> {
        match key {
            '\n' => { self.toggle_expand(); None }
            'r' => {
                if let Some(run) = self.selected_item() {
                    if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                        return Some(Cmd::RerunWorkflow { owner: owner.clone(), repo: repo.clone(), run_id: run.id });
                    }
                }
                None
            }
            'c' => {
                if let Some(run) = self.selected_item() {
                    if let (Some(owner), Some(repo)) = (&self.owner, &self.repo) {
                        return Some(Cmd::CancelWorkflow { owner: owner.clone(), repo: repo.clone(), run_id: run.id });
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
