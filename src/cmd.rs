/// Shared owner/repo context used by multiple Cmd variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoContext {
    pub owner: String,
    pub repo: String,
}

use crate::domain::FilterState;

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


pub enum Cmd {
    None,
    FetchRepos,
    FetchPulls {
        ctx: RepoContext,
        state: FilterState,
    },
    FetchIssues {
        ctx: RepoContext,
        state: FilterState,
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

impl std::fmt::Debug for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::None => write!(f, "None"),
            Cmd::FetchRepos => write!(f, "FetchRepos"),
            Cmd::FetchPulls { ctx, state } => {
                write!(f, "FetchPulls({}, {}, {})", ctx.owner, ctx.repo, state.as_str())
            }
            Cmd::FetchIssues { ctx, state } => {
                write!(f, "FetchIssues({}, {}, {})", ctx.owner, ctx.repo, state.as_str())
            }
            Cmd::FetchNotifications { all } => write!(f, "FetchNotifications(all={})", all),
            Cmd::FetchActions { ctx } => write!(f, "FetchActions({}, {})", ctx.owner, ctx.repo),
            Cmd::MarkNotificationRead { id } => write!(f, "MarkNotificationRead({})", id),
            Cmd::MarkAllNotificationsRead => write!(f, "MarkAllNotificationsRead"),
            Cmd::ApprovePr { ctx, pr_number } => {
                write!(f, "ApprovePr({}/{}/{})", ctx.owner, ctx.repo, pr_number)
            }
            Cmd::MergePr { ctx, pr_number } => {
                write!(f, "MergePr({}/{}/{})", ctx.owner, ctx.repo, pr_number)
            }
            Cmd::RerunWorkflow { ctx, run_id } => {
                write!(f, "RerunWorkflow({}/{}/{})", ctx.owner, ctx.repo, run_id)
            }
            Cmd::CancelWorkflow { ctx, run_id } => {
                write!(f, "CancelWorkflow({}/{}/{})", ctx.owner, ctx.repo, run_id)
            }
            Cmd::OpenRepoDetail { ctx } => write!(f, "OpenRepoDetail({}/{})", ctx.owner, ctx.repo),
            Cmd::CycleRepoSort => write!(f, "CycleRepoSort"),
            Cmd::FetchPrComments { ctx, pr_number } => {
                write!(f, "FetchPrComments({}/{}/{})", ctx.owner, ctx.repo, pr_number)
            }
            Cmd::ClosePr { ctx, pr_number } => {
                write!(f, "ClosePr({}/{}/{})", ctx.owner, ctx.repo, pr_number)
            }
            Cmd::ReopenPr { ctx, pr_number } => {
                write!(f, "ReopenPr({}/{}/{})", ctx.owner, ctx.repo, pr_number)
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
                state: FilterState::Open,
            },
            Cmd::FetchIssues {
                ctx: RepoContext::new("o", "r"),
                state: FilterState::Open,
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
    fn debug_format_has_closing_paren() {
        let cmd = Cmd::FetchPrComments {
            ctx: RepoContext::new("o", "r"),
            pr_number: 1,
        };
        let debug = format!("{:?}", cmd);
        assert!(
            debug.ends_with(')'),
            "Debug should have closing paren: {}",
            debug
        );

        let cmd = Cmd::ClosePr {
            ctx: RepoContext::new("o", "r"),
            pr_number: 1,
        };
        let debug = format!("{:?}", cmd);
        assert!(
            debug.ends_with(')'),
            "Debug should have closing paren: {}",
            debug
        );

        let cmd = Cmd::ReopenPr {
            ctx: RepoContext::new("o", "r"),
            pr_number: 1,
        };
        let debug = format!("{:?}", cmd);
        assert!(
            debug.ends_with(')'),
            "Debug should have closing paren: {}",
            debug
        );
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
