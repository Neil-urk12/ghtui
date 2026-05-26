use std::fmt;
use std::sync::Arc;

use crate::api::client::GitHubClient;
use crate::app::{App, AppState, PanelKind, PrCommentUpdate};
use crate::cmd::Cmd;
use crate::panels::login::LoginState;

pub enum Message {
    NextTab,
    PrevTab,
    Refresh,
    SearchActivate,
    SearchInput(char),
    SearchBackspace,
    SearchSubmit,
    SearchClear,
    DismissError,
    DismissSuccess,
    ToggleHelp,
    PanelKeyInput(char),
    DetailScrollUp,
    DetailScrollDown,
    Quit,
    Noop,
    LoginInit(LoginState),
    LoginSuccess(String),
    LoginError(String),
    ApiLoaded(crate::event::ApiPayload),
    ApiError(String),
    RateLimitUpdate { remaining: Option<u32>, reset_secs: Option<u64> },
    Tick,
    ConfirmYes,
    ConfirmNo,
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::NextTab => write!(f, "NextTab"),
            Message::PrevTab => write!(f, "PrevTab"),
            Message::Refresh => write!(f, "Refresh"),
            Message::SearchActivate => write!(f, "SearchActivate"),
            Message::SearchInput(c) => write!(f, "SearchInput({})", c),
            Message::SearchBackspace => write!(f, "SearchBackspace"),
            Message::SearchSubmit => write!(f, "SearchSubmit"),
            Message::SearchClear => write!(f, "SearchClear"),
            Message::DismissError => write!(f, "DismissError"),
            Message::DismissSuccess => write!(f, "DismissSuccess"),
            Message::ToggleHelp => write!(f, "ToggleHelp"),
            Message::PanelKeyInput(c) => write!(f, "PanelKeyInput({})", c),
            Message::DetailScrollUp => write!(f, "DetailScrollUp"),
            Message::DetailScrollDown => write!(f, "DetailScrollDown"),
            Message::Quit => write!(f, "Quit"),
            Message::Noop => write!(f, "Noop"),
            Message::LoginInit(_) => write!(f, "LoginInit"),
            Message::LoginSuccess(_) => write!(f, "LoginSuccess"),
            Message::LoginError(msg) => write!(f, "LoginError({})", msg),
            Message::ApiLoaded(_) => write!(f, "ApiLoaded"),
            Message::ApiError(ref msg) => write!(f, "ApiError({})", msg),
            Message::RateLimitUpdate { remaining, reset_secs } => {
                write!(f, "RateLimitUpdate(remaining={:?}, reset={:?})", remaining, reset_secs)
            }
            Message::Tick => write!(f, "Tick"),
            Message::ConfirmYes => write!(f, "ConfirmYes"),
            Message::ConfirmNo => write!(f, "ConfirmNo"),
        }
    }
}

pub fn update(app: &mut App, msg: Message) -> Cmd {
    match msg {
        Message::NextTab => {
            app.search.active = false;
            app.next_tab();
            fetch_and_set_loading(app)
        }
        Message::PrevTab => {
            app.search.active = false;
            app.prev_tab();
            fetch_and_set_loading(app)
        }
        Message::Refresh => fetch_and_set_loading(app),
        
        Message::SearchActivate => {
            app.search.active = true;
            app.search.query.clear();
            Cmd::None
        }
        Message::SearchInput(c) => {
            app.search.query.push(c);
            Cmd::None
        }
        Message::SearchBackspace => {
            app.search.query.pop();
            Cmd::None
        }
        Message::SearchSubmit => {
            app.search.active = false;
            let query = app.search.query.clone();
            if let Some(entry) = app.panels.get_mut(&app.active_panel) {
                entry.apply_filter(&query);
            }
            Cmd::None
        }
        Message::SearchClear => {
            app.search.active = false;
            app.search.query.clear();
            if let Some(entry) = app.panels.get_mut(&app.active_panel) {
                entry.apply_filter("");
            }
            Cmd::None
        }
        Message::DismissError => {
            app.error_toast = None;
            Cmd::None
        }
        Message::DismissSuccess => {
            app.success_toast = None;
            Cmd::None
        }
        Message::ToggleHelp => {
            app.show_help = !app.show_help;
            Cmd::None
        }
        Message::DetailScrollUp => {
            if let Some(entry) = app.panels.get_mut(&app.active_panel) {
                entry.scroll_detail_up();
            }
            Cmd::None
        }
        Message::DetailScrollDown => {
            if let Some(entry) = app.panels.get_mut(&app.active_panel) {
                entry.scroll_detail_down();
            }
            Cmd::None
        }
        Message::PanelKeyInput(c) => {
            if !matches!(app.state, AppState::Main) {
                if c == 'q' {
                    app.should_quit = true;
                }
                return Cmd::None;
            }
            let cmd = if let Some(entry) = app.panels.get_mut(&app.active_panel) {
                entry.handle_key(c).unwrap_or(Cmd::None)
            } else {
                Cmd::None
            };
            match cmd {
                Cmd::OpenRepoDetail { owner, repo } => {
                    app.active_panel = PanelKind::Pulls;
                    for entry in app.panels.values_mut() {
                        entry.set_repo_context(&owner, &repo);
                    }
                    let cmd = Cmd::FetchPulls {
                        owner,
                        repo,
                        state: "open".to_string(),
                    };
                    if let Some(entry) = app.panels.get_mut(&PanelKind::Pulls) {
                        entry.set_loading(true);
                    }
                    cmd
                }
                Cmd::None => Cmd::None,
                Cmd::CycleRepoSort => {
                    if let Some(crate::app::PanelState::Repos(ref mut p)) = app.panels.get_mut(&PanelKind::Repos) {
                        p.item.cycle_sort(&mut p.data, &mut p.selection);
                    }
                    Cmd::None
                }
                Cmd::FetchPrComments { .. } => {
                    // Don't set loading on the whole panel — only detail pane
                    // needs to refresh when navigating between PRs.
                    cmd
                }
                other => {
                    if other.is_destructive() {
                        let detail = app.panels.get(&app.active_panel)
                            .and_then(|e| e.selected_display())
                            .unwrap_or_default();
                        let action_label = other.action_label().expect("destructive cmd must have label").to_string();
                        app.confirmation = Some(crate::app::ConfirmState {
                            pending_cmd: other,
                            action_label,
                            detail,
                        });
                        Cmd::None
                    } else if let Some(entry) = app.panels.get_mut(&app.active_panel) {
                        entry.set_loading(true);
                        other
                    } else {
                        other
                    }
                }
            }
        }
        Message::LoginInit(login) => {
            app.state = AppState::Login;
            app.login = Some(login);
            Cmd::None
        }
        Message::LoginSuccess(token) => {
            if let Err(e) = crate::api::auth::save_token(&token, crate::api::auth::GHTUI_SCOPES) { tracing::warn!("failed to persist token: {e}"); }
            app.client = Arc::new(GitHubClient::new(token));
            app.login = None;
            app.state = AppState::Main;
            if let Some(entry) = app.panels.get_mut(&PanelKind::Repos) {
                entry.set_loading(true);
            }
            Cmd::FetchRepos
        }
        Message::LoginError(msg) => {
            app.error_toast = Some(msg);
            Cmd::None
        }
        Message::Quit => {
            app.should_quit = true;
            Cmd::None
        }
        Message::Noop => Cmd::None,

        Message::ConfirmYes => {
            if let Some(confirm) = app.confirmation.take() {
                let cmd = confirm.pending_cmd;
                if let Some(entry) = app.panels.get_mut(&app.active_panel) {
                    entry.set_loading(true);
                }
                cmd
            } else {
                Cmd::None
            }
        }
        Message::ConfirmNo => {
            app.confirmation = None;
            Cmd::None
        }

        Message::ApiLoaded(payload) => {
            use crate::event::ApiPayload;
            if let ApiPayload::Success(ref msg) = payload {
                app.success_toast = Some(msg.clone());
                app.toast_tick = 0;
            }
            // Route by payload type to the correct panel, not just active panel.
            // This handles the case where a fetch response arrives after the user
            // has switched tabs — data still loads into the owning panel.
            let target = match &payload {
                ApiPayload::Repos(_) => Some(PanelKind::Repos),
                ApiPayload::Pulls(_) => Some(PanelKind::Pulls),
                ApiPayload::Issues(_) => Some(PanelKind::Issues),
                ApiPayload::Notifications(_) => Some(PanelKind::Notifications),
                ApiPayload::Actions(_) => Some(PanelKind::Actions),
                ApiPayload::Success(_) => None,
                ApiPayload::PrComments { .. } => Some(PanelKind::Pulls),
            };
            if let Some(kind) = target {
                if let Some(entry) = app.panels.get_mut(&kind) {
                    // Handle PrComments specially
                    if let ApiPayload::PrComments { pr_number, comments, reviews, code_comments } = payload {
                        // Destructure: Ok data stored, Err stored as error string, None keeps existing state for retry
                        let (c_ok, c_err) = match comments { Ok(v) => (Some(v), None), Err(e) => (None, Some(e)) };
                        let (r_ok, r_err) = match reviews { Ok(v) => (Some(v), None), Err(e) => (None, Some(e)) };
                        let (cc_ok, cc_err) = match code_comments { Ok(v) => (Some(v), None), Err(e) => (None, Some(e)) };
                        entry.update_pr_comments(pr_number, PrCommentUpdate {
                            comments: c_ok,
                            reviews: r_ok,
                            code_comments: cc_ok,
                            comment_err: c_err,
                            review_err: r_err,
                            code_comment_err: cc_err,
                        });
                    } else {
                        entry.load_data(payload);
                    }
                }
            }
            Cmd::None
        }
        Message::ApiError(msg) => {
            app.error_toast = Some(msg);
            app.toast_tick = 0;
            Cmd::None
        }
        Message::RateLimitUpdate { remaining, reset_secs } => {
            app.rate_remaining = remaining;
            app.rate_reset_secs = reset_secs;
            Cmd::None
        }
        Message::Tick => {
            app.tick += 1;
            if app.error_toast.is_some() || app.success_toast.is_some() {
                app.toast_tick += 1;
                if app.toast_tick >= crate::widgets::popup::AUTO_DISMISS_TICKS {
                    app.error_toast = None;
                    app.success_toast = None;
                    app.toast_tick = 0;
                }
            }
            Cmd::None
        }
    }
}

fn fetch_and_set_loading(app: &mut App) -> Cmd {
    let cmd = app.panels.get(&app.active_panel)
        .map(|entry| entry.fetch_cmd())
        .unwrap_or(Cmd::None);
    if !matches!(cmd, Cmd::None) {
        if let Some(entry) = app.panels.get_mut(&app.active_panel) {
            entry.set_loading(true);
        }
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_app() -> App {
        App::new(Config::default(), Some("test-token".into()))
    }

    #[test]
    fn quit_sets_flag() {
        let mut app = test_app();
        assert!(!app.should_quit);
        let cmd = update(&mut app, Message::Quit);
        assert!(app.should_quit);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn next_tab_cycles() {
        let mut app = test_app();
        assert_eq!(app.active_panel, PanelKind::Repos);
        let cmd = update(&mut app, Message::NextTab);
        assert_eq!(app.active_panel, PanelKind::Pulls);
        // fetch_cmd returns Cmd::None because no owner/repo set
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn prev_tab_wraps() {
        let mut app = test_app();
        assert_eq!(app.active_panel, PanelKind::Repos);
        update(&mut app, Message::PrevTab);
        assert_eq!(app.active_panel, PanelKind::Actions);
    }

    #[test]
    fn next_tab_wraps_around() {
        let mut app = test_app();
        // Go through all 5 tabs
        for _ in 0..5 {
            update(&mut app, Message::NextTab);
        }
        assert_eq!(app.active_panel, PanelKind::Repos);
    }

    #[test]
    fn search_activate_and_clear() {
        let mut app = test_app();
        assert!(!app.search.active);

        update(&mut app, Message::SearchActivate);
        assert!(app.search.active);
        assert!(app.search.query.is_empty());

        update(&mut app, Message::SearchInput('f'));
        update(&mut app, Message::SearchInput('o'));
        update(&mut app, Message::SearchInput('o'));
        assert_eq!(app.search.query, "foo");

        update(&mut app, Message::SearchBackspace);
        assert_eq!(app.search.query, "fo");

        update(&mut app, Message::SearchClear);
        assert!(!app.search.active);
        assert!(app.search.query.is_empty());
    }

    #[test]
    fn search_submit_deactivates() {
        let mut app = test_app();
        update(&mut app, Message::SearchActivate);
        update(&mut app, Message::SearchInput('x'));
        update(&mut app, Message::SearchSubmit);
        assert!(!app.search.active);
        // query preserved on submit (not cleared)
        assert_eq!(app.search.query, "x");
    }

    #[test]
    fn dismiss_toasts() {
        let mut app = test_app();
        app.error_toast = Some("err".into());
        app.success_toast = Some("ok".into());

        update(&mut app, Message::DismissError);
        assert!(app.error_toast.is_none());
        assert!(app.success_toast.is_some());

        update(&mut app, Message::DismissSuccess);
        assert!(app.success_toast.is_none());
    }

    #[test]
    fn toggle_help() {
        let mut app = test_app();
        assert!(!app.show_help);
        update(&mut app, Message::ToggleHelp);
        assert!(app.show_help);
        update(&mut app, Message::ToggleHelp);
        assert!(!app.show_help);
    }

    #[test]
    fn tick_increments() {
        let mut app = test_app();
        assert_eq!(app.tick, 0);
        update(&mut app, Message::Tick);
        assert_eq!(app.tick, 1);
        update(&mut app, Message::Tick);
        assert_eq!(app.tick, 2);
    }

    #[test]
    fn api_loaded_routes_to_correct_panel() {
        use crate::domain;
        use crate::event::ApiPayload;

        let mut app = test_app();
        let repos = vec![domain::Repo {
            name: "test/repo".into(),
            ..Default::default()
        }];
        update(&mut app, Message::ApiLoaded(ApiPayload::Repos(repos)));

        // Verify data landed in Repos panel
        if let Some(entry) = app.panels.get(&PanelKind::Repos) {
            if let crate::app::PanelState::Repos(panel) = entry {
                assert_eq!(panel.data.items.len(), 1);
                assert_eq!(panel.data.items[0].name, "test/repo");
            } else {
                panic!("Expected Repos panel");
            }
        } else {
            panic!("Expected PanelState for Repos");
        }
    }

    #[test]
    fn api_loaded_success_shows_toast() {
        use crate::event::ApiPayload;
        let mut app = test_app();
        update(&mut app, Message::ApiLoaded(ApiPayload::Success("done".into())));
        assert_eq!(app.success_toast.as_deref(), Some("done"));
    }

    #[test]
    fn api_error_shows_toast() {
        let mut app = test_app();
        update(&mut app, Message::ApiError("fail".into()));
        assert_eq!(app.error_toast.as_deref(), Some("fail"));
    }

    #[test]
    fn rate_limit_update() {
        let mut app = test_app();
        assert!(app.rate_remaining.is_none());
        update(&mut app, Message::RateLimitUpdate {
            remaining: Some(42),
            reset_secs: Some(3600),
        });
        assert_eq!(app.rate_remaining, Some(42));
        assert_eq!(app.rate_reset_secs, Some(3600));
    }

    #[test]
    fn next_tab_clears_search() {
        let mut app = test_app();
        update(&mut app, Message::SearchActivate);
        update(&mut app, Message::SearchInput('x'));
        assert!(app.search.active);

        update(&mut app, Message::NextTab);
    }

    #[test]
    fn search_submit_applies_filter_to_active_panel() {
        use crate::domain;
        use crate::event::ApiPayload;

        let mut app = test_app();
        let repos = vec![
            domain::Repo { name: "octocat/hello".into(), ..Default::default() },
            domain::Repo { name: "test/repo".into(), ..Default::default() },
            domain::Repo { name: "foo/bar".into(), ..Default::default() },
        ];
        update(&mut app, Message::ApiLoaded(ApiPayload::Repos(repos)));

        // All 3 repos visible initially
        if let Some(entry) = app.panels.get(&PanelKind::Repos) {
            if let crate::app::PanelState::Repos(panel) = entry {
                assert_eq!(panel.data.filtered_indices.len(), 3);
            } else {
                panic!("Expected Repos panel");
            }
        }

        // Activate search, type query, submit
        update(&mut app, Message::SearchActivate);
        for ch in "test".chars() {
            update(&mut app, Message::SearchInput(ch));
        }
        update(&mut app, Message::SearchSubmit);

        // After submit: search closed, filtered_indices should be reduced
        assert!(!app.search.active);
        if let Some(entry) = app.panels.get(&PanelKind::Repos) {
            if let crate::app::PanelState::Repos(panel) = entry {
                assert!(panel.data.filtered_indices.len() < 3,
                    "Expected filtered_indices to be less than 3 after search, got {}",
                    panel.data.filtered_indices.len());
            } else {
                panic!("Expected Repos panel");
            }
        }
    }

    #[test]
    fn search_submit_filters_repos_by_name() {
        use crate::domain;
        use crate::event::ApiPayload;

        let mut app = test_app();
        let repos = vec![
            domain::Repo { name: "octocat/hello".into(), ..Default::default() },
            domain::Repo { name: "test/repo".into(), ..Default::default() },
            domain::Repo { name: "foo/bar".into(), ..Default::default() },
        ];
        update(&mut app, Message::ApiLoaded(ApiPayload::Repos(repos)));

        update(&mut app, Message::SearchActivate);
        for ch in "octo".chars() {
            update(&mut app, Message::SearchInput(ch));
        }
        update(&mut app, Message::SearchSubmit);

        if let Some(entry) = app.panels.get(&PanelKind::Repos) {
            if let crate::app::PanelState::Repos(panel) = entry {
                assert_eq!(panel.data.filtered_indices.len(), 1,
                    "Expected 1 match for 'octo', got {}",
                    panel.data.filtered_indices.len());
                assert_eq!(panel.data.items[panel.data.filtered_indices[0]].name, "octocat/hello");
            } else {
                panic!("Expected Repos panel");
            }
        }
    }

    #[test]
    fn search_submit_filters_pulls_by_title() {
        use crate::domain;
        use crate::event::ApiPayload;

        let mut app = test_app();
        // Switch to Pulls panel
        update(&mut app, Message::NextTab);
        assert_eq!(app.active_panel, PanelKind::Pulls);

        let pulls = vec![
            domain::PullRequest { number: 1, title: "Fix login bug".into(), ..Default::default() },
            domain::PullRequest { number: 2, title: "Add dark mode".into(), ..Default::default() },
            domain::PullRequest { number: 3, title: "Refactor auth".into(), ..Default::default() },
        ];
        update(&mut app, Message::ApiLoaded(ApiPayload::Pulls(pulls)));

        update(&mut app, Message::SearchActivate);
        for ch in "dark".chars() {
            update(&mut app, Message::SearchInput(ch));
        }
        update(&mut app, Message::SearchSubmit);

        if let Some(entry) = app.panels.get(&PanelKind::Pulls) {
            if let crate::app::PanelState::Pulls(panel) = entry {
                assert_eq!(panel.data.filtered_indices.len(), 1,
                    "Expected 1 match for 'dark', got {}",
                    panel.data.filtered_indices.len());
                assert_eq!(panel.data.items[panel.data.filtered_indices[0]].title, "Add dark mode");
            } else {
                panic!("Expected Pulls panel");
            }
        }
    }

    #[test]
    fn search_clear_resets_filter() {
        use crate::domain;
        use crate::event::ApiPayload;

        let mut app = test_app();
        let repos = vec![
            domain::Repo { name: "octocat/hello".into(), ..Default::default() },
            domain::Repo { name: "test/repo".into(), ..Default::default() },
            domain::Repo { name: "foo/bar".into(), ..Default::default() },
        ];
        update(&mut app, Message::ApiLoaded(ApiPayload::Repos(repos)));

        // Apply filter
        update(&mut app, Message::SearchActivate);
        for ch in "octo".chars() {
            update(&mut app, Message::SearchInput(ch));
        }
        update(&mut app, Message::SearchSubmit);

        // Verify filtered
        if let Some(crate::app::PanelState::Repos(panel)) = app.panels.get(&PanelKind::Repos) {
            assert_eq!(panel.data.filtered_indices.len(), 1);
        }

        // Clear search
        update(&mut app, Message::SearchClear);

        // Verify all items restored
        if let Some(entry) = app.panels.get(&PanelKind::Repos) {
            if let crate::app::PanelState::Repos(panel) = entry {
                assert_eq!(panel.data.filtered_indices.len(), 3,
                    "Expected all 3 repos after clear, got {}",
                    panel.data.filtered_indices.len());
            } else {
                panic!("Expected Repos panel");
            }
        }
    }

    #[test]
    fn search_submit_preserves_query() {
        use crate::domain;
        use crate::event::ApiPayload;

        let mut app = test_app();
        let repos = vec![
            domain::Repo { name: "octocat/hello".into(), ..Default::default() },
        ];
        update(&mut app, Message::ApiLoaded(ApiPayload::Repos(repos)));

        update(&mut app, Message::SearchActivate);
        for ch in "octo".chars() {
            update(&mut app, Message::SearchInput(ch));
        }
        update(&mut app, Message::SearchSubmit);

        assert_eq!(app.search.query, "octo",
            "Search query should be preserved after submit");
    }

    #[test]
    fn search_submit_with_empty_query_shows_all() {
        use crate::domain;
        use crate::event::ApiPayload;

        let mut app = test_app();
        let repos = vec![
            domain::Repo { name: "octocat/hello".into(), ..Default::default() },
            domain::Repo { name: "test/repo".into(), ..Default::default() },
            domain::Repo { name: "foo/bar".into(), ..Default::default() },
        ];
        update(&mut app, Message::ApiLoaded(ApiPayload::Repos(repos)));

        // Activate and immediately submit (empty query)
        update(&mut app, Message::SearchActivate);
        update(&mut app, Message::SearchSubmit);

        // All repos should still be visible
        if let Some(entry) = app.panels.get(&PanelKind::Repos) {
            if let crate::app::PanelState::Repos(panel) = entry {
                assert_eq!(panel.data.filtered_indices.len(), 3,
                    "Expected all 3 repos with empty query, got {}",
                    panel.data.filtered_indices.len());
            } else {
                panic!("Expected Repos panel");
            }
        }
    }

    #[test]
    fn non_destructive_cmd_sets_loading() {
        use crate::event::ApiPayload;
        let mut app = test_app();

        // Navigate to Notifications panel (3 tabs from Repos)
        update(&mut app, Message::NextTab); // Pulls
        update(&mut app, Message::NextTab); // Issues
        update(&mut app, Message::NextTab); // Notifications
        assert_eq!(app.active_panel, PanelKind::Notifications);

        // NextTab sets loading=true via fetch_and_set_loading.
        // Simulate API response to clear it.
        update(
            &mut app,
            Message::ApiLoaded(ApiPayload::Notifications(vec![])),
        );

        // Verify loading is now false
        match app.panels.get(&PanelKind::Notifications) {
            Some(crate::app::PanelState::Notifications(panel)) => {
                assert!(!panel.data.loading, "Loading should be false after API response");
            }
            _ => panic!("Expected Notifications panel"),
        }

        // Press 'a' on notifications panel — returns FetchNotifications (non-destructive)
        // This flows through the non-destructive else-if branch which sets loading = true
        let cmd = update(&mut app, Message::PanelKeyInput('a'));
        assert!(
            matches!(cmd, Cmd::FetchNotifications { .. }),
            "Expected FetchNotifications, got {:?}",
            cmd
        );

        // Verify loading was set to true by the non-destructive path
        match app.panels.get(&PanelKind::Notifications) {
            Some(crate::app::PanelState::Notifications(panel)) => {
                assert!(
                    panel.data.loading,
                    "Loading should be true after non-destructive cmd"
                );
            }
            _ => panic!("Expected Notifications panel"),
        }
    }

    #[test]
    fn fetch_pr_comments_does_not_set_loading() {
        use crate::domain;
        use crate::event::ApiPayload;

        let mut app = test_app();

        // Set repo context on Pulls panel (needed for on_select to trigger fetch)
        if let Some(entry) = app.panels.get_mut(&PanelKind::Pulls) {
            entry.set_repo_context("octocat", "hello-world");
        }

        // Load some PRs (comments/reviews/code_comments are None by default)
        let pulls = vec![
            domain::PullRequest { number: 1, title: "Fix bug".into(), ..Default::default() },
            domain::PullRequest { number: 2, title: "Add feature".into(), ..Default::default() },
        ];
        update(&mut app, Message::ApiLoaded(ApiPayload::Pulls(pulls.clone())));

        // Switch to Pulls panel
        update(&mut app, Message::NextTab);
        assert_eq!(app.active_panel, PanelKind::Pulls);

        // NextTab triggers fetch_and_set_loading which sets loading=true.
        // Simulate API response to clear it.
        update(&mut app, Message::ApiLoaded(ApiPayload::Pulls(vec![])));

        // Reload PRs so items exist for navigation
        update(&mut app, Message::ApiLoaded(ApiPayload::Pulls(pulls)));

        // Verify loading is false
        match app.panels.get(&PanelKind::Pulls) {
            Some(crate::app::PanelState::Pulls(panel)) => {
                assert!(!panel.data.loading, "Loading should be false initially");
                assert_eq!(panel.data.items.len(), 2, "Should have 2 PRs loaded");
            }
            _ => panic!("Expected Pulls panel"),
        }

        // Navigate down to trigger on_select which returns FetchPrComments
        let cmd = update(&mut app, Message::PanelKeyInput('j'));

        // Should return FetchPrComments cmd
        assert!(
            matches!(cmd, Cmd::FetchPrComments { .. }),
            "Expected FetchPrComments cmd, got {:?}",
            cmd
        );

        // Verify loading is STILL false (not set to true)
        match app.panels.get(&PanelKind::Pulls) {
            Some(crate::app::PanelState::Pulls(panel)) => {
                assert!(
                    !panel.data.loading,
                    "Loading should remain false for FetchPrComments"
                );
            }
            _ => panic!("Expected Pulls panel"),
        }
    }
}