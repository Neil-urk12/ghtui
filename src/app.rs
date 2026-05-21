use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::api::client::GitHubClient;
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
pub struct SearchState {
    pub active: bool,
    pub query: String,
}

pub enum AppState {
    Login,
    Main,
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
        panels.insert(PanelKind::Pulls, PanelState::Pulls(PullsPanel {
            state_filter: config.defaults.pulls_state.clone(),
            ..Default::default()
        }));
        panels.insert(PanelKind::Issues, PanelState::Issues(IssuesPanel {
            state_filter: config.defaults.issues_state.clone(),
            ..Default::default()
        }));
        panels.insert(PanelKind::Notifications, PanelState::Notifications(NotificationsPanel {
            show_all: config.defaults.notifications_show_all,
            ..Default::default()
        }));
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
