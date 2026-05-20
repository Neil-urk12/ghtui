use std::collections::HashMap;
use std::fmt;

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::cmd::Cmd;
use crate::config::Config;
use crate::panels::{ActionsPanel, IssuesPanel, NotificationsPanel, PullsPanel, ReposPanel};
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

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Default)]
pub struct SearchState {
    pub active: bool,
    pub query: String,
}

#[derive(Debug)]
pub enum Message {
    NextTab,
    PrevTab,
    FocusPanel(PanelKind),
    Refresh,
    SearchActivate,
    SearchInput(char),
    SearchBackspace,
    SearchSubmit,
    SearchClear,
    ShowError(String),
    DismissError,
    ToggleHelp,
    PanelKeyInput(char),
    Quit,
    Noop,
}

pub struct App {
    pub active_panel: PanelKind,
    pub panels: HashMap<PanelKind, PanelState>,
    pub config: Config,
    pub theme: Theme,
    pub search: SearchState,
    pub error_toast: Option<String>,
    pub show_help: bool,
    pub should_quit: bool,
    pub tick: u64,
    pub rate_remaining: Option<u32>,
    pub rate_reset_secs: Option<u64>,
}

impl App {
    pub fn new(config: Config) -> Self {
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

        let theme = Theme::from_config(&config.theme);

        Self {
            active_panel: PanelKind::Repos,
            panels,
            config,
            theme,
            search: SearchState::default(),
            error_toast: None,
            show_help: false,
            should_quit: false,
            tick: 0,
            rate_remaining: None,
            rate_reset_secs: None,
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
            app.search.active = false;
            app.next_tab();
            Cmd::None
        }
        Message::PrevTab => {
            app.search.active = false;
            app.prev_tab();
            Cmd::None
        }
        Message::FocusPanel(panel) => {
            app.active_panel = panel;
            Cmd::None
        }
        Message::Refresh => Cmd::FetchRepos,
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
            // Filter stays active, query preserved
            Cmd::None
        }
        Message::SearchClear => {
            app.search.active = false;
            app.search.query.clear();
            Cmd::None
        }
        Message::ShowError(msg) => {
            app.error_toast = Some(msg);
            Cmd::None
        }
        Message::DismissError => {
            app.error_toast = None;
            Cmd::None
        }
        Message::ToggleHelp => {
            app.show_help = !app.show_help;
            Cmd::None
        }
        Message::PanelKeyInput(c) => {
            match app.active_panel {
                PanelKind::Repos => {
                    if let Some(PanelState::Repos(ref mut repos)) = app.panels.get_mut(&PanelKind::Repos) {
                        match c {
                            'j' => repos.navigate_down(),
                            'k' => repos.navigate_up(),
                            's' => repos.cycle_sort(),
                            '\n' => {
                                let nav_target = repos.selected_repo().and_then(|repo| {
                                    let parts: Vec<&str> = repo.name.splitn(2, '/').collect();
                                    if parts.len() == 2 {
                                        Some((parts[0].to_string(), parts[1].to_string()))
                                    } else {
                                        None
                                    }
                                });
                                if let Some((owner, repo)) = nav_target {
                                    if let Some(PanelState::Pulls(ref mut pulls)) = app.panels.get_mut(&PanelKind::Pulls) {
                                        pulls.set_repo(owner.clone(), repo.clone());
                                    }
                                    app.active_panel = PanelKind::Pulls;
                                    return Cmd::FetchPulls { owner, repo };
                                }
                            }
                            _ => {}
                        }
                    }
                }
                PanelKind::Pulls => {
                    if let Some(PanelState::Pulls(ref mut pulls)) = app.panels.get_mut(&PanelKind::Pulls) {
                        match c {
                            'j' => pulls.navigate_down(),
                            'k' => pulls.navigate_up(),
                            '1' => {
                                pulls.state_filter = "open".to_string();
                                return Cmd::FetchPulls {
                                    owner: pulls.owner.clone().unwrap_or_default(),
                                    repo: pulls.repo.clone().unwrap_or_default(),
                                };
                            }
                            '2' => {
                                pulls.state_filter = "closed".to_string();
                                return Cmd::FetchPulls {
                                    owner: pulls.owner.clone().unwrap_or_default(),
                                    repo: pulls.repo.clone().unwrap_or_default(),
                                };
                            }
                            '3' => {
                                pulls.state_filter = "all".to_string();
                                return Cmd::FetchPulls {
                                    owner: pulls.owner.clone().unwrap_or_default(),
                                    repo: pulls.repo.clone().unwrap_or_default(),
                                };
                            }
                            'o' => {
                                if let Some(pr) = pulls.selected_pr() {
                                    let _ = std::process::Command::new("gh")
                                        .args(["browse", &format!("{}", pr.number)])
                                        .spawn();
                                }
                            }
                            _ => {}
                        }
                    }
                }
                PanelKind::Issues => {
                    if let Some(PanelState::Issues(ref mut issues)) = app.panels.get_mut(&PanelKind::Issues) {
                        match c {
                            'j' => issues.navigate_down(),
                            'k' => issues.navigate_up(),
                            '1' => {
                                issues.state_filter = "open".to_string();
                                return Cmd::FetchIssues {
                                    owner: issues.owner.clone().unwrap_or_default(),
                                    repo: issues.repo.clone().unwrap_or_default(),
                                };
                            }
                            '2' => {
                                issues.state_filter = "closed".to_string();
                                return Cmd::FetchIssues {
                                    owner: issues.owner.clone().unwrap_or_default(),
                                    repo: issues.repo.clone().unwrap_or_default(),
                                };
                            }
                            '3' => {
                                issues.state_filter = "all".to_string();
                                return Cmd::FetchIssues {
                                    owner: issues.owner.clone().unwrap_or_default(),
                                    repo: issues.repo.clone().unwrap_or_default(),
                                };
                            }
                            'o' => {
                                if let Some(issue) = issues.selected_item() {
                                    let _ = std::process::Command::new("gh")
                                        .args(["browse", &format!("{}", issue.number)])
                                        .spawn();
                                }
                            }
                            _ => {}
                        }
                    }
                }
                PanelKind::Notifications => {
                    if let Some(PanelState::Notifications(ref mut notifs)) = app.panels.get_mut(&PanelKind::Notifications) {
                        match c {
                            'j' => notifs.navigate_down(),
                            'k' => notifs.navigate_up(),
                            'x' => { /* TODO: mark single notification as read */ }
                            'X' => { /* TODO: mark all as read */ }
                            '1' => { notifs.show_all = false; }
                            '2' => { notifs.show_all = true; }
                            _ => {}
                        }
                    }
                }
                PanelKind::Actions => {
                    if let Some(PanelState::Actions(ref mut actions)) = app.panels.get_mut(&PanelKind::Actions) {
                        match c {
                            'j' => actions.navigate_down(),
                            'k' => actions.navigate_up(),
                            '\n' => { actions.toggle_expand(); }
                            'r' => { /* TODO: re-run workflow */ }
                            'c' => { /* TODO: cancel workflow */ }
                            _ => {}
                        }
                    }
                }
            }
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
    match &app.active_panel {
        PanelKind::Repos => {
            if let Some(PanelState::Repos(ref repos)) = app.panels.get(&PanelKind::Repos) {
                crate::panels::repos::ReposPanel::view(frame, content_area, repos, true, &app.theme, app.tick);
            }
        }
        PanelKind::Pulls => {
            if let Some(PanelState::Pulls(ref pulls)) = app.panels.get(&PanelKind::Pulls) {
                crate::panels::pulls::PullsPanel::view(frame, content_area, pulls, true, &app.theme, app.tick);
            }
        }
        PanelKind::Issues => {
            if let Some(PanelState::Issues(ref issues)) = app.panels.get(&PanelKind::Issues) {
                crate::panels::issues::IssuesPanel::view(frame, content_area, issues, true, &app.theme, app.tick);
            }
        }
        PanelKind::Notifications => {
            if let Some(PanelState::Notifications(ref notifs)) = app.panels.get(&PanelKind::Notifications) {
                crate::panels::notifications::NotificationsPanel::view(frame, content_area, notifs, true, &app.theme, app.tick);
            }
        }
        PanelKind::Actions => {
            if let Some(PanelState::Actions(ref actions)) = app.panels.get(&PanelKind::Actions) {
                crate::panels::actions::ActionsPanel::view(frame, content_area, actions, true, &app.theme, app.tick);
            }
        }
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

    // Help overlay
    if app.show_help {
        popup::render_help(frame, area, &app.active_panel.to_string());
    }
}
