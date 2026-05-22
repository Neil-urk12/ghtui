use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::domain::WorkflowRun;
use crate::cmd::Cmd;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::util::format::relative_time;
use crate::widgets::detail_pane::DetailPane;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::{render_list_view, LeftContent, ListViewConfig};
use super::panel::{Panel, SelectionState, handle_key_default, ListPanelData};

#[derive(Debug, Default)]
pub struct ActionsPanel {
    pub data: ListPanelData<WorkflowRun>,
    pub expanded_run: Option<usize>,
}

impl ActionsPanel {
    pub fn toggle_expand(&mut self, selection: &SelectionState) {
        if let Some(idx) = selection.selected() {
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

    #[allow(clippy::too_many_arguments)]
    pub fn view(
        frame: &mut Frame,
        area: Rect,
        state: &ActionsPanel,
        selection: &mut SelectionState,
        detail_pane: &DetailPane,
        focused: bool,
        theme: &Theme,
        tick: u64,
    ) {
        let items: Vec<ListItem> = state.data.items.iter().enumerate().map(|(i, run)| {
            let (icon, style) = Self::status_icon(run);
            let is_expanded = state.expanded_run == Some(i);

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
        }).collect();

        let detail_lines = if let Some(run) = state.data.selected(selection) {
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

            if state.expanded_run == selection.selected() {
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

            content
        } else {
            vec![]
        };

        let config = ListViewConfig {
            split_ratio: (50, 50),
            margin: 1,
            focused,
            block_title: format!(
                " Actions: {}/{} ",
                state.data.owner.as_deref().unwrap_or(""),
                state.data.repo.as_deref().unwrap_or("")
            ),
            wrap_full: true,
            loading: state.data.loading,
            empty_state: EmptyState::new("No workflow runs")
                .icon(" ")
                .hint("Press `r` to refresh"),
            detail_title: " Detail ".to_string(),
            highlight_symbol: "  ".to_string(),
            detail_empty_state: EmptyState::new("Select a run")
                .icon(" ")
                .hint("j/k to navigate, Enter to expand"),
        };

        let left = LeftContent::List { items };

        render_list_view(frame, area, config, left, selection, detail_pane, detail_lines, theme, tick);
    }
}

impl Panel for ActionsPanel {
    fn fetch_cmd(&self) -> Cmd {
        match (&self.data.owner, &self.data.repo) {
            (Some(owner), Some(repo)) => Cmd::FetchActions {
                owner: owner.clone(),
                repo: repo.clone(),
            },
            _ => Cmd::None,
        }
    }

    fn set_repo_context(&mut self, owner: &str, repo: &str) {
        self.data.set_repo(owner.to_string(), repo.to_string());
    }

    fn set_loading(&mut self, loading: bool) {
        self.data.loading = loading;
    }

    fn load_data(&mut self, payload: ApiPayload, selection: &mut SelectionState) {
        if let ApiPayload::Actions(runs) = payload {
            self.data.load_items(runs, selection);
            self.expanded_run = None;
        }
    }

    fn handle_key(&mut self, key: char, selection: &mut SelectionState, detail_pane: &mut DetailPane) -> Option<Cmd> {
        match key {
            '\n' => {
                self.toggle_expand(selection);
                None
            }
            'o' => {
                if let Some(run) = self.data.selected(selection) {
                    if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                        let _ = std::process::Command::new("gh")
                            .args(["browse", &format!("{}", run.id), "--repo", &format!("{}/{}", owner, repo)])
                            .spawn();
                    }
                }
                None
            }
            'r' => {
                if let Some(run) = self.data.selected(selection) {
                    if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                        return Some(Cmd::RerunWorkflow { owner: owner.clone(), repo: repo.clone(), run_id: run.id });
                    }
                }
                None
            }
            'c' => {
                if let Some(run) = self.data.selected(selection) {
                    if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                        return Some(Cmd::CancelWorkflow { owner: owner.clone(), repo: repo.clone(), run_id: run.id });
                    }
                }
                None
            }
            'v' => {
                if let Some(run) = self.data.selected(selection) {
                    if let (Some(owner), Some(repo)) = (&self.data.owner, &self.data.repo) {
                        let _ = std::process::Command::new("gh")
                            .args(["run", "view", &format!("{}", run.id), "--repo", &format!("{}/{}", owner, repo)])
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
