use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::api::client::GitHubClient;
use crate::config::Config;
use crate::domain::FilterState;
use crate::panels::login::LoginState;
use crate::panels::panel::{ListPanel, ListPanelData, Panel, SelectionState};
use crate::panels::{
    ActionsPanelItem, IssuesPanelItem, NotificationsPanelItem, PullsPanelItem, ReposPanelItem,
};
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
    /// Why the first device-code request failed; cleared to retry.
    pub login_error: Option<String>,
    pub active_panel: PanelKind,
    pub panels: HashMap<PanelKind, Box<dyn Panel>>,
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
        let mut panels: HashMap<PanelKind, Box<dyn Panel>> = HashMap::new();
        panels.insert(
            PanelKind::Repos,
            Box::new(ListPanel::<ReposPanelItem>::default()),
        );
        panels.insert(
            PanelKind::Pulls,
            Box::new(ListPanel {
                data: ListPanelData::new(),
                selection: SelectionState::table(),
                detail_pane: DetailPane::new(),
                item: PullsPanelItem::new(FilterState::from(config.defaults.pulls_state.as_str())),
            }),
        );
        panels.insert(
            PanelKind::Issues,
            Box::new(ListPanel {
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
            Box::new(ListPanel {
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
            Box::new(ListPanel::<ActionsPanelItem>::default()),
        );

        Ok(Self {
            state,
            login: None,
            login_error: None,
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

    #[allow(dead_code)]
    pub fn panel<T: 'static>(&self, kind: PanelKind) -> Option<&T> {
        self.panels.get(&kind)?.as_any().downcast_ref::<T>()
    }

    pub fn panel_mut<T: 'static>(&mut self, kind: PanelKind) -> Option<&mut T> {
        self.panels.get_mut(&kind)?.as_any_mut().downcast_mut::<T>()
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
