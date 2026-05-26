use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::api::client::GitHubClient;
use crate::config::Config;
use crate::domain::FilterState;
use crate::panels::{ActionsPanelItem, IssuesPanelItem, NotificationsPanelItem, PullsPanelItem, ReposPanelItem};
use crate::panels::login::LoginState;
use crate::panels::panel::{ListPanel, ListPanelData, SelectionState};
use crate::theme::Theme;
use crate::widgets::detail_pane::DetailPane;

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
    Repos(ListPanel<ReposPanelItem>),
    Pulls(ListPanel<PullsPanelItem>),
    Issues(ListPanel<IssuesPanelItem>),
    Notifications(ListPanel<NotificationsPanelItem>),
    Actions(ListPanel<ActionsPanelItem>),
}

impl PanelState {

    pub fn apply_filter(&mut self, query: &str) {
        match self {
            PanelState::Repos(p) => p.apply_filter(query),
            PanelState::Pulls(p) => p.apply_filter(query),
            PanelState::Issues(p) => p.apply_filter(query),
            PanelState::Notifications(p) => p.apply_filter(query),
            PanelState::Actions(p) => p.apply_filter(query),
        }
    }

    pub fn handle_key(&mut self, key: char) -> Option<crate::cmd::Cmd> {
        match self {
            PanelState::Repos(p) => p.handle_key(key),
            PanelState::Pulls(p) => p.handle_key(key),
            PanelState::Issues(p) => p.handle_key(key),
            PanelState::Notifications(p) => p.handle_key(key),
            PanelState::Actions(p) => p.handle_key(key),
        }
    }

    pub fn render(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
        has_focus: bool,
        theme: &crate::theme::Theme,
        tick: u64,
    ) {
        match self {
            PanelState::Repos(p) => p.render(frame, area, has_focus, theme, tick),
            PanelState::Pulls(p) => p.render(frame, area, has_focus, theme, tick),
            PanelState::Issues(p) => p.render(frame, area, has_focus, theme, tick),
            PanelState::Notifications(p) => p.render(frame, area, has_focus, theme, tick),
            PanelState::Actions(p) => p.render(frame, area, has_focus, theme, tick),
        }
    }

    pub fn fetch_cmd(&self) -> crate::cmd::Cmd {
        match self {
            PanelState::Repos(p) => p.fetch_cmd(),
            PanelState::Pulls(p) => p.fetch_cmd(),
            PanelState::Issues(p) => p.fetch_cmd(),
            PanelState::Notifications(p) => p.fetch_cmd(),
            PanelState::Actions(p) => p.fetch_cmd(),
        }
    }

    /// Update PR with comment data (Pulls panel only).
    pub fn update_pr_comments(&mut self, pr_number: u64, comments: Option<Vec<crate::domain::PrComment>>, reviews: Option<Vec<crate::domain::PrReview>>, code_comments: Option<Vec<crate::domain::PrCodeComment>>) {
        if let PanelState::Pulls(p) = self {
            if let Some(pr) = p.data.items.iter_mut().find(|pr| pr.number == pr_number as u32) {
                // Only overwrite if we got data (None means keep existing state for retry)
                if let Some(c) = comments { pr.comments = Some(c); }
                if let Some(r) = reviews { pr.reviews = Some(r); }
                if let Some(cc) = code_comments { pr.code_comments = Some(cc); }
            }
        }
        self.set_loading(false);
    }

    pub fn set_loading(&mut self, loading: bool) {
        match self {
            PanelState::Repos(p) => p.set_loading(loading),
            PanelState::Pulls(p) => p.set_loading(loading),
            PanelState::Issues(p) => p.set_loading(loading),
            PanelState::Notifications(p) => p.set_loading(loading),
            PanelState::Actions(p) => p.set_loading(loading),
        }
    }

    pub fn set_repo_context(&mut self, owner: &str, repo: &str) {
        match self {
            PanelState::Repos(p) => p.set_repo_context(owner, repo),
            PanelState::Pulls(p) => p.set_repo_context(owner, repo),
            PanelState::Issues(p) => p.set_repo_context(owner, repo),
            PanelState::Notifications(p) => p.set_repo_context(owner, repo),
            PanelState::Actions(p) => p.set_repo_context(owner, repo),
        }
    }

    pub fn load_data(&mut self, payload: crate::event::ApiPayload) {
        match self {
            PanelState::Repos(p) => p.load_data(payload),
            PanelState::Pulls(p) => p.load_data(payload),
            PanelState::Issues(p) => p.load_data(payload),
            PanelState::Notifications(p) => p.load_data(payload),
            PanelState::Actions(p) => p.load_data(payload),
        }
    }

    pub fn scroll_detail_up(&mut self) {
        match self {
            PanelState::Repos(p) => p.scroll_detail_up(),
            PanelState::Pulls(p) => p.scroll_detail_up(),
            PanelState::Issues(p) => p.scroll_detail_up(),
            PanelState::Notifications(p) => p.scroll_detail_up(),
            PanelState::Actions(p) => p.scroll_detail_up(),
        }
    }

    pub fn scroll_detail_down(&mut self) {
        match self {
            PanelState::Repos(p) => p.scroll_detail_down(),
            PanelState::Pulls(p) => p.scroll_detail_down(),
            PanelState::Issues(p) => p.scroll_detail_down(),
            PanelState::Notifications(p) => p.scroll_detail_down(),
            PanelState::Actions(p) => p.scroll_detail_down(),
        }
    }

    pub fn selected_display(&self) -> Option<String> {
        match self {
            PanelState::Repos(p) => p.selected_display(),
            PanelState::Pulls(p) => p.selected_display(),
            PanelState::Issues(p) => p.selected_display(),
            PanelState::Notifications(p) => p.selected_display(),
            PanelState::Actions(p) => p.selected_display(),
        }
    }
}


#[derive(Debug, Default)]
pub struct SearchState {
    pub active: bool,
    pub query: String,
}


/// State for the confirmation dialog. Holds the pending destructive command
/// and display information for the confirmation modal.
pub struct ConfirmState {
    pub pending_cmd: crate::cmd::Cmd,
    pub action_label: String,
    pub detail: String,
}

impl std::fmt::Debug for ConfirmState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfirmState")
            .field("action_label", &self.action_label)
            .field("detail", &self.detail)
            .finish()
    }
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
    pub toast_tick: u64,
    pub show_help: bool,
    pub should_quit: bool,
    pub tick: u64,
    pub rate_remaining: Option<u32>,
    pub rate_reset_secs: Option<u64>,
    pub client: Arc<GitHubClient>,
    pub confirmation: Option<ConfirmState>,
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
        panels.insert(PanelKind::Repos, PanelState::Repos(ListPanel::default()));
        panels.insert(PanelKind::Pulls, PanelState::Pulls(ListPanel {
                data: ListPanelData::new(),
                selection: SelectionState::table(),
                detail_pane: DetailPane::new(),
                item: PullsPanelItem { state_filter: FilterState::from(config.defaults.pulls_state.as_str()) },
            }));
        panels.insert(PanelKind::Issues, PanelState::Issues(ListPanel {
                data: ListPanelData::new(),
                selection: SelectionState::list(),
                detail_pane: DetailPane::new(),
                item: IssuesPanelItem { state_filter: FilterState::from(config.defaults.issues_state.as_str()) },
            }));
        panels.insert(PanelKind::Notifications, PanelState::Notifications(ListPanel {
                data: ListPanelData::new(),
                selection: SelectionState::list(),
                detail_pane: DetailPane::new(),
                item: NotificationsPanelItem { show_all: config.defaults.notifications_show_all },
            }));
        panels.insert(PanelKind::Actions, PanelState::Actions(ListPanel::default()));

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
            toast_tick: 0,
            show_help: false,
            should_quit: false,
            tick: 0,
            rate_remaining: None,
            rate_reset_secs: None,
            client,
            confirmation: None,
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

