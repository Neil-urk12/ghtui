use std::collections::HashMap;

use crate::app::{App, PanelKind, PanelState};
use crate::event::ApiPayload;

/// Shared owner/repo context used by multiple Cmd variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoContext {
    pub owner: String,
    pub repo: String,
}

impl RepoContext {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
        }
    }
}

/// Behavior classification for commands.
/// Eliminates the manual sync invariant between `is_destructive()` and `action_label()`.
pub enum CmdBehavior {
    /// Command mutates state and requires user confirmation.
    Destructive { label: &'static str },
    /// Command is safe to execute without confirmation.
    Benign,
}

#[derive(Debug)]
pub enum Cmd {
    None,
    FetchRepos,
    FetchPulls {
        ctx: RepoContext,
        state: String,
    },
    FetchIssues {
        ctx: RepoContext,
        state: String,
    },
    FetchNotifications {
        all: bool,
    },
    FetchActions {
        ctx: RepoContext,
    },
    MarkNotificationRead {
        id: String,
    },
    MarkAllNotificationsRead,
    ApprovePr {
        ctx: RepoContext,
        pr_number: u64,
    },
    MergePr {
        ctx: RepoContext,
        pr_number: u64,
    },
    RerunWorkflow {
        ctx: RepoContext,
        run_id: u64,
    },
    CancelWorkflow {
        ctx: RepoContext,
        run_id: u64,
    },
    OpenRepoDetail {
        ctx: RepoContext,
    },
    CycleRepoSort,
    FetchPrComments {
        ctx: RepoContext,
        pr_number: u64,
    },
    ClosePr {
        ctx: RepoContext,
        pr_number: u64,
    },
    ReopenPr {
        ctx: RepoContext,
        pr_number: u64,
    },
}

impl Cmd {
    /// Returns the behavior classification for this command.
    /// Destructive commands carry a human-readable label for confirmation dialogs.
    pub fn behavior(&self) -> CmdBehavior {
        match self {
            Cmd::MergePr { .. } => CmdBehavior::Destructive {
                label: "Merge Pull Request",
            },
            Cmd::ApprovePr { .. } => CmdBehavior::Destructive {
                label: "Approve Pull Request",
            },
            Cmd::CancelWorkflow { .. } => CmdBehavior::Destructive {
                label: "Cancel Workflow",
            },
            Cmd::RerunWorkflow { .. } => CmdBehavior::Destructive {
                label: "Rerun Workflow",
            },
            Cmd::MarkAllNotificationsRead => CmdBehavior::Destructive {
                label: "Mark All Notifications as Read",
            },
            Cmd::ClosePr { .. } => CmdBehavior::Destructive {
                label: "Close Pull Request",
            },
            Cmd::ReopenPr { .. } => CmdBehavior::Destructive {
                label: "Reopen Pull Request",
            },
            _ => CmdBehavior::Benign,
        }
    }
}

#[derive(Default)]
pub struct CmdState {
    next_id: u64,
    pending: HashMap<u64, Origin>,
}

#[derive(Debug)]
pub struct CmdTicket(u64);

pub struct PendingCmd {
    pub cmd: Cmd,
    pub ticket: CmdTicket,
}

#[derive(Debug)]
pub struct CmdCompletion {
    ticket: CmdTicket,
    pub result: Result<ApiPayload, String>,
}

impl CmdTicket {
    pub fn finish(self, result: Result<ApiPayload, String>) -> CmdCompletion {
        CmdCompletion {
            ticket: self,
            result,
        }
    }
}

#[derive(PartialEq, Eq)]
enum RequestKind {
    List,
    Details(u64),
    Mutation,
}

struct Origin {
    panel: PanelKind,
    ctx: Option<RepoContext>,
    revision: u64,
    kind: RequestKind,
}

impl Origin {
    fn is_current(&self, panels: &HashMap<PanelKind, PanelState>) -> bool {
        panels.get(&self.panel).is_some_and(|panel| {
            let (ctx, revision) = panel.data_context();
            self.ctx == ctx && (self.kind == RequestKind::Mutation || self.revision == revision)
        })
    }
}

pub fn start(app: &mut App, cmd: Cmd) -> Option<PendingCmd> {
    use PanelKind::*;
    use RequestKind::*;

    let (panel, ctx, kind) = match &cmd {
        Cmd::None | Cmd::OpenRepoDetail { .. } | Cmd::CycleRepoSort => return None,
        Cmd::FetchRepos => (Repos, None, List),
        Cmd::FetchPulls { ctx, .. } => (Pulls, Some(ctx.clone()), List),
        Cmd::FetchIssues { ctx, .. } => (Issues, Some(ctx.clone()), List),
        Cmd::FetchActions { ctx } => (Actions, Some(ctx.clone()), List),
        Cmd::FetchNotifications { .. } => (Notifications, None, List),
        Cmd::FetchPrComments { ctx, pr_number } => (Pulls, Some(ctx.clone()), Details(*pr_number)),
        Cmd::ApprovePr { ctx, .. }
        | Cmd::MergePr { ctx, .. }
        | Cmd::ClosePr { ctx, .. }
        | Cmd::ReopenPr { ctx, .. } => (Pulls, Some(ctx.clone()), Mutation),
        Cmd::RerunWorkflow { ctx, .. } | Cmd::CancelWorkflow { ctx, .. } => {
            (Actions, Some(ctx.clone()), Mutation)
        }
        Cmd::MarkNotificationRead { .. } | Cmd::MarkAllNotificationsRead => {
            (Notifications, None, Mutation)
        }
    };
    let (_, revision) = app.panels.get(&panel)?.data_context();
    let origin = Origin {
        panel,
        ctx,
        revision,
        kind,
    };
    if origin.kind != Mutation && !origin.is_current(&app.panels) {
        return None;
    }

    // shortcut: superseded reads can still cache results; coordinate cache writes if freshness must follow request order.
    app.commands.pending.retain(|_, previous| {
        previous.kind == Mutation
            || (previous.is_current(&app.panels)
                && !(previous.panel == panel && previous.kind == origin.kind))
    });
    app.commands.next_id += 1;
    let ticket = CmdTicket(app.commands.next_id);
    app.commands.pending.insert(ticket.0, origin);
    sync_loading(app, panel);
    Some(PendingCmd { cmd, ticket })
}

pub fn complete(app: &mut App, completion: CmdCompletion) -> Cmd {
    let Some(origin) = app.commands.pending.remove(&completion.ticket.0) else {
        return Cmd::None;
    };
    let current = origin.is_current(&app.panels);
    if origin.kind != RequestKind::Mutation && !current {
        sync_loading(app, origin.panel);
        return Cmd::None;
    }

    let mut refresh = Cmd::None;
    match completion.result {
        Ok(ApiPayload::Success(msg)) => {
            app.success_toast = Some(msg);
            app.toast_tick = 0;
            if current {
                refresh = app.panels[&origin.panel].fetch_cmd();
            }
        }
        Ok(ApiPayload::PrComments {
            pr_number,
            comments,
            reviews,
            code_comments,
            files,
        }) => {
            app.panels
                .get_mut(&origin.panel)
                .unwrap()
                .update_pr_comments(pr_number, comments, reviews, code_comments, files);
        }
        Ok(payload) => {
            app.panels
                .get_mut(&origin.panel)
                .unwrap()
                .load_data(payload);
        }
        Err(msg) => {
            app.error_toast = Some(msg);
            app.toast_tick = 0;
        }
    }
    app.commands.pending.retain(|_, pending| {
        pending.kind == RequestKind::Mutation || pending.is_current(&app.panels)
    });
    sync_loading(app, origin.panel);
    refresh
}

fn sync_loading(app: &mut App, panel: PanelKind) {
    let loading = app.commands.pending.values().any(|pending| {
        pending.panel == panel
            && !matches!(pending.kind, RequestKind::Details(_))
            && pending.is_current(&app.panels)
    });
    if let Some(entry) = app.panels.get_mut(&panel) {
        entry.set_loading(loading);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::domain::{FilterState, PrComment, PullRequest, Repo};
    use crate::panels::panel::ListPanel;
    use crate::panels::PullsPanelItem;
    use crate::update::{update, Message};

    fn test_app() -> App {
        let mut app = App::new(Config::default(), Some("tok".into())).unwrap();
        for kind in [PanelKind::Pulls, PanelKind::Issues, PanelKind::Actions] {
            app.panels
                .get_mut(&kind)
                .unwrap()
                .set_repo_context("o", "r");
        }
        app.panels
            .get_mut(&PanelKind::Pulls)
            .unwrap()
            .load_data(pull_list("original"));
        app
    }

    fn pulls(app: &App) -> &ListPanel<PullsPanelItem> {
        let PanelState::Pulls(panel) = &app.panels[&PanelKind::Pulls] else {
            panic!("Expected Pulls panel");
        };
        panel
    }

    fn pull_list(title: &str) -> ApiPayload {
        ApiPayload::Pulls(
            (1..=2)
                .map(|number| PullRequest {
                    number,
                    title: title.into(),
                    state: "open".into(),
                    ..Default::default()
                })
                .collect(),
        )
    }

    fn fetch() -> Cmd {
        Cmd::FetchPulls {
            ctx: RepoContext::new("o", "r"),
            state: "open".into(),
        }
    }

    fn detail(number: u64) -> Cmd {
        Cmd::FetchPrComments {
            ctx: RepoContext::new("o", "r"),
            pr_number: number,
        }
    }

    fn detail_result(number: u64, body: &str) -> ApiPayload {
        ApiPayload::PrComments {
            pr_number: number,
            comments: Ok(vec![PrComment {
                body: body.into(),
                ..Default::default()
            }]),
            reviews: Ok(vec![]),
            code_comments: Ok(vec![]),
            files: Ok(vec![]),
        }
    }

    fn mutation() -> Cmd {
        Cmd::MergePr {
            ctx: RepoContext::new("o", "r"),
            pr_number: 1,
        }
    }

    #[test]
    fn failed_refresh_preserves_items_selection_filter_and_scroll() {
        let mut app = test_app();
        let PanelState::Pulls(panel) = app.panels.get_mut(&PanelKind::Pulls).unwrap() else {
            unreachable!();
        };
        panel.selection.select(Some(1));
        panel.data.filtered_indices = vec![1, 0];
        panel.detail_pane.scroll = 9;
        let pending = start(&mut app, fetch()).unwrap();
        assert!(pulls(&app).data.loading);
        complete(&mut app, pending.ticket.finish(Err("failed".into())));
        assert!(!pulls(&app).data.loading);
        assert_eq!(pulls(&app).data.items[0].title, "original");
        assert_eq!(pulls(&app).data.filtered_indices, vec![1, 0]);
        assert_eq!(pulls(&app).selection.selected(), Some(1));
        assert_eq!(pulls(&app).detail_pane.scroll, 9);
        assert_eq!(app.error_toast.as_deref(), Some("failed"));
    }

    #[test]
    fn latest_list_request_wins_in_both_completion_orders_even_on_failure() {
        for newest_first in [false, true] {
            for newest_fails in [false, true] {
                let mut app = test_app();
                let older = start(&mut app, fetch())
                    .unwrap()
                    .ticket
                    .finish(Ok(pull_list("older")));
                let latest = start(&mut app, fetch())
                    .unwrap()
                    .ticket
                    .finish(if newest_fails {
                        Err("latest failed".into())
                    } else {
                        Ok(pull_list("latest"))
                    });
                if newest_first {
                    complete(&mut app, latest);
                    complete(&mut app, older);
                } else {
                    complete(&mut app, older);
                    assert!(pulls(&app).data.loading);
                    assert_eq!(pulls(&app).data.items[0].title, "original");
                    complete(&mut app, latest);
                }
                assert!(!pulls(&app).data.loading);
                assert_eq!(
                    pulls(&app).data.items[0].title,
                    if newest_fails { "original" } else { "latest" }
                );
                assert_eq!(
                    app.error_toast.as_deref(),
                    newest_fails.then_some("latest failed")
                );
            }
        }
    }

    #[test]
    fn filter_change_supersedes_old_read_and_its_error() {
        let mut app = test_app();
        let old = start(&mut app, fetch()).unwrap();
        app.active_panel = PanelKind::Pulls;
        let filtered = update(&mut app, Message::PanelKeyInput('2'));
        assert!(matches!(&filtered, Cmd::FetchPulls { state, .. } if state == "closed"));
        let new = start(&mut app, filtered).unwrap();
        complete(&mut app, old.ticket.finish(Err("obsolete error".into())));
        assert!(app.error_toast.is_none());
        assert!(pulls(&app).data.loading);
        complete(&mut app, new.ticket.finish(Ok(pull_list("filtered"))));
        assert_eq!(pulls(&app).data.items[0].title, "filtered");
        assert!(!pulls(&app).data.loading);
    }

    #[test]
    fn mutation_after_tab_switch_refreshes_origin_with_current_filter() {
        let mut app = test_app();
        app.active_panel = PanelKind::Pulls;
        let pending = start(&mut app, mutation()).unwrap();
        let PanelState::Pulls(panel) = app.panels.get_mut(&PanelKind::Pulls).unwrap() else {
            unreachable!();
        };
        panel.item.state_filter = FilterState::Closed;
        app.active_panel = PanelKind::Issues;
        let refresh = update(
            &mut app,
            Message::CmdCompleted(
                pending
                    .ticket
                    .finish(Ok(ApiPayload::Success("merged".into()))),
            ),
        );
        assert!(matches!(refresh, Cmd::FetchPulls { ctx, state }
            if ctx == RepoContext::new("o", "r") && state == "closed"));
        assert_eq!(app.active_panel, PanelKind::Issues);
        assert_eq!(app.success_toast.as_deref(), Some("merged"));
        assert!(!pulls(&app).data.loading);
    }

    #[test]
    fn mutation_after_repo_switch_reports_outcome_without_refreshing_new_repo() {
        for succeeds in [false, true] {
            let mut app = test_app();
            let pending = start(&mut app, mutation()).unwrap();
            app.panels
                .get_mut(&PanelKind::Pulls)
                .unwrap()
                .set_repo_context("o", "other");
            let read = start(
                &mut app,
                Cmd::FetchPulls {
                    ctx: RepoContext::new("o", "other"),
                    state: "open".into(),
                },
            )
            .unwrap();
            let result = if succeeds {
                Ok(ApiPayload::Success("merged".into()))
            } else {
                Err("merge failed".into())
            };
            assert!(matches!(
                complete(&mut app, pending.ticket.finish(result)),
                Cmd::None
            ));
            assert!(pulls(&app).data.loading);
            if succeeds {
                assert_eq!(app.success_toast.as_deref(), Some("merged"));
            } else {
                assert_eq!(app.error_toast.as_deref(), Some("merge failed"));
            }
            complete(&mut app, read.ticket.finish(Ok(pull_list("new repo"))));
            assert!(!pulls(&app).data.loading);
        }
    }

    #[test]
    fn multiple_mutations_and_list_read_keep_loading_until_relevant_work_finishes() {
        let mut app = test_app();
        let first = start(&mut app, mutation()).unwrap();
        let second = start(&mut app, mutation()).unwrap();
        let refresh = complete(
            &mut app,
            first.ticket.finish(Ok(ApiPayload::Success("done".into()))),
        );
        assert!(pulls(&app).data.loading);
        let read = start(&mut app, refresh).unwrap();
        complete(&mut app, read.ticket.finish(Ok(pull_list("refreshed"))));
        assert!(pulls(&app).data.loading);
        complete(&mut app, second.ticket.finish(Err("second failed".into())));
        assert!(!pulls(&app).data.loading);
        assert_eq!(pulls(&app).data.items[0].title, "refreshed");
    }

    #[test]
    fn detail_results_update_unselected_rows_without_ending_list_loading() {
        let mut app = test_app();
        let first = start(&mut app, detail(1)).unwrap();
        app.active_panel = PanelKind::Pulls;
        update(&mut app, Message::PanelKeyInput('j'));
        assert_eq!(pulls(&app).selection.selected(), Some(1));
        let second = start(&mut app, detail(2)).unwrap();
        let list = start(&mut app, fetch()).unwrap();
        complete(
            &mut app,
            second.ticket.finish(Ok(detail_result(2, "second"))),
        );
        complete(&mut app, first.ticket.finish(Ok(detail_result(1, "first"))));
        assert_eq!(
            pulls(&app).data.items[0].comments.as_ref().unwrap()[0].body,
            "first"
        );
        assert_eq!(
            pulls(&app).data.items[1].comments.as_ref().unwrap()[0].body,
            "second"
        );
        assert!(pulls(&app).data.loading);
        complete(&mut app, list.ticket.finish(Err("list failed".into())));
        assert!(!pulls(&app).data.loading);
        assert_eq!(
            pulls(&app).data.items[0].comments.as_ref().unwrap()[0].body,
            "first"
        );
    }

    #[test]
    fn newest_detail_request_wins_per_pr() {
        let mut app = test_app();
        let old = start(&mut app, detail(1)).unwrap();
        let new = start(&mut app, detail(1)).unwrap();
        let other = start(&mut app, detail(2)).unwrap();
        complete(&mut app, new.ticket.finish(Ok(detail_result(1, "new"))));
        complete(&mut app, old.ticket.finish(Ok(detail_result(1, "old"))));
        complete(&mut app, other.ticket.finish(Ok(detail_result(2, "other"))));
        assert_eq!(
            pulls(&app).data.items[0].comments.as_ref().unwrap()[0].body,
            "new"
        );
        assert_eq!(
            pulls(&app).data.items[1].comments.as_ref().unwrap()[0].body,
            "other"
        );
    }

    #[test]
    fn list_replacement_invalidates_previous_detail_reads() {
        let mut app = test_app();
        let old = start(&mut app, detail(1)).unwrap();
        let list = start(&mut app, fetch()).unwrap();
        complete(&mut app, list.ticket.finish(Ok(pull_list("new list"))));
        let new = start(&mut app, detail(1)).unwrap();
        complete(
            &mut app,
            new.ticket.finish(Ok(detail_result(1, "new details"))),
        );
        complete(
            &mut app,
            old.ticket.finish(Ok(detail_result(1, "old details"))),
        );
        assert_eq!(
            pulls(&app).data.items[0].comments.as_ref().unwrap()[0].body,
            "new details"
        );
    }

    #[test]
    fn repo_changes_reject_old_reads_even_without_starting_another_request() {
        let mut app = test_app();
        let list = start(&mut app, fetch()).unwrap();
        let details = start(&mut app, detail(1)).unwrap();
        let entry = app.panels.get_mut(&PanelKind::Pulls).unwrap();
        entry.set_repo_context("o", "other");
        entry.load_data(pull_list("other repo"));
        complete(
            &mut app,
            details.ticket.finish(Ok(detail_result(1, "wrong repo"))),
        );
        assert!(pulls(&app).data.items[0].comments.is_none());
        let entry = app.panels.get_mut(&PanelKind::Pulls).unwrap();
        entry.set_repo_context("o", "r");
        entry.load_data(pull_list("returned"));
        complete(&mut app, list.ticket.finish(Ok(pull_list("old visit"))));
        assert_eq!(pulls(&app).data.items[0].title, "returned");
        assert!(app.error_toast.is_none());
        assert!(!pulls(&app).data.loading);
    }

    #[test]
    fn mutation_loading_starts_after_confirmation_and_cancellation_keeps_rows() {
        let mut app = test_app();
        app.active_panel = PanelKind::Pulls;
        assert!(matches!(
            update(&mut app, Message::PanelKeyInput('m')),
            Cmd::None
        ));
        assert!(app.confirmation.is_some());
        assert!(!pulls(&app).data.loading);
        update(&mut app, Message::ConfirmNo);
        assert!(app.confirmation.is_none());
        assert_eq!(pulls(&app).data.items[0].title, "original");
        update(&mut app, Message::PanelKeyInput('m'));
        let cmd = update(&mut app, Message::ConfirmYes);
        assert!(matches!(cmd, Cmd::MergePr { pr_number: 1, .. }));
        let pending = start(&mut app, cmd).unwrap();
        assert!(pulls(&app).data.loading);
        complete(&mut app, pending.ticket.finish(Err("merge failed".into())));
        assert!(!pulls(&app).data.loading);
        assert_eq!(pulls(&app).data.items[0].title, "original");
    }

    #[test]
    fn repo_switch_clears_scoped_rows_and_rejects_late_results_after_switching_back() {
        let mut app = test_app();
        app.panels
            .get_mut(&PanelKind::Repos)
            .unwrap()
            .load_data(ApiPayload::Repos(vec![Repo {
                name: "o/other".into(),
                ..Default::default()
            }]));
        app.panels
            .get_mut(&PanelKind::Issues)
            .unwrap()
            .load_data(ApiPayload::Issues(vec![Default::default()]));
        app.panels
            .get_mut(&PanelKind::Actions)
            .unwrap()
            .load_data(ApiPayload::Actions(vec![Default::default()]));
        app.panels
            .get_mut(&PanelKind::Notifications)
            .unwrap()
            .load_data(ApiPayload::Notifications(vec![Default::default()]));
        let old_list = start(&mut app, fetch()).unwrap();
        let old_detail = start(&mut app, detail(1)).unwrap();
        let switched = update(&mut app, Message::PanelKeyInput('\n'));
        assert!(pulls(&app).data.items.is_empty());
        assert_eq!(pulls(&app).selection.selected(), None);
        let PanelState::Issues(issues) = &app.panels[&PanelKind::Issues] else {
            unreachable!();
        };
        let PanelState::Actions(actions) = &app.panels[&PanelKind::Actions] else {
            unreachable!();
        };
        let PanelState::Repos(repos) = &app.panels[&PanelKind::Repos] else {
            unreachable!();
        };
        let PanelState::Notifications(notifications) = &app.panels[&PanelKind::Notifications]
        else {
            unreachable!();
        };
        assert!(issues.data.items.is_empty() && actions.data.items.is_empty());
        assert_eq!(repos.data.items.len(), 1);
        assert_eq!(notifications.data.items.len(), 1);
        assert!(matches!(
            update(&mut app, Message::PanelKeyInput('m')),
            Cmd::None
        ));
        assert!(app.confirmation.is_none());
        let new_list = start(&mut app, switched).unwrap();
        complete(
            &mut app,
            new_list.ticket.finish(Err("new repo failed".into())),
        );
        assert!(!pulls(&app).data.loading && pulls(&app).data.items.is_empty());
        app.panels
            .get_mut(&PanelKind::Pulls)
            .unwrap()
            .set_repo_context("o", "r");
        app.panels
            .get_mut(&PanelKind::Pulls)
            .unwrap()
            .load_data(pull_list("returned"));
        complete(&mut app, old_list.ticket.finish(Ok(pull_list("obsolete"))));
        complete(
            &mut app,
            old_detail.ticket.finish(Ok(detail_result(1, "obsolete"))),
        );
        assert_eq!(pulls(&app).data.items[0].title, "returned");
        assert!(pulls(&app).data.items[0].comments.is_none());
        assert_eq!(app.error_toast.as_deref(), Some("new repo failed"));
    }

    #[test]
    fn reopening_same_repo_preserves_rows_selection_scroll_and_detail_validity() {
        let mut app = test_app();
        let PanelState::Pulls(panel) = app.panels.get_mut(&PanelKind::Pulls).unwrap() else {
            unreachable!();
        };
        panel.selection.select(Some(1));
        panel.detail_pane.scroll = 7;
        panel.item.state_filter = FilterState::Closed;
        let pending = start(&mut app, detail(1)).unwrap();
        app.panels
            .get_mut(&PanelKind::Repos)
            .unwrap()
            .load_data(ApiPayload::Repos(vec![Repo {
                name: "o/r".into(),
                ..Default::default()
            }]));
        let fetch = update(&mut app, Message::PanelKeyInput('\n'));
        assert!(matches!(fetch, Cmd::FetchPulls { state, .. } if state == "closed"));
        assert_eq!(pulls(&app).selection.selected(), Some(1));
        assert_eq!(pulls(&app).detail_pane.scroll, 7);
        complete(
            &mut app,
            pending.ticket.finish(Ok(detail_result(1, "retained"))),
        );
        assert_eq!(
            pulls(&app).data.items[0].comments.as_ref().unwrap()[0].body,
            "retained"
        );
    }

    #[test]
    fn global_notification_mutation_refreshes_current_notification_filter() {
        let mut app = test_app();
        let pending = start(&mut app, Cmd::MarkNotificationRead { id: "1".into() }).unwrap();
        let PanelState::Notifications(panel) =
            app.panels.get_mut(&PanelKind::Notifications).unwrap()
        else {
            unreachable!();
        };
        panel.item.show_all = true;
        let refresh = complete(
            &mut app,
            pending
                .ticket
                .finish(Ok(ApiPayload::Success("read".into()))),
        );
        assert!(matches!(refresh, Cmd::FetchNotifications { all: true }));
        assert_eq!(app.active_panel, PanelKind::Repos);
    }

    #[test]
    fn destructive_cmds_carry_labels() {
        let cases: Vec<(Cmd, &str)> = vec![
            (
                Cmd::MergePr {
                    ctx: RepoContext::new("o", "r"),
                    pr_number: 1,
                },
                "Merge Pull Request",
            ),
            (
                Cmd::ApprovePr {
                    ctx: RepoContext::new("o", "r"),
                    pr_number: 1,
                },
                "Approve Pull Request",
            ),
            (
                Cmd::CancelWorkflow {
                    ctx: RepoContext::new("o", "r"),
                    run_id: 1,
                },
                "Cancel Workflow",
            ),
            (
                Cmd::RerunWorkflow {
                    ctx: RepoContext::new("o", "r"),
                    run_id: 1,
                },
                "Rerun Workflow",
            ),
            (Cmd::MarkAllNotificationsRead, "Mark All Notifications as Read"),
            (
                Cmd::ClosePr {
                    ctx: RepoContext::new("o", "r"),
                    pr_number: 1,
                },
                "Close Pull Request",
            ),
            (
                Cmd::ReopenPr {
                    ctx: RepoContext::new("o", "r"),
                    pr_number: 1,
                },
                "Reopen Pull Request",
            ),
        ];

        for (cmd, expected_label) in cases {
            match cmd.behavior() {
                CmdBehavior::Destructive { label } => {
                    assert_eq!(label, expected_label, "{:?}", cmd);
                }
                CmdBehavior::Benign => panic!("{:?} should be Destructive", cmd),
            }
        }
    }

    #[test]
    fn benign_cmds_return_benign() {
        let cmds = vec![
            Cmd::None,
            Cmd::FetchRepos,
            Cmd::FetchPulls {
                ctx: RepoContext::new("o", "r"),
                state: "open".into(),
            },
            Cmd::FetchIssues {
                ctx: RepoContext::new("o", "r"),
                state: "open".into(),
            },
            Cmd::FetchNotifications { all: false },
            Cmd::FetchActions {
                ctx: RepoContext::new("o", "r"),
            },
            Cmd::MarkNotificationRead { id: "1".into() },
            Cmd::FetchPrComments {
                ctx: RepoContext::new("o", "r"),
                pr_number: 1,
            },
            Cmd::OpenRepoDetail {
                ctx: RepoContext::new("o", "r"),
            },
            Cmd::CycleRepoSort,
        ];

        for cmd in cmds {
            assert!(
                matches!(cmd.behavior(), CmdBehavior::Benign),
                "{:?} should be Benign",
                cmd
            );
        }
    }

    #[test]
    fn repo_context_new_from_str_and_string() {
        let ctx = RepoContext::new("o", String::from("r"));
        assert_eq!(ctx.owner, "o");
        assert_eq!(ctx.repo, "r");
    }

    #[test]
    fn repo_context_equality() {
        assert_eq!(RepoContext::new("a", "b"), RepoContext::new("a", "b"));
        assert_ne!(RepoContext::new("a", "b"), RepoContext::new("a", "c"));
    }
}
