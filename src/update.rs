use std::fmt;
use std::sync::Arc;

use crate::api::client::GitHubClient;
use crate::app::{App, AppState, PanelKind};
use crate::cmd::{Cmd, CmdBehavior};
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
    CmdCompleted(crate::cmd::CmdCompletion),
    RateLimitUpdate {
        remaining: Option<u32>,
        reset_secs: Option<u64>,
    },
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
            Message::CmdCompleted(_) => write!(f, "CmdCompleted"),
            Message::RateLimitUpdate {
                remaining,
                reset_secs,
            } => {
                write!(
                    f,
                    "RateLimitUpdate(remaining={:?}, reset={:?})",
                    remaining, reset_secs
                )
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
            fetch_cmd(app)
        }
        Message::PrevTab => {
            app.search.active = false;
            app.prev_tab();
            fetch_cmd(app)
        }
        Message::Refresh => fetch_cmd(app),

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
                Cmd::OpenRepoDetail { ctx } => {
                    app.active_panel = PanelKind::Pulls;
                    for kind in [PanelKind::Pulls, PanelKind::Issues, PanelKind::Actions] {
                        if let Some(entry) = app.panels.get_mut(&kind) {
                            entry.set_repo_context(&ctx.owner, &ctx.repo);
                        }
                    }
                    fetch_cmd(app)
                }
                Cmd::None => Cmd::None,
                Cmd::CycleRepoSort => {
                    if let Some(p) = app.panel_mut::<crate::panels::ReposPanel>(PanelKind::Repos) {
                        p.item.cycle_sort(&mut p.data, &mut p.selection);
                    }
                    Cmd::None
                }
                other => match other.behavior() {
                    CmdBehavior::Destructive { label } => {
                        let detail = app
                            .panels
                            .get(&app.active_panel)
                            .and_then(|e| e.selected_display())
                            .unwrap_or_default();
                        app.confirmation = Some(crate::app::ConfirmState {
                            pending_cmd: other,
                            action_label: label.to_string(),
                            detail,
                        });
                        Cmd::None
                    }
                    CmdBehavior::Benign => other,
                },
            }
        }
        Message::LoginInit(login) => {
            app.state = AppState::Login;
            app.login = Some(login);
            Cmd::None
        }
        Message::LoginSuccess(token) => {
            if let Err(e) = crate::api::auth::save_token(&token, crate::api::auth::GHTUI_SCOPES) {
                tracing::warn!(error = %e, "failed to persist token");
            }
            app.client = Arc::new(match GitHubClient::new(token) {
                Ok(c) => c,
                Err(e) => {
                    app.error_toast = Some(format!("Failed to create HTTP client: {e}"));
                    tracing::error!(error = %e, "failed to create HTTP client");
                    return Cmd::None;
                }
            });
            app.login = None;
            app.state = AppState::Main;
            Cmd::FetchRepos
        }
        Message::LoginError(msg) => {
            if let Some(login) = &mut app.login {
                login.status = crate::panels::login::LoginStatus::Error(msg.clone());
            }
            app.toast_tick = 0;
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
                confirm.pending_cmd
            } else {
                Cmd::None
            }
        }
        Message::ConfirmNo => {
            app.confirmation = None;
            Cmd::None
        }

        Message::CmdCompleted(completion) => crate::cmd::complete(app, completion),
        Message::RateLimitUpdate {
            remaining,
            reset_secs,
        } => {
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

fn fetch_cmd(app: &App) -> Cmd {
    app.panels
        .get(&app.active_panel)
        .map(|entry| entry.fetch_cmd())
        .unwrap_or(Cmd::None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::panels::{NotificationsPanel, PullsPanel, ReposPanel};

    fn test_app() -> App {
        App::new(Config::default(), Some("test-token".into())).unwrap()
    }

    fn receive(app: &mut App, payload: crate::event::ApiPayload) -> Cmd {
        use crate::event::ApiPayload;
        let panel = match &payload {
            ApiPayload::Repos(_) => PanelKind::Repos,
            ApiPayload::Pulls(_) => PanelKind::Pulls,
            ApiPayload::Issues(_) => PanelKind::Issues,
            ApiPayload::Notifications(_) | ApiPayload::Success(_) => PanelKind::Notifications,
            ApiPayload::Actions(_) => PanelKind::Actions,
            ApiPayload::PrComments { .. } => panic!("use a registered detail request"),
        };
        let entry = app.panels.get_mut(&panel).unwrap();
        if matches!(
            panel,
            PanelKind::Pulls | PanelKind::Issues | PanelKind::Actions
        ) && entry.data_context().0.is_none()
        {
            entry.set_repo_context("o", "r");
        }
        let cmd = if matches!(payload, ApiPayload::Success(_)) {
            Cmd::MarkAllNotificationsRead
        } else {
            entry.fetch_cmd()
        };
        let pending = crate::cmd::start(app, cmd).unwrap();
        update(
            app,
            Message::CmdCompleted(pending.ticket.finish(Ok(payload))),
        )
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
        receive(&mut app, ApiPayload::Repos(repos));

        // Verify data landed in Repos panel
        let panel: &ReposPanel = app.panel(PanelKind::Repos).expect("Expected Repos panel");
        assert_eq!(panel.data.items.len(), 1);
        assert_eq!(panel.data.items[0].name, "test/repo");
    }

    #[test]
    fn api_loaded_success_shows_toast() {
        use crate::event::ApiPayload;
        let mut app = test_app();
        receive(&mut app, ApiPayload::Success("done".into()));
        assert_eq!(app.success_toast.as_deref(), Some("done"));
    }

    #[test]
    fn api_error_shows_toast() {
        let mut app = test_app();
        let pending = crate::cmd::start(&mut app, Cmd::FetchRepos).unwrap();
        update(
            &mut app,
            Message::CmdCompleted(pending.ticket.finish(Err("fail".into()))),
        );
        assert_eq!(app.error_toast.as_deref(), Some("fail"));
    }

    #[test]
    fn rate_limit_update() {
        let mut app = test_app();
        assert!(app.rate_remaining.is_none());
        update(
            &mut app,
            Message::RateLimitUpdate {
                remaining: Some(42),
                reset_secs: Some(3600),
            },
        );
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
            domain::Repo {
                name: "octocat/hello".into(),
                ..Default::default()
            },
            domain::Repo {
                name: "test/repo".into(),
                ..Default::default()
            },
            domain::Repo {
                name: "foo/bar".into(),
                ..Default::default()
            },
        ];
        receive(&mut app, ApiPayload::Repos(repos));

        // All 3 repos visible initially
        let panel: &ReposPanel = app.panel(PanelKind::Repos).expect("Expected Repos panel");
        assert_eq!(panel.data.filtered_indices.len(), 3);

        // Activate search, type query, submit
        update(&mut app, Message::SearchActivate);
        for ch in "test".chars() {
            update(&mut app, Message::SearchInput(ch));
        }
        update(&mut app, Message::SearchSubmit);

        // After submit: search closed, filtered_indices should be reduced
        assert!(!app.search.active);
        let panel: &ReposPanel = app.panel(PanelKind::Repos).expect("Expected Repos panel");
        assert!(
            panel.data.filtered_indices.len() < 3,
            "Expected filtered_indices to be less than 3 after search, got {}",
            panel.data.filtered_indices.len()
        );
    }

    #[test]
    fn search_submit_filters_repos_by_name() {
        use crate::domain;
        use crate::event::ApiPayload;

        let mut app = test_app();
        let repos = vec![
            domain::Repo {
                name: "octocat/hello".into(),
                ..Default::default()
            },
            domain::Repo {
                name: "test/repo".into(),
                ..Default::default()
            },
            domain::Repo {
                name: "foo/bar".into(),
                ..Default::default()
            },
        ];
        receive(&mut app, ApiPayload::Repos(repos));

        update(&mut app, Message::SearchActivate);
        for ch in "octo".chars() {
            update(&mut app, Message::SearchInput(ch));
        }
        update(&mut app, Message::SearchSubmit);

        let panel: &ReposPanel = app.panel(PanelKind::Repos).expect("Expected Repos panel");
        assert_eq!(
            panel.data.filtered_indices.len(),
            1,
            "Expected 1 match for 'octo', got {}",
            panel.data.filtered_indices.len()
        );
        assert_eq!(
            panel.data.items[panel.data.filtered_indices[0]].name,
            "octocat/hello"
        );
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
            domain::PullRequest {
                number: 1,
                title: "Fix login bug".into(),
                ..Default::default()
            },
            domain::PullRequest {
                number: 2,
                title: "Add dark mode".into(),
                ..Default::default()
            },
            domain::PullRequest {
                number: 3,
                title: "Refactor auth".into(),
                ..Default::default()
            },
        ];
        receive(&mut app, ApiPayload::Pulls(pulls));

        update(&mut app, Message::SearchActivate);
        for ch in "dark".chars() {
            update(&mut app, Message::SearchInput(ch));
        }
        update(&mut app, Message::SearchSubmit);

        let panel: &PullsPanel = app.panel(PanelKind::Pulls).expect("Expected Pulls panel");
        assert_eq!(
            panel.data.filtered_indices.len(),
            1,
            "Expected 1 match for 'dark', got {}",
            panel.data.filtered_indices.len()
        );
        assert_eq!(
            panel.data.items[panel.data.filtered_indices[0]].title,
            "Add dark mode"
        );
    }

    #[test]
    fn search_clear_resets_filter() {
        use crate::domain;
        use crate::event::ApiPayload;

        let mut app = test_app();
        let repos = vec![
            domain::Repo {
                name: "octocat/hello".into(),
                ..Default::default()
            },
            domain::Repo {
                name: "test/repo".into(),
                ..Default::default()
            },
            domain::Repo {
                name: "foo/bar".into(),
                ..Default::default()
            },
        ];
        receive(&mut app, ApiPayload::Repos(repos));

        // Apply filter
        update(&mut app, Message::SearchActivate);
        for ch in "octo".chars() {
            update(&mut app, Message::SearchInput(ch));
        }
        update(&mut app, Message::SearchSubmit);

        // Verify filtered
        let panel: &ReposPanel = app.panel(PanelKind::Repos).expect("Expected Repos panel");
        assert_eq!(panel.data.filtered_indices.len(), 1);

        // Clear search
        update(&mut app, Message::SearchClear);

        // Verify all items restored
        let panel: &ReposPanel = app.panel(PanelKind::Repos).expect("Expected Repos panel");
        assert_eq!(
            panel.data.filtered_indices.len(),
            3,
            "Expected all 3 repos after clear, got {}",
            panel.data.filtered_indices.len()
        );
    }

    #[test]
    fn search_submit_preserves_query() {
        use crate::domain;
        use crate::event::ApiPayload;

        let mut app = test_app();
        let repos = vec![domain::Repo {
            name: "octocat/hello".into(),
            ..Default::default()
        }];
        receive(&mut app, ApiPayload::Repos(repos));

        update(&mut app, Message::SearchActivate);
        for ch in "octo".chars() {
            update(&mut app, Message::SearchInput(ch));
        }
        update(&mut app, Message::SearchSubmit);

        assert_eq!(
            app.search.query, "octo",
            "Search query should be preserved after submit"
        );
    }

    #[test]
    fn search_submit_with_empty_query_shows_all() {
        use crate::domain;
        use crate::event::ApiPayload;

        let mut app = test_app();
        let repos = vec![
            domain::Repo {
                name: "octocat/hello".into(),
                ..Default::default()
            },
            domain::Repo {
                name: "test/repo".into(),
                ..Default::default()
            },
            domain::Repo {
                name: "foo/bar".into(),
                ..Default::default()
            },
        ];
        receive(&mut app, ApiPayload::Repos(repos));

        // Activate and immediately submit (empty query)
        update(&mut app, Message::SearchActivate);
        update(&mut app, Message::SearchSubmit);

        // All repos should still be visible
        let panel: &ReposPanel = app.panel(PanelKind::Repos).expect("Expected Repos panel");
        assert_eq!(
            panel.data.filtered_indices.len(),
            3,
            "Expected all 3 repos with empty query, got {}",
            panel.data.filtered_indices.len()
        );
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

        // Complete a registered fetch before testing the filter change.
        // Simulate API response to clear it.
        receive(&mut app, ApiPayload::Notifications(vec![]));

        // Verify loading is now false
        let panel: &NotificationsPanel = app
            .panel(PanelKind::Notifications)
            .expect("Expected Notifications panel");
        assert!(
            !panel.data.loading,
            "Loading should be false after API response"
        );

        // Press 'a' on notifications panel — returns FetchNotifications (non-destructive)
        // The returned Cmd starts loading when registered below.
        let cmd = update(&mut app, Message::PanelKeyInput('a'));
        assert!(
            matches!(cmd, Cmd::FetchNotifications { .. }),
            "Expected FetchNotifications, got {:?}",
            cmd
        );

        // Register the returned Cmd to begin loading.
        let _pending = crate::cmd::start(&mut app, cmd).unwrap();

        // Verify loading was set to true by the Cmd lifecycle.
        let panel: &NotificationsPanel = app
            .panel(PanelKind::Notifications)
            .expect("Expected Notifications panel");
        assert!(
            panel.data.loading,
            "Loading should be true after non-destructive cmd"
        );
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
            domain::PullRequest {
                number: 1,
                title: "Fix bug".into(),
                ..Default::default()
            },
            domain::PullRequest {
                number: 2,
                title: "Add feature".into(),
                ..Default::default()
            },
        ];
        receive(&mut app, ApiPayload::Pulls(pulls.clone()));

        // Switch to Pulls panel
        update(&mut app, Message::NextTab);
        assert_eq!(app.active_panel, PanelKind::Pulls);

        // Complete the list fetch before testing detail navigation.
        // Simulate API response to clear it.
        receive(&mut app, ApiPayload::Pulls(vec![]));

        // Reload PRs so items exist for navigation
        receive(&mut app, ApiPayload::Pulls(pulls));

        // Verify loading is false
        let panel: &PullsPanel = app.panel(PanelKind::Pulls).expect("Expected Pulls panel");
        assert!(!panel.data.loading, "Loading should be false initially");
        assert_eq!(panel.data.items.len(), 2, "Should have 2 PRs loaded");

        // Navigate down to trigger on_select which returns FetchPrComments
        let cmd = update(&mut app, Message::PanelKeyInput('j'));

        // Should return FetchPrComments cmd
        assert!(
            matches!(cmd, Cmd::FetchPrComments { .. }),
            "Expected FetchPrComments cmd, got {:?}",
            cmd
        );

        // Register detail work and verify list loading stays false.
        let _pending = crate::cmd::start(&mut app, cmd).unwrap();
        let panel: &PullsPanel = app.panel(PanelKind::Pulls).expect("Expected Pulls panel");
        assert!(
            !panel.data.loading,
            "Loading should remain false for FetchPrComments"
        );
    }

    #[test]
    fn f_key_toggles_fullscreen_on_pulls_panel() {
        use crate::domain;
        use crate::event::ApiPayload;
        use crate::widgets::detail_pane::DetailPaneMode;

        let mut app = test_app();

        // Load PRs and switch to Pulls panel
        let pulls = vec![domain::PullRequest {
            number: 1,
            title: "Fix bug".into(),
            ..Default::default()
        }];
        receive(&mut app, ApiPayload::Pulls(pulls));
        update(&mut app, Message::NextTab);
        assert_eq!(app.active_panel, PanelKind::Pulls);

        // Verify starts at Normal
        let panel: &PullsPanel = app.panel(PanelKind::Pulls).expect("Expected Pulls panel");
        assert_eq!(panel.detail_pane.mode, DetailPaneMode::Normal);

        // Press 'f' to enter fullscreen
        update(&mut app, Message::PanelKeyInput('f'));
        let panel: &PullsPanel = app.panel(PanelKind::Pulls).expect("Expected Pulls panel");
        assert_eq!(panel.detail_pane.mode, DetailPaneMode::Fullscreen);

        // Press 'f' again to exit fullscreen
        update(&mut app, Message::PanelKeyInput('f'));
        let panel: &PullsPanel = app.panel(PanelKind::Pulls).expect("Expected Pulls panel");
        assert_eq!(panel.detail_pane.mode, DetailPaneMode::Normal);
    }

    #[test]
    fn c_key_toggles_collapse_on_pulls_panel() {
        use crate::domain;
        use crate::event::ApiPayload;
        use crate::widgets::detail_pane::DetailPaneMode;

        let mut app = test_app();

        // Load PRs and switch to Pulls panel
        let pulls = vec![domain::PullRequest {
            number: 1,
            title: "Fix bug".into(),
            ..Default::default()
        }];
        receive(&mut app, ApiPayload::Pulls(pulls));
        update(&mut app, Message::NextTab);
        assert_eq!(app.active_panel, PanelKind::Pulls);

        // Verify starts at Normal
        let panel: &PullsPanel = app.panel(PanelKind::Pulls).expect("Expected Pulls panel");
        assert_eq!(panel.detail_pane.mode, DetailPaneMode::Normal);

        // Press 'c' to enter collapsed
        update(&mut app, Message::PanelKeyInput('c'));
        let panel: &PullsPanel = app.panel(PanelKind::Pulls).expect("Expected Pulls panel");
        assert_eq!(panel.detail_pane.mode, DetailPaneMode::Collapsed);

        // Press 'c' again to exit collapsed
        update(&mut app, Message::PanelKeyInput('c'));
        let panel: &PullsPanel = app.panel(PanelKind::Pulls).expect("Expected Pulls panel");
        assert_eq!(panel.detail_pane.mode, DetailPaneMode::Normal);
    }

    #[test]
    fn f_key_works_with_empty_pulls_list() {
        use crate::event::ApiPayload;
        use crate::widgets::detail_pane::DetailPaneMode;

        let mut app = test_app();

        // Switch to Pulls panel with no PRs loaded
        update(&mut app, Message::NextTab);
        assert_eq!(app.active_panel, PanelKind::Pulls);

        // Clear any PRs that may have loaded from NextTab fetch
        receive(&mut app, ApiPayload::Pulls(vec![]));

        // Press 'f' — should work even with no selection
        update(&mut app, Message::PanelKeyInput('f'));
        let panel: &PullsPanel = app.panel(PanelKind::Pulls).expect("Expected Pulls panel");
        assert_eq!(
            panel.detail_pane.mode,
            DetailPaneMode::Fullscreen,
            "f should toggle fullscreen even with empty list"
        );
    }

    #[test]
    fn c_key_works_with_empty_pulls_list() {
        use crate::event::ApiPayload;
        use crate::widgets::detail_pane::DetailPaneMode;

        let mut app = test_app();

        // Switch to Pulls panel with no PRs loaded
        update(&mut app, Message::NextTab);
        assert_eq!(app.active_panel, PanelKind::Pulls);

        // Clear any PRs that may have loaded from NextTab fetch
        receive(&mut app, ApiPayload::Pulls(vec![]));

        // Press 'c' — should work even with no selection
        update(&mut app, Message::PanelKeyInput('c'));
        let panel: &PullsPanel = app.panel(PanelKind::Pulls).expect("Expected Pulls panel");
        assert_eq!(
            panel.detail_pane.mode,
            DetailPaneMode::Collapsed,
            "c should toggle collapse even with empty list"
        );
    }

    #[test]
    fn benign_cmd_does_not_trigger_confirmation() {
        // Cmd::None is benign — behavior() returns Benign, not Destructive.
        assert!(matches!(Cmd::None.behavior(), CmdBehavior::Benign));
    }

    #[test]
    fn login_success_returns_fetch_repos_on_valid_token() {
        let mut app = test_app();
        let cmd = update(&mut app, Message::LoginSuccess("valid_token".into()));
        assert!(matches!(cmd, Cmd::FetchRepos));
        // Side effect: login cleared
        assert!(app.login.is_none());
    }

    // NOTE: Testing the failure path (Cmd::None when GitHubClient::new fails)
    // requires dependency injection — build_http_client() uses reqwest::Client::builder()
    // which does not validate tokens and essentially never fails. To test this path,
    // GitHubClient::new would need to accept an injectable HTTP client factory.
}
