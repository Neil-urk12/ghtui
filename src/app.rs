use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::api::client::GitHubClient;
use crate::cmd::Cmd;
use crate::config::Config;
use crate::panels::{login::LoginState, ActionsPanel, IssuesPanel, NotificationsPanel, Panel, PullsPanel, ReposPanel};
use crate::theme::Theme;
use crate::widgets::popup;
use crate::widgets::status_bar::render_status_bar;
use crate::widgets::tab_bar::render_tab_bar;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelKind {
    Repos,
    Pulls,
    Issues,
    Notifications,
    Actions,
}

impl PanelKind {
    pub const ALL: [PanelKind; 5] = [
        PanelKind::Repos,
        PanelKind::Pulls,
        PanelKind::Issues,
        PanelKind::Notifications,
        PanelKind::Actions,
    ];
}

impl fmt::Display for PanelKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PanelKind::Repos => write!(f, "Repos"),
            PanelKind::Pulls => write!(f, "PRs"),
            PanelKind::Issues => write!(f, "Issues"),
            PanelKind::Notifications => write!(f, "Notif"),
            PanelKind::Actions => write!(f, "Actions"),
        }
    }
}

pub enum PanelState {
    Repos(ReposPanel),
    Pulls(PullsPanel),
    Issues(IssuesPanel),
    Notifications(NotificationsPanel),
    Actions(ActionsPanel),
}

impl PanelState {
    pub fn as_panel(&self) -> &dyn Panel {
        match self {
            PanelState::Repos(p) => p,
            PanelState::Pulls(p) => p,
            PanelState::Issues(p) => p,
            PanelState::Notifications(p) => p,
            PanelState::Actions(p) => p,
        }
    }

    pub fn as_panel_mut(&mut self) -> &mut dyn Panel {
        match self {
            PanelState::Repos(p) => p,
            PanelState::Pulls(p) => p,
            PanelState::Issues(p) => p,
            PanelState::Notifications(p) => p,
            PanelState::Actions(p) => p,
        }
    }
}

#[derive(Debug, Default)]
pub struct Repo {
    pub name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stars: u32,
    pub forks: u32,
    pub updated_at: String,
    pub visibility: String,
    pub default_branch: String,
}

#[derive(Debug, Default)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub state: String,
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub body: Option<String>,
}

#[derive(Debug, Default)]
pub struct Issue {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub state: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Notification {
    pub id: String,
    pub repo_name: String,
    pub subject_title: String,
    pub subject_type: String,
    pub reason: String,
    pub unread: bool,
    pub updated_at: String,
}

#[derive(Debug, Default)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub branch: String,
    pub event: String,
    pub created_at: String,
    pub run_number: u32,
}

#[derive(Debug, Default)]
pub struct SearchState {
    pub active: bool,
    pub query: String,
}

pub enum AppState {
    Login,
    Main,
}

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
        }
    }
}

pub struct App {
    pub state: AppState,
    pub login: Option<LoginState>,
    pub active_panel: PanelKind,
    pub panels: HashMap<PanelKind, PanelState>,
    pub config: Config,
    pub theme: Theme,
    pub search: SearchState,
    pub error_toast: Option<String>,
    pub success_toast: Option<String>,
    pub show_help: bool,
    pub should_quit: bool,
    pub tick: u64,
    pub rate_remaining: Option<u32>,
    pub rate_reset_secs: Option<u64>,
    pub client: Arc<GitHubClient>,
}

impl App {
    pub fn new(config: Config, token: Option<String>) -> Self {
        let theme = Theme::from_config(&config.theme);

        let (client, state) = match token {
            Some(token) => (Arc::new(GitHubClient::new(token)), AppState::Main),
            None => (Arc::new(GitHubClient::new(String::new())), AppState::Login),
        };

        // Apply config defaults
        let mut panels = HashMap::new();
        panels.insert(PanelKind::Repos, PanelState::Repos(ReposPanel::default()));
        let mut pulls = PullsPanel::default();
        pulls.state_filter = config.defaults.pulls_state.clone();
        panels.insert(PanelKind::Pulls, PanelState::Pulls(pulls));
        let mut issues = IssuesPanel::default();
        issues.state_filter = config.defaults.issues_state.clone();
        panels.insert(PanelKind::Issues, PanelState::Issues(issues));
        let mut notifs = NotificationsPanel::default();
        notifs.show_all = config.defaults.notifications_show_all;
        panels.insert(PanelKind::Notifications, PanelState::Notifications(notifs));
        panels.insert(PanelKind::Actions, PanelState::Actions(ActionsPanel::default()));

        Self {
            state,
            login: None,
            active_panel: PanelKind::Repos,
            panels,
            config,
            theme,
            search: SearchState::default(),
            error_toast: None,
            success_toast: None,
            show_help: false,
            should_quit: false,
            tick: 0,
            rate_remaining: None,
            rate_reset_secs: None,
            client,
        }
    }

    pub fn next_tab(&mut self) {
        let idx = PanelKind::ALL
            .iter()
            .position(|&p| p == self.active_panel)
            .unwrap_or(0);
        self.active_panel = PanelKind::ALL[(idx + 1) % PanelKind::ALL.len()];
    }

    pub fn prev_tab(&mut self) {
        let idx = PanelKind::ALL
            .iter()
            .position(|&p| p == self.active_panel)
            .unwrap_or(0);
        self.active_panel = PanelKind::ALL[(idx + PanelKind::ALL.len() - 1) % PanelKind::ALL.len()];
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
                        // Set context on downstream panels
                        if let Some(PanelState::Pulls(ref mut pulls)) = app.panels.get_mut(&PanelKind::Pulls) {
                            pulls.set_repo(owner.clone(), repo.clone());
                        }
                        if let Some(PanelState::Actions(ref mut actions)) = app.panels.get_mut(&PanelKind::Actions) {
                            actions.owner = Some(owner.clone());
                            actions.repo = Some(repo.clone());
                        }
                        app.active_panel = PanelKind::Pulls;
                        let state = if let Some(PanelState::Pulls(ref p)) = app.panels.get(&PanelKind::Pulls) {
                            p.state_filter.clone()
                        } else {
                            "open".to_string()
                        };
                        Cmd::FetchPulls { owner, repo, state }
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
    }
}

pub fn view(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Login screen — render instead of main UI
    if matches!(app.state, AppState::Login) {
        if let Some(ref login) = app.login {
            crate::panels::login::render_login(frame, login);
        } else {
            let loading = Paragraph::new("Initializing login...")
                .alignment(Alignment::Center);
            frame.render_widget(loading, area);
        }
        return;
    }

    // Main layout: tab bar, content, search/status
    let (content_height, status_height) = if app.search.active {
        (Constraint::Fill(1), Constraint::Length(2))
    } else {
        (Constraint::Fill(1), Constraint::Length(1))
    };

    let layout = Layout::vertical([
        Constraint::Length(1),
        content_height,
        status_height,
    ])
    .split(area);

    render_tab_bar(frame, layout[0], app);

    // Content area - delegate to active panel
    let content_area = layout[1];
    if let Some(panel) = app.panels.get(&app.active_panel) {
        panel.as_panel().render(frame, content_area, true, &app.theme, app.tick);
    }

    // Status area + optional search bar
    let status_area = layout[2];
    if app.search.active {
        let status_layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(status_area);

        // Search bar
        let search_text = format!("/{}", app.search.query);
        let search = Paragraph::new(search_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default());
        frame.render_widget(search, status_layout[0]);

        render_status_bar(frame, status_layout[1], app);
    } else {
        render_status_bar(frame, status_area, app);
    }

    // Error toast overlay
    if let Some(ref err) = app.error_toast {
        popup::render_error_toast(frame, area, err);
    }

    // Success toast overlay
    if let Some(ref msg) = app.success_toast {
        popup::render_success_toast(frame, area, msg);
    }

    // Help overlay
    if app.show_help {
        popup::render_help(frame, area, &app.active_panel.to_string());
    }
}
