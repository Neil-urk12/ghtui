use std::collections::HashMap;

use crate::app::{PanelKind, PanelState};
use crate::cmd::Cmd;

/// Wire all downstream panels when opening a repo from the Repos panel.
/// Sets owner/repo on PullsPanel, ActionsPanel, and IssuesPanel,
/// then switches to Pulls tab and returns a FetchPulls command.
pub fn handle_open_repo_detail(
    panels: &mut HashMap<PanelKind, PanelState>,
    owner: String,
    repo: String,
) -> Cmd {
    // Wire PullsPanel
    if let Some(PanelState::Pulls(ref mut pulls)) = panels.get_mut(&PanelKind::Pulls) {
        pulls.set_repo(owner.clone(), repo.clone());
    }
    // Wire ActionsPanel
    if let Some(PanelState::Actions(ref mut actions)) = panels.get_mut(&PanelKind::Actions) {
        actions.owner = Some(owner.clone());
        actions.repo = Some(repo.clone());
    }
    // Wire IssuesPanel
    if let Some(PanelState::Issues(ref mut issues)) = panels.get_mut(&PanelKind::Issues) {
        issues.set_repo(owner.clone(), repo.clone());
    }
    Cmd::FetchPulls {
        owner,
        repo,
        state: "open".to_string(),
    }
}