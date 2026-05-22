use std::fmt;
use std::sync::Arc;

use crate::api::client::GitHubClient;
use crate::app::{App, AppState, PanelKind};
use crate::cmd::Cmd;
use crate::navigation;
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
        }
    }
}

pub fn update(app: &mut App, msg: Message) -> Cmd {
    match msg {
        Message::NextTab => {
            app.search.active = false;
            app.next_tab();
            let cmd = app.panels.get(&app.active_panel)
                .map(|entry| entry.panel.as_panel().fetch_cmd())
                .unwrap_or(Cmd::None);
            if !matches!(cmd, Cmd::None) {
                if let Some(entry) = app.panels.get_mut(&app.active_panel) {
                    entry.panel.as_panel_mut().set_loading(true);
                }
            }
            cmd
        }
        Message::PrevTab => {
            app.search.active = false;
            app.prev_tab();
            let cmd = app.panels.get(&app.active_panel)
                .map(|entry| entry.panel.as_panel().fetch_cmd())
                .unwrap_or(Cmd::None);
            if !matches!(cmd, Cmd::None) {
                if let Some(entry) = app.panels.get_mut(&app.active_panel) {
                    entry.panel.as_panel_mut().set_loading(true);
                }
            }
            cmd
        }
        Message::Refresh => {
            let cmd = app.panels.get(&app.active_panel)
                .map(|entry| entry.panel.as_panel().fetch_cmd())
                .unwrap_or(Cmd::None);
            if !matches!(cmd, Cmd::None) {
                if let Some(entry) = app.panels.get_mut(&app.active_panel) {
                    entry.panel.as_panel_mut().set_loading(true);
                }
            }
            cmd
        }
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
            Cmd::None
        }
        Message::SearchClear => {
            app.search.active = false;
            app.search.query.clear();
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
                crate::panels::panel::scroll_detail_up(&mut entry.detail_pane);
            }
            Cmd::None
        }
        Message::DetailScrollDown => {
            if let Some(entry) = app.panels.get_mut(&app.active_panel) {
                crate::panels::panel::scroll_detail_down(&mut entry.detail_pane);
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
                entry.panel.as_panel_mut().handle_key(c, &mut entry.selection, &mut entry.detail_pane).unwrap_or(Cmd::None)
            } else {
                Cmd::None
            };
            match cmd {
                Cmd::OpenRepoDetail { owner, repo } => {
                    app.active_panel = PanelKind::Pulls;
                    let cmd = navigation::handle_open_repo_detail(&mut app.panels, owner, repo);
                    if let Some(entry) = app.panels.get_mut(&PanelKind::Pulls) {
                        entry.panel.as_panel_mut().set_loading(true);
                    }
                    cmd
                }
                Cmd::None => Cmd::None,
                other => {
                    if let Some(entry) = app.panels.get_mut(&app.active_panel) {
                        entry.panel.as_panel_mut().set_loading(true);
                    }
                    other
                }
            }
        }
        Message::LoginInit(login) => {
            app.state = AppState::Login;
            app.login = Some(login);
            Cmd::None
        }
        Message::LoginSuccess(token) => {
            let _ = crate::api::auth::save_token(&token, crate::api::auth::GHTUI_SCOPES);
            app.client = Arc::new(GitHubClient::new(token));
            app.login = None;
            app.state = AppState::Main;
            if let Some(entry) = app.panels.get_mut(&PanelKind::Repos) {
                entry.panel.as_panel_mut().set_loading(true);
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
            };
            if let Some(kind) = target {
                if let Some(entry) = app.panels.get_mut(&kind) {
                    entry.panel.as_panel_mut().load_data(payload, &mut entry.selection);
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
            if let crate::app::PanelState::Repos(panel) = &entry.panel {
                assert_eq!(panel.repos.len(), 1);
                assert_eq!(panel.repos[0].name, "test/repo");
            } else {
                panic!("Expected Repos panel");
            }
        } else {
            panic!("Expected PanelEntry for Repos");
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
        assert!(!app.search.active);
    }
}