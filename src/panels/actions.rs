use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::domain::WorkflowRun;
use crate::cmd::Cmd;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::util::format::relative_time;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::LeftContent;
use super::panel::{ListPanelData, PanelItem};

#[derive(Debug, Default)]
pub struct ActionsPanelItem {
    pub expanded_run: Option<usize>,
}

impl ActionsPanelItem {
    fn toggle_expand(&mut self, selected_index: Option<usize>) {
        if let Some(idx) = selected_index {
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
}

impl PanelItem for ActionsPanelItem {
    type Item = WorkflowRun;

    fn left_content<'a>(
        &'a self,
        data: &'a ListPanelData<Self::Item>,
        theme: &'a Theme,
    ) -> LeftContent<'a> {
        let items: Vec<ListItem> = data.items.iter().enumerate().map(|(i, run)| {
            let (icon, style) = Self::status_icon(run);
            let is_expanded = self.expanded_run == Some(i);

            let status_text = if run.status == "in_progress" {
                "in_progress"
            } else {
                run.conclusion.as_deref().unwrap_or(&run.status)
            };

            let expand_marker = if is_expanded { "▾ " } else { "▸ " };

            let line = Line::from(vec![
                Span::raw(expand_marker),
                icon,
                Span::styled(run.name.clone(), style),
                Span::raw(format!(" #{}", run.run_number)),
                Span::styled(format!("  {}  ", status_text), theme.muted),
                Span::styled(run.branch.clone(), Style::default().fg(Color::Blue)),
                Span::raw(format!("  {}", relative_time(&run.created_at))),
            ]);
            ListItem::new(line)
        }).collect();

        LeftContent::List { items }
    }

    fn detail_lines(&self, run: &Self::Item, theme: &Theme) -> Vec<Line<'static>> {
        let mut content: Vec<Line> = Vec::new();

        let (icon, status_style) = Self::status_icon(run);
        content.push(Line::from(vec![
            icon,
            Span::styled(run.name.clone(), Style::default().add_modifier(Modifier::BOLD)),
        ]));
        content.push(Line::from(""));

        let status_text = if run.status == "in_progress" {
            "in_progress".to_string()
        } else {
            run.conclusion.as_deref().unwrap_or(&run.status).to_string()
        };

        content.push(Line::from(vec![
            Span::styled("Status: ", theme.muted),
            Span::styled(status_text, status_style),
        ]));
        content.push(Line::from(vec![
            Span::styled("Branch: ", theme.muted),
            Span::styled(run.branch.clone(), Style::default().fg(Color::Blue)),
        ]));
        content.push(Line::from(vec![
            Span::styled("Event: ", theme.muted),
            Span::raw(run.event.clone()),
        ]));
        content.push(Line::from(vec![
            Span::styled("Run: ", theme.muted),
            Span::raw(format!("#{}", run.run_number)),
        ]));
        content.push(Line::from(vec![
            Span::styled("Started: ", theme.muted),
            Span::raw(relative_time(&run.created_at)),
        ]));

        // Show expanded job info if this run is expanded
        // Use pointer comparison to find the index since we have the item ref
        content
    }

    fn handle_key(
        &mut self,
        key: char,
        selected: Option<&Self::Item>,
        data: &ListPanelData<Self::Item>,
    ) -> Option<Cmd> {
        // Compute selected index via pointer comparison into data.items
        let selected_index = selected.and_then(|sel| {
            data.items.iter().position(|r| std::ptr::eq(r, sel))
        });

        match key {
            '\n' => {
                self.toggle_expand(selected_index);
                None
            }
            'o' => {
                if let Some(run) = selected {
                    if let (Some(owner), Some(repo)) = (&data.owner, &data.repo) {
                        let _ = std::process::Command::new("gh")
                            .args(["browse", &format!("{}", run.id), "--repo", &format!("{}/{}", owner, repo)])
                            .spawn();
                    }
                }
                None
            }
            'r' => {
                if let Some(run) = selected {
                    if let (Some(owner), Some(repo)) = (&data.owner, &data.repo) {
                        return Some(Cmd::RerunWorkflow {
                            owner: owner.clone(),
                            repo: repo.clone(),
                            run_id: run.id,
                        });
                    }
                }
                None
            }
            'c' => {
                if let Some(run) = selected {
                    if let (Some(owner), Some(repo)) = (&data.owner, &data.repo) {
                        return Some(Cmd::CancelWorkflow {
                            owner: owner.clone(),
                            repo: repo.clone(),
                            run_id: run.id,
                        });
                    }
                }
                None
            }
            'v' => {
                if let Some(run) = selected {
                    if let (Some(owner), Some(repo)) = (&data.owner, &data.repo) {
                        let _ = std::process::Command::new("gh")
                            .args(["run", "view", &format!("{}", run.id), "--repo", &format!("{}/{}", owner, repo)])
                            .spawn();
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn fetch_cmd(&self, data: &ListPanelData<Self::Item>) -> Cmd {
        match (&data.owner, &data.repo) {
            (Some(owner), Some(repo)) => Cmd::FetchActions {
                owner: owner.clone(),
                repo: repo.clone(),
            },
            _ => Cmd::None,
        }
    }

    fn block_title(&self, data: &ListPanelData<Self::Item>) -> String {
        format!(
            " Actions: {}/{} ",
            data.owner.as_deref().unwrap_or(""),
            data.repo.as_deref().unwrap_or("")
        )
    }

    fn empty_state(&self, _data: &ListPanelData<Self::Item>) -> EmptyState {
        EmptyState::new("No workflow runs")
            .icon(" ")
            .hint("Press `r` to refresh")
    }

    fn extract_items(payload: ApiPayload) -> Option<Vec<Self::Item>> {
        if let ApiPayload::Actions(runs) = payload {
            Some(runs)
        } else {
            None
        }
    }

    fn on_loaded(&mut self, _data: &mut ListPanelData<Self::Item>) {
        self.expanded_run = None;
    }

    fn highlight_symbol(&self) -> String {
        "  ".to_string()
    }

    fn detail_title(&self) -> String {
        " Detail ".to_string()
    }

    fn detail_empty_state(&self) -> EmptyState {
        EmptyState::new("Select a run")
            .icon(" ")
            .hint("j/k to navigate, Enter to expand")
    }
}
