use super::*;
use crate::util::diff::parse_hunk_header;
use crate::event::ApiPayload;
use crate::widgets::detail_pane::DetailPane;
use std::time::Duration;

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
        comment_errors: None,
        review_errors: None,
        code_comment_errors: None,
        files: None,
        file_errors: None,
        merged_at: None,
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
        LeftContent::Table {
            headers,
            rows,
            widths,
        } => {
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

    let text: String = lines
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("#42"), "should contain PR number");
    assert!(text.contains("Fix login bug"), "should contain title");
    assert!(text.contains("octocat"), "should contain author");
    assert!(text.contains("bug"), "should contain label");
    assert!(text.contains("security"), "should contain label");
    assert!(text.contains("login redirect"), "should contain body");
}

#[test]
fn detail_lines_snapshot() {
    let pr = test_pr();
    let item = PullsPanelItem::default();
    let theme = Theme::default();
    let lines = item.detail_lines(&pr, &theme);
    let text: String = lines
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!("pr_detail_lines", text);
}

#[test]
fn detail_lines_no_body_shows_placeholder() {
    let pr = PullRequest {
        number: 1,
        title: "Empty PR".into(),
        author: "dev".into(),
        body: None,
        labels: vec![],
        created_at: "2024-01-15T10:30:00Z".into(),
        updated_at: "2024-01-15T10:30:00Z".into(),
        ..test_pr()
    };
    let item = PullsPanelItem::default();
    let theme = Theme::default();

    let lines = item.detail_lines(&pr, &theme);

    let has_placeholder = lines
        .iter()
        .any(|l| l.to_string().contains("(no description)"));
    assert!(has_placeholder);
}

// --- handle_key ---

#[test]
fn handle_key_a_returns_approve_cmd() {
    let pr = test_pr();
    let data = make_data(pr);
    let mut item = PullsPanelItem::default();

    let selected = data.items.first();
    let cmd = item.handle_key('a', selected, &data, &mut DetailPane::default());

    assert!(cmd.is_some());
    assert!(matches!(
        cmd.unwrap(),
        Cmd::ApprovePr { ctx, pr_number }
            if ctx.owner == "octocat" && ctx.repo == "hello-world" && pr_number == 42
    ));
}

#[test]
fn handle_key_m_returns_merge_cmd() {
    let pr = test_pr();
    let data = make_data(pr);
    let mut item = PullsPanelItem::default();

    let selected = data.items.first();
    let cmd = item.handle_key('m', selected, &data, &mut DetailPane::default());

    assert!(cmd.is_some());
    assert!(matches!(
        cmd.unwrap(),
        Cmd::MergePr { ctx, pr_number }
            if ctx.owner == "octocat" && ctx.repo == "hello-world" && pr_number == 42
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
    let cmd = item.handle_key('a', selected, &data, &mut DetailPane::default());

    assert!(cmd.is_none());
}

#[test]
fn handle_key_no_selection_returns_none() {
    let pr = test_pr();
    let data = make_data(pr);
    let mut item = PullsPanelItem::default();

    let cmd = item.handle_key('a', None, &data, &mut DetailPane::default());

    assert!(cmd.is_none());
}

#[test]
fn handle_key_filter_1_sets_open_state() {
    let data = ListPanelData::<PullRequest>::new();
    let mut item = PullsPanelItem::default();

    let cmd = item.handle_key('1', None, &data, &mut DetailPane::default());

    // Without owner/repo, returns None but sets the filter
    assert_eq!(item.state_filter, FilterState::Open);
    assert!(cmd.is_none());
}

#[test]
fn handle_key_filter_1_with_repo_returns_none_when_already_open() {
    let pr = test_pr();
    let data = make_data(pr);
    let mut item = PullsPanelItem::default();

    // Default is Open, pressing '1' again is a no-op
    let cmd = item.handle_key('1', None, &data, &mut DetailPane::default());
    assert_eq!(item.state_filter, FilterState::Open);
    assert!(cmd.is_none());

    // Pressing '2' switches to Closed
    let cmd = item.handle_key('2', None, &data, &mut DetailPane::default());
    assert_eq!(item.state_filter, FilterState::Closed);
    assert!(cmd.is_some());

    // Pressing '1' now switches back to Open and fetches
    let cmd = item.handle_key('1', None, &data, &mut DetailPane::default());
    assert_eq!(item.state_filter, FilterState::Open);
    assert!(matches!(
        cmd.unwrap(),
        Cmd::FetchPulls { ctx, state }
            if ctx.owner == "octocat" && ctx.repo == "hello-world" && state == FilterState::Open
        ));
}

#[test]
fn handle_key_filter_2_noop_when_already_closed() {
    let pr = test_pr();
    let data = make_data(pr);
    let mut item = PullsPanelItem::default();

    // Switch to Closed first
    let cmd = item.handle_key('2', None, &data, &mut DetailPane::default());
    assert_eq!(item.state_filter, FilterState::Closed);
    assert!(cmd.is_some());

    // Pressing '2' again is a no-op
    let cmd = item.handle_key('2', None, &data, &mut DetailPane::default());
    assert_eq!(item.state_filter, FilterState::Closed);
    assert!(cmd.is_none());

    // Switch to All
    let cmd = item.handle_key('3', None, &data, &mut DetailPane::default());
    assert_eq!(item.state_filter, FilterState::All);
    assert!(cmd.is_some());

    // Pressing '2' switches back to Closed and fetches
    let cmd = item.handle_key('2', None, &data, &mut DetailPane::default());
    assert_eq!(item.state_filter, FilterState::Closed);
    assert!(matches!(
        cmd.unwrap(),
        Cmd::FetchPulls { ctx, state }
            if ctx.owner == "octocat" && ctx.repo == "hello-world" && state == FilterState::Closed
        ));
}

#[test]
fn handle_key_filter_3_noop_when_already_all() {
    let pr = test_pr();
    let data = make_data(pr);
    let mut item = PullsPanelItem::default();

    // Switch to All first
    let cmd = item.handle_key('3', None, &data, &mut DetailPane::default());
    assert_eq!(item.state_filter, FilterState::All);
    assert!(cmd.is_some());

    // Pressing '3' again is a no-op
    let cmd = item.handle_key('3', None, &data, &mut DetailPane::default());
    assert_eq!(item.state_filter, FilterState::All);
    assert!(cmd.is_none());

    // Switch to Open
    let cmd = item.handle_key('1', None, &data, &mut DetailPane::default());
    assert_eq!(item.state_filter, FilterState::Open);
    assert!(cmd.is_some());

    // Pressing '3' switches back to All and fetches
    let cmd = item.handle_key('3', None, &data, &mut DetailPane::default());
    assert_eq!(item.state_filter, FilterState::All);
    assert!(matches!(
        cmd.unwrap(),
        Cmd::FetchPulls { ctx, state }
            if ctx.owner == "octocat" && ctx.repo == "hello-world" && state == FilterState::All
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
        PullRequest {
            number: 1,
            title: "Fix login bug".into(),
            ..Default::default()
        },
        PullRequest {
            number: 2,
            title: "Add dark mode".into(),
            ..Default::default()
        },
        PullRequest {
            number: 3,
            title: "Fix signup bug".into(),
            ..Default::default()
        },
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
        PullRequest {
            number: 1,
            title: "PR 1".into(),
            author: "octocat".into(),
            ..Default::default()
        },
        PullRequest {
            number: 2,
            title: "PR 2".into(),
            author: "dev1".into(),
            ..Default::default()
        },
        PullRequest {
            number: 3,
            title: "PR 3".into(),
            author: "octocat".into(),
            ..Default::default()
        },
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
    let text: String = lines
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        text.contains("1:open 2:closed 3:all | o:browser | d:diff | f:fullscreen | c:collapse | a:approve | m:merge"),
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
        state
            .hint
            .as_deref()
            .unwrap_or("")
            .contains("Press 1 for open, 2 for closed, 3 for all · r to refresh"),
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
    let cmd = item.handle_key('x', selected, &data, &mut DetailPane::default());

    assert!(cmd.is_some());
    assert!(matches!(
        cmd.unwrap(),
        Cmd::ClosePr { ctx, pr_number }
            if ctx.owner == "octocat" && ctx.repo == "hello-world" && pr_number == 42
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
    let cmd = item.handle_key('x', selected, &data, &mut DetailPane::default());

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
    let cmd = item.handle_key('X', selected, &data, &mut DetailPane::default());

    assert!(cmd.is_some());
    assert!(matches!(
        cmd.unwrap(),
        Cmd::ReopenPr { ctx, pr_number }
            if ctx.owner == "octocat" && ctx.repo == "hello-world" && pr_number == 42
    ));
}

#[test]
fn handle_key_uppercase_x_on_open_pr_returns_none() {
    let pr = test_pr(); // state is "open"
    let data = make_data(pr);
    let mut item = PullsPanelItem::default();

    let selected = data.items.first();
    let cmd = item.handle_key('X', selected, &data, &mut DetailPane::default());

    assert!(cmd.is_none());
}

// --- d/f/c key handlers (detail pane toggles) ---

#[test]
fn handle_key_f_toggles_fullscreen() {
    let data = make_data(test_pr());
    let mut item = PullsPanelItem::default();
    let mut dp = DetailPane::default();

    let selected = data.items.first();
    let cmd = item.handle_key('f', selected, &data, &mut dp);

    assert!(matches!(cmd, Some(Cmd::None)));
    assert_eq!(dp.mode, crate::widgets::detail_pane::DetailPaneMode::Fullscreen);
}

#[test]
fn handle_key_c_toggles_collapse() {
    let data = make_data(test_pr());
    let mut item = PullsPanelItem::default();
    let mut dp = DetailPane::default();

    let selected = data.items.first();
    let cmd = item.handle_key('c', selected, &data, &mut dp);

    assert!(matches!(cmd, Some(Cmd::None)));
    assert_eq!(dp.mode, crate::widgets::detail_pane::DetailPaneMode::Collapsed);
}

#[test]
fn handle_key_d_toggles_diff_with_selection() {
    let data = make_data(test_pr());
    let mut item = PullsPanelItem::default();
    let mut dp = DetailPane::default();

    let selected = data.items.first();
    let cmd = item.handle_key('d', selected, &data, &mut dp);

    assert!(matches!(cmd, Some(Cmd::None)));
    assert!(dp.showing_diff);
}

#[test]
fn handle_key_d_no_selection_does_not_show_diff() {
    let data = make_data(test_pr());
    let mut item = PullsPanelItem::default();
    let mut dp = DetailPane::default();

    let cmd = item.handle_key('d', None, &data, &mut dp);

    assert!(matches!(cmd, Some(Cmd::None)));
    assert!(!dp.showing_diff);
}

#[test]
fn handle_key_d_no_selection_preserves_existing_diff_state() {
    let data = make_data(test_pr());
    let mut item = PullsPanelItem::default();
    let mut dp = DetailPane { showing_diff: true, ..Default::default() };

    let cmd = item.handle_key('d', None, &data, &mut dp);

    assert!(matches!(cmd, Some(Cmd::None)));
    assert!(dp.showing_diff, "diff state should be preserved when no selection");
}

// --- comment rendering ---

#[test]
fn detail_lines_shows_reviews_when_present() {
    let pr = PullRequest {
        reviews: Some(vec![crate::domain::PrReview {
            id: 1,
            author: "reviewer".into(),
            state: "APPROVED".into(),
            body: None,
            submitted_at: "2024-01-15T10:30:00Z".into(),
        }]),
        ..test_pr()
    };
    let item = PullsPanelItem::default();
    let theme = Theme::default();

    let lines = item.detail_lines(&pr, &theme);
    let text: String = lines
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        text.contains("Reviews (1)"),
        "should contain reviews header"
    );
    assert!(
        text.contains("✓ reviewer: APPROVED"),
        "should contain approval"
    );
}

#[test]
fn detail_lines_shows_comments_when_present() {
    let pr = PullRequest {
        comments: Some(vec![crate::domain::PrComment {
            id: 1,
            author: "user".into(),
            body: "Looks good!".into(),
            created_at: "2024-01-15T10:30:00Z".into(),
        }]),
        ..test_pr()
    };
    let item = PullsPanelItem::default();
    let theme = Theme::default();

    let lines = item.detail_lines(&pr, &theme);
    let text: String = lines
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        text.contains("Comments (1)"),
        "should contain comments header"
    );
    assert!(text.contains("user: Looks good!"), "should contain comment");
}

#[test]
fn detail_lines_shows_code_comments_when_present() {
    let pr = PullRequest {
        code_comments: Some(vec![crate::domain::PrCodeComment {
            id: 1,
            author: "reviewer".into(),
            body: "Fix this".into(),
            path: "src/main.rs".into(),
            line: Some(42),
            created_at: "2024-01-15T10:30:00Z".into(),
        }]),
        ..test_pr()
    };
    let item = PullsPanelItem::default();
    let theme = Theme::default();

    let lines = item.detail_lines(&pr, &theme);
    let text: String = lines
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        text.contains("Code Comments (1)"),
        "should contain code comments header"
    );
    assert!(text.contains("src/main.rs:42"), "should contain file:line");
    assert!(text.contains("Fix this"), "should contain comment body");
}

#[test]
fn detail_lines_no_comments_shows_no_sections() {
    let pr = test_pr(); // No comments
    let item = PullsPanelItem::default();
    let theme = Theme::default();

    let lines = item.detail_lines(&pr, &theme);
    let text: String = lines
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !text.contains("Reviews"),
        "should not contain reviews header"
    );
    assert!(
        !text.contains("Comments"),
        "should not contain comments header"
    );
    assert!(
        !text.contains("Code Comments"),
        "should not contain code comments header"
    );
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
    let text: String = lines
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !text.contains("Reviews"),
        "should not contain reviews header"
    );
    assert!(
        !text.contains("Comments"),
        "should not contain comments header"
    );
    assert!(
        !text.contains("Code Comments"),
        "should not contain code comments header"
    );
}

#[test]
fn on_select_returns_none_within_debounce_window() {
    let pr = test_pr();
    let data = make_data(pr);
    let item = PullsPanelItem::default();

    let selected = data.items.first();
    let cmd1 = item.on_select(selected, &data);
    assert!(cmd1.is_some(), "first on_select should trigger fetch");

    // Same PR within debounce — should be None
    let cmd2 = item.on_select(selected, &data);
    assert!(
        cmd2.is_none(),
        "same PR within debounce window should return None"
    );
}

#[test]
fn on_select_different_pr_bypasses_debounce() {
    let pr1 = PullRequest {
        number: 1,
        ..test_pr()
    };
    let pr2 = PullRequest {
        number: 2,
        ..test_pr()
    };
    let mut data = ListPanelData::<PullRequest>::new();
    data.items = vec![pr1, pr2];
    data.filtered_indices = vec![0, 1];
    data.owner = Some("octocat".into());
    data.repo = Some("hello-world".into());

    let item = PullsPanelItem::default();

    // Select PR 1
    let selected = Some(&data.items[0]);
    let cmd1 = item.on_select(selected, &data);
    assert!(cmd1.is_some());

    // Select PR 2 immediately — different PR, should NOT be debounced
    let selected = Some(&data.items[1]);
    let cmd2 = item.on_select(selected, &data);
    assert!(cmd2.is_some(), "different PR should bypass debounce");
}

#[test]
fn detail_lines_renders_review_error() {
    let mut pr = test_pr();
    pr.review_errors = Some("rate limited".to_string());
    let lines = PullsPanelItem::default().detail_lines(&pr, &Theme::default());
    let text: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let all = text.join("\n");
    assert!(
        all.contains("Reviews: failed to load"),
        "should show review error"
    );
    assert!(all.contains("rate limited"), "should show error detail");
}

#[test]
fn detail_lines_renders_comment_error() {
    let mut pr = test_pr();
    pr.comment_errors = Some("403 forbidden".to_string());
    let lines = PullsPanelItem::default().detail_lines(&pr, &Theme::default());
    let text: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let all = text.join("\n");
    assert!(
        all.contains("Comments: failed to load"),
        "should show comment error"
    );
}

#[test]
fn detail_lines_renders_code_comment_error() {
    let mut pr = test_pr();
    pr.code_comment_errors = Some("timeout".to_string());
    let lines = PullsPanelItem::default().detail_lines(&pr, &Theme::default());
    let text: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let all = text.join("\n");
    assert!(
        all.contains("Code Comments: failed to load"),
        "should show code comment error"
    );
}

#[test]
fn detail_lines_no_errors_when_all_succeed() {
    let mut pr = test_pr();
    pr.reviews = Some(vec![]);
    pr.comments = Some(vec![]);
    pr.code_comments = Some(vec![]);
    // error fields are None by default from test_pr()
    let lines = PullsPanelItem::default().detail_lines(&pr, &Theme::default());
    let text: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let all = text.join("\n");
    assert!(
        !all.contains("failed to load"),
        "should not show errors when all succeed"
    );
}

#[test]
fn detail_lines_shows_partial_errors_with_mixed_results() {
    let mut pr = test_pr();
    pr.reviews = Some(vec![crate::domain::PrReview {
        id: 1,
        author: "r".to_string(),
        state: "APPROVED".to_string(),
        body: None,
        submitted_at: String::new(),
    }]);
    pr.comments = Some(vec![]);
    pr.review_errors = None;
    pr.comment_errors = None;
    pr.code_comment_errors = Some("500".to_string());
    let lines = PullsPanelItem::default().detail_lines(&pr, &Theme::default());
    let text: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let all = text.join("\n");
    assert!(
        all.contains("Code Comments: failed to load"),
        "should show code comment error"
    );
    assert!(
        !all.contains("Reviews: failed"),
        "should not show review error"
    );
}

#[test]
fn on_select_retries_on_first_select_after_error() {
    let mut pr = test_pr();
    pr.review_errors = Some("403 rate limited".to_string());
    let data = make_data(pr);
    let item = PullsPanelItem::default();

    let selected = data.items.first();
    let cmd = item.on_select(selected, &data);

    // With error-clearing fix: first select triggers retry (no debounce entry yet)
    assert!(
        cmd.is_some(),
        "should retry on first select even when error exists"
    );
}

#[test]
fn on_select_retries_after_error_past_debounce() {
    let mut pr = test_pr();
    pr.review_errors = Some("rate limited".to_string());
    // reviews stays None → needs_fetch = true
    let data = make_data(pr);
    let item = PullsPanelItem::default();

    // First call — fetches, sets debounce
    let selected = data.items.first();
    let cmd = item.on_select(selected, &data);
    assert!(cmd.is_some(), "first call should fetch");

    // Second call within debounce — blocked
    let cmd = item.on_select(selected, &data);
    assert!(cmd.is_none(), "within debounce should block");

    // Simulate time passing beyond debounce window
    item.last_fetch_attempts
        .borrow_mut()
        .insert(42, Instant::now() - Duration::from_millis(600));

    // Third call — past debounce, should retry despite prior error
    let cmd = item.on_select(selected, &data);
    assert!(
        cmd.is_some(),
        "should retry fetch after debounce expires, even with prior error"
    );
}

#[test]
fn on_select_still_debounces_error_retry() {
    let mut pr = test_pr();
    pr.comment_errors = Some("403".to_string());
    // comments stays None → needs_fetch = true
    let data = make_data(pr);
    let item = PullsPanelItem::default();

    // First call — triggers fetch, starts debounce
    let selected = data.items.first();
    let cmd = item.on_select(selected, &data);
    assert!(cmd.is_some(), "first call should trigger fetch");

    // Second call — immediate, within debounce window
    let cmd = item.on_select(selected, &data);
    assert!(cmd.is_none(), "should debounce rapid retry");
}

// --- diff rendering ---

#[test]
fn parse_diff_patch_basic_hunk() {
    let theme = Theme::default();
    let patch = "@@ -1,3 +1,4 @@\n line1\n+new line\n line2\n line3";
    let lines = parse_diff_patch(patch, &theme);
    // 1 hunk header + 4 content lines = 5
    assert_eq!(lines.len(), 5);
}

#[test]
fn parse_diff_patch_line_numbering() {
    let theme = Theme::default();
    let patch = "@@ -10,3 +10,4 @@\n context\n+added\n removed";
    let lines = parse_diff_patch(patch, &theme);
    // context line should show line 10
    let context_text = lines[1].to_string();
    assert!(context_text.contains("10"), "context line should show line 10, got: {context_text}");
    // added line should show line 11
    let added_text = lines[2].to_string();
    assert!(added_text.contains("11"), "added line should show line 11, got: {added_text}");
}

#[test]
fn parse_hunk_header_parses_line_numbers() {
    assert_eq!(parse_hunk_header("@@ -5,10 +20,15 @@"), Some((5, 20)));
    assert_eq!(parse_hunk_header("@@ -1 +1 @@"), Some((1, 1)));
    assert_eq!(parse_hunk_header("invalid"), None);
}

// --- diff_lines integration tests ---

#[test]
fn diff_lines_returns_styled_content_when_files_loaded() {
    let pr = PullRequest {
        files: Some(vec![crate::domain::PrFile {
            filename: "src/main.rs".into(),
            status: "modified".into(),
            additions: 5,
            deletions: 2,
            patch: Some("@@ -1,3 +1,4 @@\n line1\n+added".into()),
            previous_filename: None,
        }]),
        ..test_pr()
    };
    let item = PullsPanelItem::default();
    let theme = Theme::default();

    let lines = item.diff_lines(&pr, &theme);

    assert!(!lines.is_empty(), "should return non-empty content");
    let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
    assert!(text.contains("src/main.rs"), "should contain file header");
}

#[test]
fn diff_lines_returns_empty_when_files_none() {
    let pr = PullRequest {
        files: None,
        ..test_pr()
    };
    let item = PullsPanelItem::default();
    let theme = Theme::default();

    let lines = item.diff_lines(&pr, &theme);

    assert!(lines.is_empty(), "should return empty vec when files is None");
}

#[test]
fn detail_title_returns_pr_diff_when_showing_diff_true() {
    let item = PullsPanelItem::default();

    assert_eq!(item.detail_title(true), " PR Diff ");
    assert_eq!(item.detail_title(false), " PR Details ");
}

#[test]
fn large_patch_placeholder_renders_when_patch_none() {
    let pr = PullRequest {
        files: Some(vec![crate::domain::PrFile {
            filename: "huge_file.rs".into(),
            status: "modified".into(),
            additions: 100,
            deletions: 50,
            patch: None,
            previous_filename: None,
        }]),
        ..test_pr()
    };
    let item = PullsPanelItem::default();
    let theme = Theme::default();

    let lines = item.diff_lines(&pr, &theme);
    let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");

    assert!(
        text.contains("diff too large"),
        "should contain large patch placeholder, got: {text}"
    );
}

#[test]
fn renamed_file_shows_old_to_new_in_header() {
    let pr = PullRequest {
        files: Some(vec![crate::domain::PrFile {
            filename: "new.rs".into(),
            status: "renamed".into(),
            additions: 0,
            deletions: 0,
            patch: None,
            previous_filename: Some("old.rs".into()),
        }]),
        ..test_pr()
    };
    let item = PullsPanelItem::default();
    let theme = Theme::default();

    let lines = item.diff_lines(&pr, &theme);
    let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");

    assert!(
        text.contains("old.rs") && text.contains("new.rs"),
        "should contain old and new filenames, got: {text}"
    );
}

#[test]
fn detail_footer_contains_diff_hint() {
    let pr = test_pr();
    let item = PullsPanelItem::default();
    let theme = Theme::default();

    let lines = item.detail_lines(&pr, &theme);
    let text: String = lines
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        text.contains("d:diff"),
        "footer should contain d:diff hint, got: {text}"
    );
}

#[test]
fn parse_diff_patch_no_newline_marker_preserves_line_numbers() {
    let theme = Theme::default();
    let patch = "@@ -1,3 +1,3 @@\n line1\n line2\n\\ No newline at end of file\n line3";
    let lines = parse_diff_patch(patch, &theme);
    assert_eq!(lines.len(), 5);
    let line3_text = lines[4].to_string();
    assert!(line3_text.contains("3"), "line3 should show line 3, got: {line3_text}");
    assert!(!line3_text.contains("4"), "line3 should NOT show line 4, got: {line3_text}");
}

#[test]
fn parse_diff_patch_no_newline_marker_does_not_count_as_content() {
    let theme = Theme::default();
    let patch = "@@ -1,2 +1,2 @@\n+added\n\\ No newline at end of file\n-removed";
    let lines = parse_diff_patch(patch, &theme);
    // hunk header + added + marker + removed = 4
    assert_eq!(lines.len(), 4);
    // removed line should show line 1 (old_line), not line 2
    let removed_text = lines[3].to_string();
    assert!(removed_text.contains("1"), "removed should show old line 1, got: {removed_text}");
}

#[test]
fn diff_empty_state_is_distinct_from_detail_empty_state() {
    let item = PullsPanelItem::default();
    let detail_es = item.detail_empty_state();
    let diff_es = item.diff_empty_state();
    assert_ne!(diff_es.message, detail_es.message, "diff empty state should differ from detail empty state");
}

#[test]
fn implements_state_filterable_trait() {
    use crate::panels::panel::StateFilterable;

    let mut item = PullsPanelItem::default();
    assert_eq!(*item.filter_state(), FilterState::Open);

    let changed = item.set_filter(FilterState::Closed);
    assert!(changed);
    assert_eq!(*item.filter_state(), FilterState::Closed);

    // No-op when same filter
    let changed = item.set_filter(FilterState::Closed);
    assert!(!changed);
}

#[test]
fn parse_diff_patch_file_headers_rendered_as_muted() {
    let theme = Theme::default();
    let patch = "--- a/file.rs\n+++ b/file.rs\n@@ -1,1 +1,1 @@\n-old\n+new";
    let lines = parse_diff_patch(patch, &theme);
    // --- a/file, +++ b/file, @@ header, -old, +new = 5
    assert_eq!(lines.len(), 5);
    // First two lines (--- and +++) should be rendered as muted, not as deletions/additions
    // Verify they don't have the diff_deletion or diff_addition style by checking they use theme.muted
    let header_old = lines[0].to_string();
    let header_new = lines[1].to_string();
    assert!(header_old.contains("--- a/file.rs"), "first line should be --- header, got: {header_old}");
    assert!(header_new.contains("+++ b/file.rs"), "second line should be +++ header, got: {header_new}");
    // Check that these are single-span lines (muted style, not prefix + styled)
    assert_eq!(lines[0].spans.len(), 1, "--- header should be single muted span");
    assert_eq!(lines[1].spans.len(), 1, "+++ header should be single muted span");
}
#[test]
fn derive_pr_status_merged_pr_returns_merged() {
    let mut pr = test_pr();
    pr.state = "closed".into();
    pr.merged_at = Some("2024-01-02T00:00:00Z".into());
    assert_eq!(derive_pr_status(&pr), "merged");
}

#[test]
fn derive_pr_status_closed_without_merged_at_returns_closed() {
    let mut pr = test_pr();
    pr.state = "closed".into();
    pr.merged_at = None;
    assert_eq!(derive_pr_status(&pr), "closed");
}

#[test]
fn derive_pr_status_open_returns_open() {
    let pr = test_pr();
    assert_eq!(derive_pr_status(&pr), "open");
}


#[test]
fn can_reopen_closed_unmerged_pr() {
    let mut pr = test_pr();
    pr.state = "closed".into();
    pr.merged_at = None;
    assert!(can_reopen(&pr));
}

#[test]
fn cannot_reopen_merged_pr() {
    let mut pr = test_pr();
    pr.state = "closed".into();
    pr.merged_at = Some("2024-01-02T00:00:00Z".into());
    assert!(!can_reopen(&pr));
}

#[test]
fn can_merge_open_pr() {
    let mut pr = test_pr();
    pr.state = "open".into();
    assert!(can_merge(&pr));
}

#[test]
fn cannot_merge_closed_pr() {
    let mut pr = test_pr();
    pr.state = "closed".into();
    assert!(!can_merge(&pr));
}

#[test]
fn can_close_open_pr() {
    let mut pr = test_pr();
    pr.state = "open".into();
    assert!(can_close(&pr));
}

#[test]
fn cannot_close_closed_pr() {
    let mut pr = test_pr();
    pr.state = "closed".into();
    assert!(!can_close(&pr));
}
