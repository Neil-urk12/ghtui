use ratatui::prelude::*;
use ratatui::widgets::*;

use super::panel::{ListPanelData, PanelItem, SelectionState};
use crate::cmd::{Cmd, RepoContext};
use crate::domain::WorkflowRun;
use crate::theme::Theme;
use crate::util::format::relative_time;
use crate::widgets::detail_pane::DetailPane;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::LeftContent;

#[derive(Debug, Default)]
pub struct ActionsPanelItem;

impl ActionsPanelItem {
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
        let items: Vec<ListItem> = data
            .filtered_indices
            .iter()
            .map(|&i| {
                let run = &data.items[i];
                let (icon, style) = Self::status_icon(run);

                let status_text = if run.status == "in_progress" {
                    "in_progress"
                } else {
                    run.conclusion.as_deref().unwrap_or(&run.status)
                };

                let line = Line::from(vec![
                    icon,
                    Span::styled(run.name.clone(), style),
                    Span::raw(format!(" #{}", run.run_number)),
                    Span::styled(format!("  {}  ", status_text), theme.muted),
                    Span::styled(run.branch.clone(), Style::default().fg(Color::Blue)),
                    Span::raw(format!("  {}", relative_time(&run.created_at))),
                ]);
                ListItem::new(line)
            })
            .collect();

        LeftContent::List { items }
    }

    fn detail_lines(&self, run: &Self::Item, theme: &Theme) -> Vec<Line<'static>> {
        let mut content: Vec<Line> = Vec::new();

        let (icon, status_style) = Self::status_icon(run);
        content.push(Line::from(vec![
            icon,
            Span::styled(
                run.name.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
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

        content
    }

    fn handle_key(
        &mut self,
        key: char,
        selected: Option<&Self::Item>,
        data: &ListPanelData<Self::Item>,
        _detail_pane: &mut DetailPane,
    ) -> Option<Cmd> {
        match key {
            'R' => {
                if let Some(run) = selected {
                    if let Some((owner, repo)) = data.owner_repo() {
                        return Some(Cmd::RerunWorkflow {
                            ctx: RepoContext::new(owner.to_string(), repo.to_string()),
                            run_id: run.id,
                        });
                    }
                }
                None
            }
            'c' => {
                if let Some(run) = selected {
                    if let Some((owner, repo)) = data.owner_repo() {
                        return Some(Cmd::CancelWorkflow {
                            ctx: RepoContext::new(owner.to_string(), repo.to_string()),
                            run_id: run.id,
                        });
                    }
                }
                None
            }
            'v' | 'o' => {
                if let Some(run) = selected {
                    if let Some((owner, repo)) = data.owner_repo() {
                        crate::util::spawn_background(
                            "gh",
                            &[
                                "run",
                                "view",
                                &format!("{}", run.id),
                                "--repo",
                                &format!("{}/{}", owner, repo),
                                "--web",
                            ],
                        );
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn fetch_cmd(&self, data: &ListPanelData<Self::Item>) -> Cmd {
        if let Some((owner, repo)) = data.owner_repo() {
            Cmd::FetchActions {
                ctx: RepoContext::new(owner.to_string(), repo.to_string()),
            }
        } else {
            Cmd::None
        }
    }

    fn block_title(&self, data: &ListPanelData<Self::Item>) -> String {
        if let Some((o, r)) = data.owner_repo() {
            format!(" Actions: {o}/{r} ")
        } else {
            " Actions ".into()
        }
    }

    fn empty_state(&self, data: &ListPanelData<Self::Item>) -> EmptyState {
        data.repo_aware_empty_state("No workflow runs", " ", "Press `r` to refresh")
    }

    extract_items!(Actions);

    fn highlight_symbol(&self) -> String {
        "  ".to_string()
    }

    fn detail_title(&self, _showing_diff: bool) -> String {
        " Detail ".to_string()
    }

    fn detail_empty_state(&self) -> EmptyState {
        EmptyState::new("Select a run")
            .icon(" ")
            .hint("j/k to navigate, v to view logs")
    }

    fn apply_filter(
        &self,
        data: &mut ListPanelData<Self::Item>,
        query: &str,
        selection: &mut SelectionState,
    ) {
        data.filter_by(
            query,
            |item, q| {
                item.name.to_lowercase().contains(q)
                    || item.branch.to_lowercase().contains(q)
                    || item.event.to_lowercase().contains(q)
                    || item.status.to_lowercase().contains(q)
                    || item
                        .conclusion
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(q)
            },
            selection,
        );
    }

    fn item_display(&self, item: &Self::Item) -> Option<String> {
        Some(format!("#{} {}", item.run_number, item.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::ApiPayload;
    use crate::panels::panel::SelectionState;

    fn test_workflow_run() -> WorkflowRun {
        WorkflowRun {
            id: 1234567890,
            name: "CI".into(),
            status: "completed".into(),
            conclusion: Some("success".into()),
            branch: "main".into(),
            event: "push".into(),
            created_at: "2024-01-15T10:30:00Z".into(),
            run_number: 42,
        }
    }

    fn make_data() -> ListPanelData<WorkflowRun> {
        let mut data = ListPanelData::<WorkflowRun>::new();
        data.items = vec![test_workflow_run()];
        data.filtered_indices = vec![0];
        data.owner = Some("octocat".into());
        data.repo = Some("hello-world".into());
        data
    }

    fn make_data_no_repo() -> ListPanelData<WorkflowRun> {
        let mut data = ListPanelData::<WorkflowRun>::new();
        data.items = vec![test_workflow_run()];
        data.filtered_indices = vec![0];
        data
    }

    // --- left_content ---

    #[test]
    fn left_content_builds_correct_list_items() {
        let data = make_data();
        let item = ActionsPanelItem;
        let theme = Theme::default();

        let content = item.left_content(&data, &theme);

        match content {
            LeftContent::List { items } => {
                assert_eq!(items.len(), 1);
                // First list item should contain status icon + name + branch
                let debug = format!("{:?}", &items[0]);
                assert!(debug.contains("CI"));
                assert!(debug.contains("#42"));
                assert!(debug.contains("main"));
            }
            _ => panic!("expected List variant"),
        }
    }

    // --- detail_lines ---

    #[test]
    fn detail_lines_includes_run_details() {
        let run = test_workflow_run();
        let item = ActionsPanelItem;
        let theme = Theme::default();

        let lines = item.detail_lines(&run, &theme);

        let text: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("CI"), "should contain run name");
        assert!(text.contains("Status:"), "should contain status label");
        assert!(text.contains("success"), "should contain conclusion");
        assert!(text.contains("Branch:"), "should contain branch label");
        assert!(text.contains("main"), "should contain branch");
        assert!(text.contains("Event:"), "should contain event label");
        assert!(text.contains("push"), "should contain event");
        assert!(text.contains("Run:"), "should contain run label");
        assert!(text.contains("#42"), "should contain run number");
        assert!(text.contains("Started:"), "should contain started label");
    }

    // --- handle_key ---

    #[test]
    fn handle_key_r_upper_returns_rerun_workflow() {
        let data = make_data();
        let mut item = ActionsPanelItem;

        let selected = data.items.first();
        let cmd = item.handle_key('R', selected, &data, &mut DetailPane::new());

        assert!(cmd.is_some());
        assert!(matches!(
            cmd.unwrap(),
            Cmd::RerunWorkflow { ctx, run_id }
                if ctx.owner == "octocat" && ctx.repo == "hello-world" && run_id == 1234567890
        ));
    }

    #[test]
    fn handle_key_c_returns_cancel_workflow() {
        let data = make_data();
        let mut item = ActionsPanelItem;

        let selected = data.items.first();
        let cmd = item.handle_key('c', selected, &data, &mut DetailPane::new());

        assert!(cmd.is_some());
        assert!(matches!(
            cmd.unwrap(),
            Cmd::CancelWorkflow { ctx, run_id }
                if ctx.owner == "octocat" && ctx.repo == "hello-world" && run_id == 1234567890
        ));
    }

    #[test]
    fn handle_key_r_no_owner_returns_none() {
        let data = make_data_no_repo();
        let mut item = ActionsPanelItem;

        let selected = data.items.first();
        let cmd = item.handle_key('R', selected, &data, &mut DetailPane::new());

        assert!(cmd.is_none());
    }

    #[test]
    fn handle_key_c_no_owner_returns_none() {
        let data = make_data_no_repo();
        let mut item = ActionsPanelItem;

        let selected = data.items.first();
        let cmd = item.handle_key('c', selected, &data, &mut DetailPane::new());

        assert!(cmd.is_none());
    }

    #[test]
    fn handle_key_no_selection_returns_none() {
        let data = make_data();
        let mut item = ActionsPanelItem;

        let cmd = item.handle_key('R', None, &data, &mut DetailPane::new());
        assert!(cmd.is_none());

        let cmd = item.handle_key('c', None, &data, &mut DetailPane::new());
        assert!(cmd.is_none());
    }

    // --- extract_items ---

    #[test]
    fn extract_items_actions_returns_some() {
        let run = test_workflow_run();
        let payload = ApiPayload::Actions(vec![run]);

        let result = ActionsPanelItem::extract_items(payload);

        assert!(result.is_some());
        let runs = result.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, 1234567890);
    }

    #[test]
    fn extract_items_repos_returns_none() {
        let payload = ApiPayload::Repos(vec![]);

        let result = ActionsPanelItem::extract_items(payload);

        assert!(result.is_none());
    }

    #[test]
    fn extract_items_empty_actions_returns_some_empty() {
        let payload = ApiPayload::Actions(vec![]);

        let result = ActionsPanelItem::extract_items(payload);

        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn apply_filter_by_name() {
        let mut data = ListPanelData::<WorkflowRun>::new();
        let mut selection = SelectionState::list();
        let items = vec![
            WorkflowRun {
                id: 1,
                name: "CI".into(),
                ..Default::default()
            },
            WorkflowRun {
                id: 2,
                name: "Deploy".into(),
                ..Default::default()
            },
            WorkflowRun {
                id: 3,
                name: "CI nightly".into(),
                ..Default::default()
            },
        ];
        data.set_items(items, &mut selection);
        let panel = ActionsPanelItem;
        panel.apply_filter(&mut data, "CI", &mut selection);
        assert_eq!(data.filtered_indices, vec![0, 2]);
    }

    #[test]
    fn apply_filter_by_branch() {
        let mut data = ListPanelData::<WorkflowRun>::new();
        let mut selection = SelectionState::list();
        let items = vec![
            WorkflowRun {
                id: 1,
                name: "CI".into(),
                branch: "main".into(),
                ..Default::default()
            },
            WorkflowRun {
                id: 2,
                name: "CI".into(),
                branch: "feature-x".into(),
                ..Default::default()
            },
            WorkflowRun {
                id: 3,
                name: "CI".into(),
                branch: "main-hotfix".into(),
                ..Default::default()
            },
        ];
        data.set_items(items, &mut selection);
        let panel = ActionsPanelItem;
        panel.apply_filter(&mut data, "main", &mut selection);
        assert_eq!(data.filtered_indices, vec![0, 2]);
    }
    #[test]
    fn left_content_empty_items() {
        let data = ListPanelData::<WorkflowRun>::new();
        let item: ActionsPanelItem = Default::default();
        let theme = Theme::default();
        let content = item.left_content(&data, &theme);
        match content {
            LeftContent::Table { rows, .. } => assert!(rows.is_empty()),
            LeftContent::List { items, .. } => assert!(items.is_empty()),
        }
    }
}
