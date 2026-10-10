use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::api::client::GitHubClient;
use crate::config::Config;
use crate::domain::{FilterState, PrCodeComment, PrComment, PrFile, PrReview};
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
    panel_delegate!(fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect, theme: &crate::theme::Theme, tick: u64));
    panel_delegate!(fn fetch_cmd(&self) -> crate::cmd::Cmd);
    panel_delegate!(fn set_loading(&mut self, loading: bool));
    panel_delegate!(fn set_repo_context(&mut self, owner: &str, repo: &str));
    panel_delegate!(fn data_context(&self) -> (Option<crate::cmd::RepoContext>, u64));
    panel_delegate!(fn load_data(&mut self, payload: crate::event::ApiPayload));
    panel_delegate!(fn scroll_detail_up(&mut self));
    panel_delegate!(fn scroll_detail_down(&mut self));
    panel_delegate!(fn selected_display(&self) -> Option<String>);

    /// Update PR with comment data (Pulls panel only).
    pub fn update_pr_comments(
        &mut self,
        pr_number: u64,
        comments: Result<Vec<PrComment>, String>,
        reviews: Result<Vec<PrReview>, String>,
        code_comments: Result<Vec<PrCodeComment>, String>,
        files: Result<Vec<PrFile>, String>,
    ) {
        if let PanelState::Pulls(p) = self {
            if let Some(pr) = p.data.items.iter_mut().find(|pr| pr.number == pr_number) {
                // On success: store data, clear error. On error: clear stale data, store error.
                (pr.comments, pr.comment_errors) = match comments {
                    Ok(v) => (Some(v), None),
                    Err(e) => (None, Some(e)),
                };
                (pr.reviews, pr.review_errors) = match reviews {
                    Ok(v) => (Some(v), None),
                    Err(e) => (None, Some(e)),
                };
                (pr.code_comments, pr.code_comment_errors) = match code_comments {
                    Ok(v) => (Some(v), None),
                    Err(e) => (None, Some(e)),
                };
                (pr.files, pr.file_errors) = match files {
                    Ok(v) => (Some(v), None),
                    Err(e) => (None, Some(e)),
                };
            }
        }
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
    pub commands: crate::cmd::CmdState,
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
            commands: crate::cmd::CmdState::default(),
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
    use crate::config::Config;

    fn test_app() -> App {
        App::new(Config::default(), Some("test-token".into())).unwrap()
    }

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
