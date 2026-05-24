use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::cmd::Cmd;
use crate::domain::Issue;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::util::format;
use crate::util::format::relative_time;
use crate::util::markdown::render_markdown;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::LeftContent;
use super::panel::{ListPanelData, PanelItem};

fn issue_to_list_item(issue: &Issue, theme: &Theme) -> ListItem<'static> {
    let state_icon = if issue.state == "open" {
        Span::styled("\u{25cf} ", theme.success)
    } else {
        Span::styled("\u{25cf} ", theme.error)
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
        Span::styled(issue.title.clone(), title_style),
        Span::styled(labels, theme.muted),
        Span::raw(format!("  {}", relative_time(&issue.updated_at))),
    ]);
    ListItem::new(line)
}

fn issue_to_detail(issue: &Issue, theme: &Theme) -> Vec<Line<'static>> {
    let mut content: Vec<Line<'static>> = Vec::new();
    content.push(Line::from(vec![
        Span::styled(format!("#{} ", issue.number), theme.muted),
        Span::styled(issue.title.clone(), Style::default().add_modifier(Modifier::BOLD)),
    ]));
    content.push(Line::from(vec![
        Span::styled("Author: ", theme.muted),
        Span::raw(issue.author.clone()),
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
}

#[derive(Debug, Default)]
pub struct IssuesPanelItem {
    pub state_filter: String,
}

impl PanelItem for IssuesPanelItem {
    type Item = Issue;

    fn left_content<'a>(
        &'a self,
        data: &'a ListPanelData<Self::Item>,
        theme: &'a Theme,
    ) -> LeftContent<'a> {
        let items: Vec<ListItem> = data
            .filtered_indices
            .iter()
            .map(|&i| issue_to_list_item(&data.items[i], theme))
            .collect();
        LeftContent::List { items }
    }

    fn detail_lines(&self, item: &Self::Item, theme: &Theme) -> Vec<Line<'static>> {
        issue_to_detail(item, theme)
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
                    Some(Cmd::FetchIssues {
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
                    Some(Cmd::FetchIssues {
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
                    Some(Cmd::FetchIssues {
                        owner: owner.clone(),
                        repo: repo.clone(),
                        state: "all".into(),
                    })
                } else {
                    None
                }
            }
            'o' => {
                if let Some(issue) = selected {
                    if let (Some(owner), Some(repo)) = (&data.owner, &data.repo) {
                        let _ = std::process::Command::new("gh")
                            .args(["browse", &format!("{}", issue.number), "--repo", &format!("{}/{}", owner, repo)])
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
            (Some(owner), Some(repo)) => Cmd::FetchIssues {
                owner: owner.clone(),
                repo: repo.clone(),
                state: self.state_filter.clone(),
            },
            _ => Cmd::None,
        }
    }

    fn block_title(&self, data: &ListPanelData<Self::Item>) -> String {
        match (&data.owner, &data.repo) {
            (Some(o), Some(r)) => format!(" Issues: {o}/{r} ({}) ", self.state_filter),
            _ => format!(" Issues ({}) ", self.state_filter),
        }
    }

    fn empty_state(&self, data: &ListPanelData<Self::Item>) -> EmptyState {
        if data.owner.is_some() {
            EmptyState::new("No issues found")
                .icon("\u{1f41b}")
                .hint("Press `r` to refresh, 1/2/3 to filter")
        } else {
            EmptyState::new("Select a repository first")
                .icon("\u{1f4e6}")
                .hint("Switch to Repos tab, press Enter on a repo")
        }
    }

    fn extract_items(payload: ApiPayload) -> Option<Vec<Self::Item>> {
        if let ApiPayload::Issues(issues) = payload {
            Some(issues)
        } else {
            None
        }
    }

    fn split_ratio(&self) -> (u16, u16) {
        (60, 40)
    }

    fn detail_title(&self) -> String {
        " Issue Details ".to_string()
    }

    fn detail_empty_state(&self) -> EmptyState {
        EmptyState::new("Select an issue")
            .icon("\u{1f4dd}")
            .hint("j/k to navigate")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panels::panel::SelectionState;

    fn test_issue() -> Issue {
        Issue {
            number: 42,
            title: "Fix memory leak in parser".into(),
            author: "octocat".into(),
            state: "open".into(),
            labels: vec!["bug".into(), "high-priority".into()],
            assignees: vec!["dev1".into(), "dev2".into()],
            created_at: "2024-01-15T10:30:00Z".into(),
            updated_at: "2024-01-16T14:20:00Z".into(),
            body: Some("This is a test issue body\nwith multiple lines.".into()),
        }
    }

    #[test]
    fn left_content_builds_list_items() {
        let issue = test_issue();
        let mut data = ListPanelData::<Issue>::new();
        let mut selection = SelectionState::list();
        data.set_items(vec![issue], &mut selection);
        let panel = IssuesPanelItem::default();
        let theme = Theme::default();

        let content = panel.left_content(&data, &theme);
        assert!(matches!(content, LeftContent::List { .. }));
    }

    #[test]
    fn detail_lines_includes_all_fields() {
        let issue = test_issue();
        let panel = IssuesPanelItem::default();
        let theme = Theme::default();

        let lines = panel.detail_lines(&issue, &theme);

        let text: String = lines.iter().flat_map(|l| l.spans.iter().map(|s| s.content.as_ref())).collect::<Vec<&str>>().join(" ");
        assert!(text.contains("#42"), "should contain issue number");
        assert!(text.contains("Fix memory leak"), "should contain title");
        assert!(text.contains("octocat"), "should contain author");
        assert!(text.contains("open"), "should contain state");
        assert!(text.contains("bug"), "should contain bug label");
        assert!(text.contains("high-priority"), "should contain high-priority label");
        assert!(text.contains("dev1"), "should contain assignee dev1");
        assert!(text.contains("dev2"), "should contain assignee dev2");
    }

    #[test]
    fn handle_key_1_fetches_open_issues() {
        let mut panel = IssuesPanelItem::default();
        let mut data = ListPanelData::<Issue>::new();
        data.set_repo("owner".into(), "repo".into());

        let cmd = panel.handle_key('1', None, &data);
        assert!(matches!(cmd, Some(Cmd::FetchIssues { state, .. }) if state.as_str() == "open"));
        assert_eq!(panel.state_filter, "open");
    }

    #[test]
    fn handle_key_2_fetches_closed_issues() {
        let mut panel = IssuesPanelItem::default();
        let mut data = ListPanelData::<Issue>::new();
        data.set_repo("owner".into(), "repo".into());

        let cmd = panel.handle_key('2', None, &data);
        assert!(matches!(cmd, Some(Cmd::FetchIssues { state, .. }) if state.as_str() == "closed"));
        assert_eq!(panel.state_filter, "closed");
    }

    #[test]
    fn handle_key_3_fetches_all_issues() {
        let mut panel = IssuesPanelItem::default();
        let mut data = ListPanelData::<Issue>::new();
        data.set_repo("owner".into(), "repo".into());

        let cmd = panel.handle_key('3', None, &data);
        assert!(matches!(cmd, Some(Cmd::FetchIssues { state, .. }) if state.as_str() == "all"));
        assert_eq!(panel.state_filter, "all");
    }

    #[test]
    fn handle_key_filter_returns_none_when_no_repo() {
        let mut panel = IssuesPanelItem::default();
        let data = ListPanelData::<Issue>::new(); // owner and repo are None

        assert!(panel.handle_key('1', None, &data).is_none());
        assert!(panel.handle_key('2', None, &data).is_none());
        assert!(panel.handle_key('3', None, &data).is_none());
    }

    #[test]
    fn extract_items_issues_returns_some() {
        let issue = test_issue();
        let payload = ApiPayload::Issues(vec![issue]);
        let result = IssuesPanelItem::extract_items(payload);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn extract_items_pulls_returns_none() {
        let payload = ApiPayload::Pulls(vec![]);
        let result = IssuesPanelItem::extract_items(payload);
        assert!(result.is_none());
    }

    #[test]
    fn extract_items_empty_issues_returns_some_empty() {
        let result = IssuesPanelItem::extract_items(ApiPayload::Issues(vec![]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn left_content_empty_items() {
        let data = ListPanelData::<Issue>::new();
        let item: IssuesPanelItem = Default::default();
        let theme = Theme::default();
        let content = item.left_content(&data, &theme);
        match content {
            LeftContent::Table { rows, .. } => assert!(rows.is_empty()),
            LeftContent::List { items, .. } => assert!(items.is_empty()),
        }
    }
}
