use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::api::client::GitHubClient;
use crate::config::Config;
use crate::domain::FilterState;
use crate::panels::login::LoginState;
use crate::panels::panel::{ListPanel, ListPanelData, SelectionState};
use crate::panels::{
    ActionsPanelItem, IssuesPanelItem, NotificationsPanelItem, PullsPanelItem, ReposPanelItem,
};
use crate::theme::Theme;
use crate::widgets::detail_pane::DetailPane;

/// Macro to delegate method calls across all PanelState variants.
/// Eliminates repetitive match statements for common panel operations.
macro_rules! panel_delegate {
    // &mut self with args, returning a value
    (fn $method:ident(&mut self, $($arg:ident : $ty:ty),*) -> $ret:ty) => {
        pub fn $method(&mut self, $($arg: $ty),*) -> $ret {
            match self {
                PanelState::Repos(p) => p.$method($($arg),*),
                PanelState::Pulls(p) => p.$method($($arg),*),
                PanelState::Issues(p) => p.$method($($arg),*),
                PanelState::Notifications(p) => p.$method($($arg),*),
                PanelState::Actions(p) => p.$method($($arg),*),
            }
        }
    };
    // &mut self with args, returning nothing
    (fn $method:ident(&mut self, $($arg:ident : $ty:ty),*)) => {
        pub fn $method(&mut self, $($arg: $ty),*) {
            match self {
                PanelState::Repos(p) => p.$method($($arg),*),
                PanelState::Pulls(p) => p.$method($($arg),*),
                PanelState::Issues(p) => p.$method($($arg),*),
                PanelState::Notifications(p) => p.$method($($arg),*),
                PanelState::Actions(p) => p.$method($($arg),*),
            }
        }
    };
    // &mut self without args, returning nothing
    (fn $method:ident(&mut self)) => {
        pub fn $method(&mut self) {
            match self {
                PanelState::Repos(p) => p.$method(),
                PanelState::Pulls(p) => p.$method(),
                PanelState::Issues(p) => p.$method(),
                PanelState::Notifications(p) => p.$method(),
                PanelState::Actions(p) => p.$method(),
            }
        }
    };
    // &self without args, returning a value
    (fn $method:ident(&self) -> $ret:ty) => {
        pub fn $method(&self) -> $ret {
            match self {
                PanelState::Repos(p) => p.$method(),
                PanelState::Pulls(p) => p.$method(),
                PanelState::Issues(p) => p.$method(),
                PanelState::Notifications(p) => p.$method(),
                PanelState::Actions(p) => p.$method(),
            }
        }
    };
}

pub struct PrCommentUpdate {
    pub comments: Option<Vec<crate::domain::PrComment>>,
    pub reviews: Option<Vec<crate::domain::PrReview>>,
    pub code_comments: Option<Vec<crate::domain::PrCodeComment>>,
    pub comment_err: Option<String>,
    pub review_err: Option<String>,
    pub code_comment_err: Option<String>,
    pub files: Option<Vec<crate::domain::PrFile>>,
    pub file_err: Option<String>,
}
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
    // Delegated methods — all variants share the same ListPanel<T> API.
    panel_delegate!(fn apply_filter(&mut self, query: &str));
    panel_delegate!(fn handle_key(&mut self, key: char) -> Option<crate::cmd::Cmd>);
    panel_delegate!(fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect, has_focus: bool, theme: &crate::theme::Theme, tick: u64));
    panel_delegate!(fn fetch_cmd(&self) -> crate::cmd::Cmd);
    panel_delegate!(fn set_loading(&mut self, loading: bool));
    panel_delegate!(fn set_repo_context(&mut self, owner: &str, repo: &str));
    panel_delegate!(fn load_data(&mut self, payload: crate::event::ApiPayload));
    panel_delegate!(fn scroll_detail_up(&mut self));
    panel_delegate!(fn scroll_detail_down(&mut self));
    panel_delegate!(fn selected_display(&self) -> Option<String>);

    /// Update PR with comment data (Pulls panel only).
    pub fn update_pr_comments(&mut self, pr_number: u64, update: PrCommentUpdate) {
        if let PanelState::Pulls(p) = self {
            if let Some(pr) = p.data.items.iter_mut().find(|pr| pr.number == pr_number) {
                // On success: store data, clear error. On error: clear stale data, store error.
                // Neither: keep existing state for retry.
                if let Some(c) = update.comments {
                    pr.comments = Some(c);
                    pr.comment_errors = None;
                } else if let Some(e) = update.comment_err {
                    pr.comments = None;
                    pr.comment_errors = Some(e);
                }
                if let Some(r) = update.reviews {
                    pr.reviews = Some(r);
                    pr.review_errors = None;
                } else if let Some(e) = update.review_err {
                    pr.reviews = None;
                    pr.review_errors = Some(e);
                }
                if let Some(cc) = update.code_comments {
                    pr.code_comments = Some(cc);
                    pr.code_comment_errors = None;
                } else if let Some(e) = update.code_comment_err {
                    pr.code_comments = None;
                    pr.code_comment_errors = Some(e);
                }
                if let Some(f) = update.files {
                    pr.files = Some(f);
                    pr.file_errors = None;
                } else if let Some(e) = update.file_err {
                    pr.files = None;
                    pr.file_errors = Some(e);
                }
            }
        }
        self.set_loading(false);
    }

    // Pulls-only detail pane toggles.
    pub fn toggle_diff(&mut self) {
        if let PanelState::Pulls(p) = self {
            p.detail_pane.toggle_diff();
        }
    }

    pub fn toggle_fullscreen(&mut self) {
        if let PanelState::Pulls(p) = self {
            p.detail_pane.toggle_fullscreen();
        }
    }

    pub fn toggle_collapse(&mut self) {
        if let PanelState::Pulls(p) = self {
            p.detail_pane.toggle_collapse();
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

/// Toast notification state (error/success messages with auto-dismiss).
#[derive(Default)]
pub struct ToastState {
    /// Error message to display, if any.
    pub error: Option<String>,
    /// Success message to display, if any.
    pub success: Option<String>,
    /// Frame counter for auto-dismiss (resets on new toast).
    pub tick: u64,
}

impl ToastState {
    /// Set an error message, resetting the dismiss timer.
    pub fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
        self.tick = 0;
    }

    /// Set a success message, resetting the dismiss timer.
    pub fn set_success(&mut self, msg: String) {
        self.success = Some(msg);
        self.tick = 0;
    }

    /// Increment the tick counter (called once per frame).
    pub fn inc_tick(&mut self) {
        self.tick += 1;
    }

    /// Check if auto-dismiss threshold is reached.
    pub fn should_dismiss(&self) -> bool {
        self.tick >= crate::constants::AUTO_DISMISS_TICKS
    }

    /// Clear both toasts and reset tick.
    pub fn dismiss(&mut self) {
        self.error = None;
        self.success = None;
        self.tick = 0;
    }

    /// Returns true if any toast is active.
    pub fn has_toast(&self) -> bool {
        self.error.is_some() || self.success.is_some()
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
    pub toast: ToastState,
    pub show_help: bool,
    pub should_quit: bool,
    /// General tick counter for periodic UI updates.
    pub tick: u64,
    /// GitHub API rate limit remaining requests (from X-RateLimit-Remaining header).
    pub rate_remaining: Option<u32>,
    /// GitHub API rate limit reset time in seconds (from X-RateLimit-Reset header).
    pub rate_reset_secs: Option<u64>,
    pub client: Arc<GitHubClient>,
    pub confirmation: Option<ConfirmState>,
}

impl App {
    pub fn new(config: Config, token: Option<String>) -> color_eyre::Result<Self> {
        let theme = Theme::from_config(&config.theme);

        let (client, state) = match token {
            Some(token) => (Arc::new(GitHubClient::new(token)?), AppState::Main),
            None => (Arc::new(GitHubClient::new(String::new())?), AppState::Login),
        };

        // Apply config defaults
        let mut panels = HashMap::new();
        panels.insert(PanelKind::Repos, PanelState::Repos(ListPanel::default()));
        panels.insert(
            PanelKind::Pulls,
            PanelState::Pulls(ListPanel {
                data: ListPanelData::new(),
                selection: SelectionState::table(),
                detail_pane: DetailPane::new(),
                item: PullsPanelItem::new(FilterState::from(config.defaults.pulls_state.as_str())),
            }),
        );
        panels.insert(
            PanelKind::Issues,
            PanelState::Issues(ListPanel {
                data: ListPanelData::new(),
                selection: SelectionState::list(),
                detail_pane: DetailPane::new(),
                item: IssuesPanelItem {
                    state_filter: FilterState::from(config.defaults.issues_state.as_str()),
                },
            }),
        );
        panels.insert(
            PanelKind::Notifications,
            PanelState::Notifications(ListPanel {
                data: ListPanelData::new(),
                selection: SelectionState::list(),
                detail_pane: DetailPane::new(),
                item: NotificationsPanelItem {
                    show_all: config.defaults.notifications_show_all,
                },
            }),
        );
        panels.insert(
            PanelKind::Actions,
            PanelState::Actions(ListPanel::default()),
        );

        Ok(Self {
            state,
            login: None,
            active_panel: PanelKind::Repos,
            panels,
            config,
            theme,
            search: SearchState::default(),
            toast: ToastState::default(),
            show_help: false,
            should_quit: false,
            tick: 0,
            rate_remaining: None,
            rate_reset_secs: None,
            client,
            confirmation: None,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_app;


    #[test]
    fn new_app_starts_on_repos_panel() {
        let app = test_app();
        assert_eq!(app.active_panel, PanelKind::Repos);
    }

    #[test]
    fn tab_cycles_through_panels() {
        let mut app = test_app();
        let panels = PanelKind::ALL;

        for expected in &panels[1..] {
            app.next_tab();
            assert_eq!(app.active_panel, *expected);
        }
        // Wraps back to start
        app.next_tab();
        assert_eq!(app.active_panel, PanelKind::Repos);
    }

    #[test]
    fn prev_tab_wraps_around() {
        let mut app = test_app();
        assert_eq!(app.active_panel, PanelKind::Repos);
        app.prev_tab();
        assert_eq!(app.active_panel, PanelKind::Actions);
    }

    #[test]
    fn app_constructs_with_token() {
        let _app = test_app();
        // Token passed during construction — App created successfully with auth
    }

    #[test]
    fn app_constructs_without_token() {
        let app = App::new(Config::default(), None).unwrap();
        assert!(matches!(app.state, AppState::Login));
    }
}
