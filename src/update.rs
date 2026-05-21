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
            app.panels.get(&app.active_panel)
                .map(|p| p.as_panel().fetch_cmd())
                .unwrap_or(Cmd::None)
        }
        Message::PrevTab => {
            app.search.active = false;
            app.prev_tab();
            app.panels.get(&app.active_panel)
                .map(|p| p.as_panel().fetch_cmd())
                .unwrap_or(Cmd::None)
        }
        Message::Refresh => {
            app.panels.get(&app.active_panel)
                .map(|p| p.as_panel().fetch_cmd())
                .unwrap_or(Cmd::None)
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
            if let Some(panel) = app.panels.get_mut(&app.active_panel) {
                panel.as_panel_mut().scroll_detail_up();
            }
            Cmd::None
        }
        Message::DetailScrollDown => {
            if let Some(panel) = app.panels.get_mut(&app.active_panel) {
                panel.as_panel_mut().scroll_detail_down();
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
            if let Some(panel) = app.panels.get_mut(&app.active_panel) {
                let cmd = panel.as_panel_mut().handle_key(c).unwrap_or(Cmd::None);
                match cmd {
                    Cmd::OpenRepoDetail { owner, repo } => {
                        app.active_panel = PanelKind::Pulls;
                        navigation::handle_open_repo_detail(&mut app.panels, owner, repo)
                    }
                    other => other,
                }
            } else {
                Cmd::None
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
                if let Some(panel) = app.panels.get_mut(&kind) {
                    panel.as_panel_mut().load_data(payload);
                }
            }
            Cmd::None
        }
        Message::ApiError(msg) => {
            app.error_toast = Some(msg);
            Cmd::None
        }
        Message::RateLimitUpdate { remaining, reset_secs } => {
            app.rate_remaining = remaining;
            app.rate_reset_secs = reset_secs;
            Cmd::None
        }
        Message::Tick => {
            app.tick += 1;
            Cmd::None
        }
    }
}