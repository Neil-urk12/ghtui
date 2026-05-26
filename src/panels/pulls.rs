use ratatui::prelude::*;
use ratatui::widgets::{Cell, Row};

use crate::cmd::Cmd;
use crate::domain::{FilterState, PullRequest};
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

    // Render reviews
    if let Some(reviews) = &pr.reviews {
        if !reviews.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("Reviews ({})", reviews.len()),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for review in reviews {
                let icon = match review.state.as_str() {
                    "APPROVED" => "✓",
                    "CHANGES_REQUESTED" | "DISMISSED" => "✗",
                    _ => "•",
                };
                lines.push(Line::from(format!("{} {}: {}", icon, review.author, review.state)));
                if let Some(body) = &review.body {
                    if !body.is_empty() {
                        lines.push(Line::from(format!("  {}", body)));
                    }
                }
            }
            lines.push(Line::from(""));
        }
    }

    // Render comments
    if let Some(comments) = &pr.comments {
        if !comments.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("Comments ({})", comments.len()),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for comment in comments {
                lines.push(Line::from(format!("{}: {}", comment.author, comment.body)));
            }
            lines.push(Line::from(""));
        }
    }

    // Render code comments
    if let Some(code_comments) = &pr.code_comments {
        if !code_comments.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("Code Comments ({})", code_comments.len()),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for comment in code_comments {
                let location = if let Some(line) = comment.line {
                    format!("{}:{}", comment.path, line)
                } else {
                    comment.path.clone()
                };
                lines.push(Line::from(format!("{}: {}", location, comment.body)));
            }
            lines.push(Line::from(""));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "1:open 2:closed 3:all | o:browser | c:checkout | a:approve | m:merge | x:close | X:reopen",
        theme.muted,
    )));

    lines
}

#[derive(Debug, Default)]
pub struct PullsPanelItem {
    pub state_filter: FilterState,
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
                self.state_filter = FilterState::Open;
                if let Some((owner, repo)) = data.owner_repo() {
                    Some(Cmd::FetchPulls {
                        owner: owner.to_string(),
                        repo: repo.to_string(),
                        state: self.state_filter.as_str().to_string(),
                    })
                } else {
                    None
                }
            }
            '2' => {
                self.state_filter = FilterState::Closed;
                if let Some((owner, repo)) = data.owner_repo() {
                    Some(Cmd::FetchPulls {
                        owner: owner.to_string(),
                        repo: repo.to_string(),
                        state: self.state_filter.as_str().to_string(),
                    })
                } else {
                    None
                }
            }
            '3' => {
                self.state_filter = FilterState::All;
                if let Some((owner, repo)) = data.owner_repo() {
                    Some(Cmd::FetchPulls {
                        owner: owner.to_string(),
                        repo: repo.to_string(),
                        state: self.state_filter.as_str().to_string(),
                    })
                } else {
                    None
                }
            }
            'o' => {
                if let Some(pr) = selected {
                    if let Some((owner, repo)) = data.owner_repo() {
                        crate::util::open_in_browser(pr.number, owner, repo);
                    }
                }
                None
            }
            'a' => {
                if let Some(pr) = selected {
                    if let Some((owner, repo)) = data.owner_repo() {
                        return Some(Cmd::ApprovePr {
                            owner: owner.to_string(),
                            repo: repo.to_string(),
                            pr_number: pr.number as u64,
                        });
                    }
                }
                None
            }
            'm' => {
                if let Some(pr) = selected {
                    if let Some((owner, repo)) = data.owner_repo() {
                        return Some(Cmd::MergePr {
                            owner: owner.to_string(),
                            repo: repo.to_string(),
                            pr_number: pr.number as u64,
                        });
                    }
                }
                None
            }

            'x' => {
                if let Some(pr) = selected {
                    if pr.state == "open" {
                        if let Some((owner, repo)) = data.owner_repo() {
                            return Some(Cmd::ClosePr {
                                owner: owner.to_string(),
                                repo: repo.to_string(),
                                pr_number: pr.number as u64,
                            });
                        }
                    }
                }
                None
            }
            'X' => {
                if let Some(pr) = selected {
                    if pr.state == "closed" {
                        if let Some((owner, repo)) = data.owner_repo() {
                            return Some(Cmd::ReopenPr {
                                owner: owner.to_string(),
                                repo: repo.to_string(),
                                pr_number: pr.number as u64,
                            });
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn on_select(&self, selected: Option<&Self::Item>, data: &ListPanelData<Self::Item>) -> Option<Cmd> {
        if let Some(pr) = selected {
            if pr.comments.is_none() || pr.reviews.is_none() || pr.code_comments.is_none() {
                if let Some((owner, repo)) = data.owner_repo() {
                    return Some(Cmd::FetchPrComments {
                        owner: owner.to_string(),
                        repo: repo.to_string(),
                        pr_number: pr.number as u64,
                    });
                }
            }
        }
        None
    }

    fn fetch_cmd(&self, data: &ListPanelData<Self::Item>) -> Cmd {
        if let Some((owner, repo)) = data.owner_repo() {
            Cmd::FetchPulls {
                owner: owner.to_string(),
                repo: repo.to_string(),
                state: self.state_filter.as_str().to_string(),
            }
        } else {
            Cmd::None
        }
    }

    fn block_title(&self, data: &ListPanelData<Self::Item>) -> String {
        if let Some((o, r)) = data.owner_repo() {
            format!(" PRs: {o}/{r} ({}) ", self.state_filter.as_str())
        } else {
            format!(" Pull Requests ({}) ", self.state_filter.as_str())
        }
    }

    fn empty_state(&self, data: &ListPanelData<Self::Item>) -> EmptyState {
        data.repo_aware_empty_state("No pull requests found", " ", "Press 1 for open, 2 for closed, 3 for all · r to refresh")
    }

    extract_items!(Pulls);

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

    fn apply_filter(&self, data: &mut ListPanelData<Self::Item>, query: &str, selection: &mut SelectionState) {
        data.filter_by(query, |item, q| {
            item.title.to_lowercase().contains(q)
                || item.author.to_lowercase().contains(q)
                || item.labels.iter().any(|l| l.to_lowercase().contains(q))
        }, selection);
    }

    fn initial_selection(&self) -> SelectionState {
        SelectionState::table()
    }


    fn item_display(&self, item: &Self::Item) -> Option<String> {
        Some(format!("#{} {}", item.number, item.title))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::ApiPayload;

    fn test_pr() -> PullRequest {
        PullRequest {
            number: 42,
            title: "Fix login bug".into(),
            author: "octocat".into(),
            state: "open".into(),
            labels: vec!["bug".into(), "security".into()],
            created_at: "2024-01-15T10:30:00Z".into(),
            updated_at: "2024-01-16T14:00:00Z".into(),
            body: Some("This fixes the login redirect issue.".into()),
            comments: None,
            reviews: None,
            code_comments: None,
        }
    }

    fn make_data(pr: PullRequest) -> ListPanelData<PullRequest> {
        let mut data = ListPanelData::<PullRequest>::new();
        data.items = vec![pr];
        data.filtered_indices = vec![0];
        data.owner = Some("octocat".into());
        data.repo = Some("hello-world".into());
        data
    }

    // --- left_content ---

    #[test]
    fn left_content_returns_table_with_pr_info() {
        let pr = test_pr();
        let data = make_data(pr);
        let item = PullsPanelItem::default();
        let theme = Theme::default();

        let content = item.left_content(&data, &theme);

        match content {
            LeftContent::Table { headers, rows, widths } => {
                assert_eq!(headers, vec!["#", "Title", "Author", "Status", "Updated"]);
                assert_eq!(rows.len(), 1);
                assert_eq!(widths.len(), 5);
            }
            _ => panic!("expected Table variant"),
        }
    }

    // --- detail_lines ---

    #[test]
    fn detail_lines_includes_title_author_labels_body() {
        let pr = test_pr();
        let item = PullsPanelItem::default();
        let theme = Theme::default();

        let lines = item.detail_lines(&pr, &theme);

        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("#42"), "should contain PR number");
        assert!(text.contains("Fix login bug"), "should contain title");
        assert!(text.contains("octocat"), "should contain author");
        assert!(text.contains("bug"), "should contain label");
        assert!(text.contains("security"), "should contain label");
        assert!(text.contains("login redirect"), "should contain body");
    }

    #[test]
    fn detail_lines_no_body_shows_placeholder() {
        let pr = PullRequest {
            number: 1,
            title: "Empty PR".into(),
            author: "dev".into(),
            state: "open".into(),
            labels: vec![],
            created_at: "2024-01-15T10:30:00Z".into(),
            updated_at: "2024-01-15T10:30:00Z".into(),
            body: None,
            comments: None,
            reviews: None,
            code_comments: None,
        };
        let item = PullsPanelItem::default();
        let theme = Theme::default();

        let lines = item.detail_lines(&pr, &theme);

        let has_placeholder = lines.iter().any(|l| l.to_string().contains("(no description)"));
        assert!(has_placeholder);
    }

    // --- handle_key ---

    #[test]
    fn handle_key_a_returns_approve_cmd() {
        let pr = test_pr();
        let data = make_data(pr);
        let mut item = PullsPanelItem::default();

        let selected = data.items.first();
        let cmd = item.handle_key('a', selected, &data);

        assert!(cmd.is_some());
        assert!(matches!(
            cmd.unwrap(),
            Cmd::ApprovePr { owner, repo, pr_number }
                if owner == "octocat" && repo == "hello-world" && pr_number == 42
        ));
    }

    #[test]
    fn handle_key_m_returns_merge_cmd() {
        let pr = test_pr();
        let data = make_data(pr);
        let mut item = PullsPanelItem::default();

        let selected = data.items.first();
        let cmd = item.handle_key('m', selected, &data);

        assert!(cmd.is_some());
        assert!(matches!(
            cmd.unwrap(),
            Cmd::MergePr { owner, repo, pr_number }
                if owner == "octocat" && repo == "hello-world" && pr_number == 42
        ));
    }

    #[test]
    fn handle_key_a_no_owner_returns_none() {
        let pr = test_pr();
        let mut data = make_data(pr);
        data.owner = None;
        data.repo = None;
        let mut item = PullsPanelItem::default();

        let selected = data.items.first();
        let cmd = item.handle_key('a', selected, &data);

        assert!(cmd.is_none());
    }

    #[test]
    fn handle_key_no_selection_returns_none() {
        let pr = test_pr();
        let data = make_data(pr);
        let mut item = PullsPanelItem::default();

        let cmd = item.handle_key('a', None, &data);

        assert!(cmd.is_none());
    }

    #[test]
    fn handle_key_filter_1_sets_open_state() {
        let data = ListPanelData::<PullRequest>::new();
        let mut item = PullsPanelItem::default();

        let cmd = item.handle_key('1', None, &data);

        // Without owner/repo, returns None but sets the filter
        assert_eq!(item.state_filter, FilterState::Open);
        assert!(cmd.is_none());
    }

    #[test]
    fn handle_key_filter_1_with_repo_returns_fetch_cmd() {
        let pr = test_pr();
        let data = make_data(pr);
        let mut item = PullsPanelItem::default();

        let cmd = item.handle_key('1', None, &data);

        assert_eq!(item.state_filter, FilterState::Open);
        assert!(matches!(
            cmd.unwrap(),
            Cmd::FetchPulls { owner, repo, state }
                if owner == "octocat" && repo == "hello-world" && state == "open"
        ));
    }

    // --- extract_items ---

    #[test]
    fn extract_items_pulls_returns_some() {
        let pr = test_pr();
        let payload = ApiPayload::Pulls(vec![pr]);

        let result = PullsPanelItem::extract_items(payload);

        assert!(result.is_some());
        let pulls = result.unwrap();
        assert_eq!(pulls.len(), 1);
        assert_eq!(pulls[0].number, 42);
    }

    #[test]
    fn extract_items_issues_returns_none() {
        let payload = ApiPayload::Issues(vec![]);

        let result = PullsPanelItem::extract_items(payload);

        assert!(result.is_none());
    }

    #[test]
    fn extract_items_empty_pulls_returns_some_empty_vec() {
        let payload = ApiPayload::Pulls(vec![]);

        let result = PullsPanelItem::extract_items(payload);

        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn apply_filter_by_title() {
        let mut data = ListPanelData::<PullRequest>::new();
        let mut selection = SelectionState::table();
        let items = vec![
            PullRequest { number: 1, title: "Fix login bug".into(), ..Default::default() },
            PullRequest { number: 2, title: "Add dark mode".into(), ..Default::default() },
            PullRequest { number: 3, title: "Fix signup bug".into(), ..Default::default() },
        ];
        data.set_items(items, &mut selection);
        let panel = PullsPanelItem::default();
        panel.apply_filter(&mut data, "bug", &mut selection);
        assert_eq!(data.filtered_indices, vec![0, 2]);
    }

    #[test]
    fn apply_filter_by_author() {
        let mut data = ListPanelData::<PullRequest>::new();
        let mut selection = SelectionState::table();
        let items = vec![
            PullRequest { number: 1, title: "PR 1".into(), author: "octocat".into(), ..Default::default() },
            PullRequest { number: 2, title: "PR 2".into(), author: "dev1".into(), ..Default::default() },
            PullRequest { number: 3, title: "PR 3".into(), author: "octocat".into(), ..Default::default() },
        ];
        data.set_items(items, &mut selection);
        let panel = PullsPanelItem::default();
        panel.apply_filter(&mut data, "octocat", &mut selection);
        assert_eq!(data.filtered_indices, vec![0, 2]);
    }
    #[test]
    fn left_content_empty_items() {
        let data = ListPanelData::<PullRequest>::new();
        let item: PullsPanelItem = Default::default();
        let theme = Theme::default();
        let content = item.left_content(&data, &theme);
        match content {
            LeftContent::Table { rows, .. } => assert!(rows.is_empty()),
            LeftContent::List { items, .. } => assert!(items.is_empty()),
        }
    }

    #[test]
    fn detail_footer_contains_self_documenting_hints() {
        let pr = test_pr();
        let item = PullsPanelItem::default();
        let theme = Theme::default();

        let lines = item.detail_lines(&pr, &theme);
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");

        assert!(
            text.contains("1:open 2:closed 3:all | o:browser | c:checkout | a:approve | m:merge"),
            "footer should contain self-documenting key hints, got: {text}"
        );
    }

    #[test]
    fn empty_state_shows_prose() {
        let pr = test_pr();
        let mut data = make_data(pr);
        // Populate items but clear filtered to trigger the filter-empty branch
        data.filtered_indices = vec![];
        let item = PullsPanelItem::default();

        let state = item.empty_state(&data);

        assert!(
            state.hint.as_deref().unwrap_or("").contains("Press 1 for open, 2 for closed, 3 for all · r to refresh"),
            "empty state hint should show prose, got: {:?}",
            state.hint
        );
    }

    // --- x/X key handlers ---

    #[test]
    fn handle_key_x_returns_close_pr_cmd() {
        let pr = test_pr(); // state is "open"
        let data = make_data(pr);
        let mut item = PullsPanelItem::default();

        let selected = data.items.first();
        let cmd = item.handle_key('x', selected, &data);

        assert!(cmd.is_some());
        assert!(matches!(
            cmd.unwrap(),
            Cmd::ClosePr { owner, repo, pr_number }
                if owner == "octocat" && repo == "hello-world" && pr_number == 42
        ));
    }

    #[test]
    fn handle_key_x_on_closed_pr_returns_none() {
        let pr = PullRequest {
            state: "closed".into(),
            ..test_pr()
        };
        let data = make_data(pr);
        let mut item = PullsPanelItem::default();

        let selected = data.items.first();
        let cmd = item.handle_key('x', selected, &data);

        assert!(cmd.is_none());
    }

    #[test]
    fn handle_key_uppercase_x_returns_reopen_pr_cmd() {
        let pr = PullRequest {
            state: "closed".into(),
            ..test_pr()
        };
        let data = make_data(pr);
        let mut item = PullsPanelItem::default();

        let selected = data.items.first();
        let cmd = item.handle_key('X', selected, &data);

        assert!(cmd.is_some());
        assert!(matches!(
            cmd.unwrap(),
            Cmd::ReopenPr { owner, repo, pr_number }
                if owner == "octocat" && repo == "hello-world" && pr_number == 42
        ));
    }

    #[test]
    fn handle_key_uppercase_x_on_open_pr_returns_none() {
        let pr = test_pr(); // state is "open"
        let data = make_data(pr);
        let mut item = PullsPanelItem::default();

        let selected = data.items.first();
        let cmd = item.handle_key('X', selected, &data);

        assert!(cmd.is_none());
    }

    // --- comment rendering ---

    #[test]
    fn detail_lines_shows_reviews_when_present() {
        let pr = PullRequest {
            reviews: Some(vec![
                crate::domain::PrReview {
                    id: 1,
                    author: "reviewer".into(),
                    state: "APPROVED".into(),
                    body: None,
                    submitted_at: "2024-01-15T10:30:00Z".into(),
                },
            ]),
            ..test_pr()
        };
        let item = PullsPanelItem::default();
        let theme = Theme::default();

        let lines = item.detail_lines(&pr, &theme);
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");

        assert!(text.contains("Reviews (1)"), "should contain reviews header");
        assert!(text.contains("✓ reviewer: APPROVED"), "should contain approval");
    }

    #[test]
    fn detail_lines_shows_comments_when_present() {
        let pr = PullRequest {
            comments: Some(vec![
                crate::domain::PrComment {
                    id: 1,
                    author: "user".into(),
                    body: "Looks good!".into(),
                    created_at: "2024-01-15T10:30:00Z".into(),
                },
            ]),
            ..test_pr()
        };
        let item = PullsPanelItem::default();
        let theme = Theme::default();

        let lines = item.detail_lines(&pr, &theme);
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");

        assert!(text.contains("Comments (1)"), "should contain comments header");
        assert!(text.contains("user: Looks good!"), "should contain comment");
    }

    #[test]
    fn detail_lines_shows_code_comments_when_present() {
        let pr = PullRequest {
            code_comments: Some(vec![
                crate::domain::PrCodeComment {
                    id: 1,
                    author: "reviewer".into(),
                    body: "Fix this".into(),
                    path: "src/main.rs".into(),
                    line: Some(42),
                    created_at: "2024-01-15T10:30:00Z".into(),
                },
            ]),
            ..test_pr()
        };
        let item = PullsPanelItem::default();
        let theme = Theme::default();

        let lines = item.detail_lines(&pr, &theme);
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");

        assert!(text.contains("Code Comments (1)"), "should contain code comments header");
        assert!(text.contains("src/main.rs:42"), "should contain file:line");
        assert!(text.contains("Fix this"), "should contain comment body");
    }

    #[test]
    fn detail_lines_no_comments_shows_no_sections() {
        let pr = test_pr(); // No comments
        let item = PullsPanelItem::default();
        let theme = Theme::default();

        let lines = item.detail_lines(&pr, &theme);
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");

        assert!(!text.contains("Reviews"), "should not contain reviews header");
        assert!(!text.contains("Comments"), "should not contain comments header");
        assert!(!text.contains("Code Comments"), "should not contain code comments header");
    }

    #[test]
    fn detail_lines_empty_comments_shows_no_sections() {
        let pr = PullRequest {
            comments: Some(vec![]),
            reviews: Some(vec![]),
            code_comments: Some(vec![]),
            ..test_pr()
        };
        let item = PullsPanelItem::default();
        let theme = Theme::default();

        let lines = item.detail_lines(&pr, &theme);
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");

        assert!(!text.contains("Reviews"), "should not contain reviews header");
        assert!(!text.contains("Comments"), "should not contain comments header");
        assert!(!text.contains("Code Comments"), "should not contain code comments header");
    }
}
