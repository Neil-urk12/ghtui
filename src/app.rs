use std::collections::HashMap;
use std::fmt;

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::cmd::Cmd;
use crate::theme::Theme;
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

#[derive(Debug, Default)]
pub struct ReposPanel {
    pub repos: Vec<Repo>,
    pub list_state: ListState,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct PullsPanel {
    pub pulls: Vec<PullRequest>,
    pub list_state: ListState,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct IssuesPanel {
    pub issues: Vec<Issue>,
    pub list_state: ListState,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct NotificationsPanel {
    pub notifications: Vec<Notification>,
    pub list_state: ListState,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct ActionsPanel {
    pub runs: Vec<WorkflowRun>,
    pub list_state: ListState,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum PanelState {
    Repos(ReposPanel),
    Pulls(PullsPanel),
    Issues(IssuesPanel),
    Notifications(NotificationsPanel),
    Actions(ActionsPanel),
}

impl PanelState {
    pub fn loading_mut(&mut self) -> &mut bool {
        match self {
            PanelState::Repos(p) => &mut p.loading,
            PanelState::Pulls(p) => &mut p.loading,
            PanelState::Issues(p) => &mut p.loading,
            PanelState::Notifications(p) => &mut p.loading,
            PanelState::Actions(p) => &mut p.loading,
        }
    }
}

#[derive(Debug, Default)]
pub struct Repo {
    pub name: String,
    pub owner: String,
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

#[derive(Debug, Default)]
pub struct Notification {
    pub repo_name: String,
    pub subject_title: String,
    pub subject_type: String,
    pub reason: String,
    pub unread: bool,
    pub updated_at: String,
}

#[derive(Debug, Default)]
pub struct WorkflowRun {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub branch: String,
    pub event: String,
    pub created_at: String,
    pub run_number: u32,
}

#[derive(Debug)]
pub enum Message {
    NextTab,
    PrevTab,
    FocusPanel(PanelKind),
    Refresh,
    Quit,
    Noop,
}

pub struct App {
    pub active_panel: PanelKind,
    pub panels: HashMap<PanelKind, PanelState>,
    pub theme: Theme,
    pub should_quit: bool,
    pub tick: u64,
}

impl App {
    pub fn new() -> Self {
        let mut panels = HashMap::new();
        panels.insert(PanelKind::Repos, PanelState::Repos(ReposPanel::default()));
        panels.insert(PanelKind::Pulls, PanelState::Pulls(PullsPanel::default()));
        panels.insert(
            PanelKind::Issues,
            PanelState::Issues(IssuesPanel::default()),
        );
        panels.insert(
            PanelKind::Notifications,
            PanelState::Notifications(NotificationsPanel::default()),
        );
        panels.insert(
            PanelKind::Actions,
            PanelState::Actions(ActionsPanel::default()),
        );

        Self {
            active_panel: PanelKind::Repos,
            panels,
            theme: Theme::default(),
            should_quit: false,
            tick: 0,
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

pub fn update(app: &mut App, msg: Message) -> Cmd {
    match msg {
        Message::NextTab => {
            app.next_tab();
            Cmd::None
        }
        Message::PrevTab => {
            app.prev_tab();
            Cmd::None
        }
        Message::FocusPanel(panel) => {
            app.active_panel = panel;
            Cmd::None
        }
        Message::Refresh => Cmd::FetchRepos,
        Message::Quit => {
            app.should_quit = true;
            Cmd::None
        }
        Message::Noop => Cmd::None,
    }
}

pub fn view(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(area);

    render_tab_bar(frame, layout[0], app);

    // Content area - placeholder
    let content = Paragraph::new(format!(
        "Panel: {} (content area - not yet implemented)",
        app.active_panel
    ))
    .block(
        Block::bordered()
            .title(format!(" {} ", app.active_panel))
            .border_style(app.theme.active_border),
    );
    frame.render_widget(content, layout[1]);

    render_status_bar(frame, layout[2], app);
}
