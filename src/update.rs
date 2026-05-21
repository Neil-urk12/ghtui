use std::fmt;
use std::sync::Arc;

use crate::api::client::GitHubClient;
use crate::app::{App, AppState, PanelKind, PanelState};
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

fn fetch_for_active_panel(app: &App) -> Cmd {
    match app.active_panel {
        PanelKind::Repos => Cmd::FetchRepos,
        PanelKind::Pulls => {
            if let Some(PanelState::Pulls(ref p)) = app.panels.get(&PanelKind::Pulls) {
                match (&p.owner, &p.repo) {
                    (Some(owner), Some(repo)) => Cmd::FetchPulls {
                        owner: owner.clone(),
                        repo: repo.clone(),
                        state: p.state_filter.clone(),
                    },
                    _ => Cmd::None,
                }
            } else {
                Cmd::None
            }
        }
        PanelKind::Issues => {
            if let Some(PanelState::Issues(ref i)) = app.panels.get(&PanelKind::Issues) {
                match (&i.owner, &i.repo) {
                    (Some(owner), Some(repo)) => Cmd::FetchIssues {
                        owner: owner.clone(),
                        repo: repo.clone(),
                        state: i.state_filter.clone(),
                    },
                    _ => Cmd::None,
                }
            } else {
                Cmd::None
            }
        }
        PanelKind::Notifications => {
            let show_all = if let Some(PanelState::Notifications(ref n)) = app.panels.get(&PanelKind::Notifications) {
                n.show_all
            } else {
                false
            };
            Cmd::FetchNotifications { all: show_all }
        }
        PanelKind::Actions => {
            if let Some(PanelState::Actions(ref a)) = app.panels.get(&PanelKind::Actions) {
                if let (Some(owner), Some(repo)) = (&a.owner, &a.repo) {
                    return Cmd::FetchActions { owner: owner.clone(), repo: repo.clone() };
                }
            }
            Cmd::None
        }
    }
}

pub fn update(app: &mut App, msg: Message) -> Cmd {
    match msg {
        Message::NextTab => {
            app.search.active = false;
            app.next_tab();
            fetch_for_active_panel(app)
        }
        Message::PrevTab => {
            app.search.active = false;
            app.prev_tab();
            fetch_for_active_panel(app)
        }
        Message::Refresh => fetch_for_active_panel(app),
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
            match payload {
                ApiPayload::Repos(repos) => {
                    if let Some(PanelState::Repos(ref mut panel)) = app.panels.get_mut(&PanelKind::Repos) {
                        panel.set_repos(repos);
                    }
                }
                ApiPayload::Pulls(pulls) => {
                    if let Some(PanelState::Pulls(ref mut panel)) = app.panels.get_mut(&PanelKind::Pulls) {
                        panel.set_pulls(pulls);
                    }
                }
                ApiPayload::Issues(issues) => {
                    if let Some(PanelState::Issues(ref mut panel)) = app.panels.get_mut(&PanelKind::Issues) {
                        panel.set_issues(issues);
                    }
                }
                ApiPayload::Notifications(notifs) => {
                    if let Some(PanelState::Notifications(ref mut panel)) =
                        app.panels.get_mut(&PanelKind::Notifications)
                    {
                        panel.set_notifications(notifs);
                    }
                }
                ApiPayload::Actions(runs) => {
                    if let Some(PanelState::Actions(ref mut panel)) =
                        app.panels.get_mut(&PanelKind::Actions)
                    {
                        panel.runs = runs;
                        panel.loading = false;
                    }
                }
                ApiPayload::Success(msg) => {
                    app.success_toast = Some(msg);
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