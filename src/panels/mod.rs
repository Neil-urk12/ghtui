/// Generate an `extract_items` implementation that pattern-matches a single
/// `ApiPayload` variant and returns `Some(items)` / `None`.
macro_rules! extract_items {
    ($variant:ident) => {
        fn extract_items(payload: $crate::event::ApiPayload) -> Option<Vec<Self::Item>> {
            if let $crate::event::ApiPayload::$variant(items) = payload {
                Some(items)
            } else {
                None
            }
        }
    };
}

pub mod actions;
pub mod issues;
pub mod login;
pub mod notifications;
pub mod panel;
pub mod pulls;
pub mod repos;

pub use actions::ActionsPanelItem;
pub use issues::IssuesPanelItem;
pub use notifications::NotificationsPanelItem;
#[allow(unused_imports)]
pub use panel::{ActionsPanel, IssuesPanel, NotificationsPanel, Panel, PullsPanel, ReposPanel};
pub use pulls::PullsPanelItem;
pub use repos::ReposPanelItem;
