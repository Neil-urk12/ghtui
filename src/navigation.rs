use std::collections::HashMap;

use crate::app::{PanelEntry, PanelKind};
use crate::cmd::Cmd;

/// Wire all downstream panels when opening a repo from the Repos panel.
/// Sets owner/repo on PullsPanel, ActionsPanel, and IssuesPanel,
/// then switches to Pulls tab and returns a FetchPulls command.
pub fn handle_open_repo_detail(
    panels: &mut HashMap<PanelKind, PanelEntry>,
    owner: String,
    repo: String,
) -> Cmd {
    for entry in panels.values_mut() {
        entry.panel.as_panel_mut().set_repo_context(&owner, &repo);
    }
    Cmd::FetchPulls {
        owner,
        repo,
        state: "open".to_string(),
    }
}