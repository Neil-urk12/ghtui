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

impl std::fmt::Debug for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::None => write!(f, "None"),
            Cmd::FetchRepos => write!(f, "FetchRepos"),
            Cmd::FetchPulls { ctx, state } => {
                write!(f, "FetchPulls({}, {}, {})", ctx.owner, ctx.repo, state)
            }
            Cmd::FetchIssues { ctx, state } => {
                write!(f, "FetchIssues({}, {}, {})", ctx.owner, ctx.repo, state)
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
    /// Returns true if this command is destructive and requires confirmation.
    /// Must stay in sync with `action_label()`.
    pub fn is_destructive(&self) -> bool {
        matches!(
            self,
            Cmd::MergePr { .. }
                | Cmd::ApprovePr { .. }
                | Cmd::CancelWorkflow { .. }
                | Cmd::RerunWorkflow { .. }
                | Cmd::MarkAllNotificationsRead
                | Cmd::ClosePr { .. }
                | Cmd::ReopenPr { .. }
        )
    }

    /// Human-readable action label for confirmation dialogs.
    /// Must stay in sync with `is_destructive()`.
    pub fn action_label(&self) -> Option<&'static str> {
        match self {
            Cmd::MergePr { .. } => Some("Merge Pull Request"),
            Cmd::ApprovePr { .. } => Some("Approve Pull Request"),
            Cmd::CancelWorkflow { .. } => Some("Cancel Workflow"),
            Cmd::RerunWorkflow { .. } => Some("Rerun Workflow"),
            Cmd::MarkAllNotificationsRead => Some("Mark All Notifications as Read"),
            Cmd::ClosePr { .. } => Some("Close Pull Request"),
            Cmd::ReopenPr { .. } => Some("Reopen Pull Request"),
            // Invariant: non-destructive cmds return None (tested by non_destructive_cmds_lack_labels)
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_label_returns_some_for_destructive_cmds() {
        // Each destructive variant should return Some with its expected label
        let merge = Cmd::MergePr {
            ctx: RepoContext::new("o", "r"),
            pr_number: 1,
        };
        assert_eq!(merge.action_label(), Some("Merge Pull Request"));

        let approve = Cmd::ApprovePr {
            ctx: RepoContext::new("o", "r"),
            pr_number: 1,
        };
        assert_eq!(approve.action_label(), Some("Approve Pull Request"));

        let cancel = Cmd::CancelWorkflow {
            ctx: RepoContext::new("o", "r"),
            run_id: 1,
        };
        assert_eq!(cancel.action_label(), Some("Cancel Workflow"));

        let rerun = Cmd::RerunWorkflow {
            ctx: RepoContext::new("o", "r"),
            run_id: 1,
        };
        assert_eq!(rerun.action_label(), Some("Rerun Workflow"));

        assert_eq!(
            Cmd::MarkAllNotificationsRead.action_label(),
            Some("Mark All Notifications as Read")
        );
    }

    #[test]
    fn action_label_returns_some_for_close_pr() {
        let close = Cmd::ClosePr {
            ctx: RepoContext::new("o", "r"),
            pr_number: 1,
        };
        assert_eq!(close.action_label(), Some("Close Pull Request"));
    }
    #[test]
    fn action_label_returns_none_for_non_destructive_cmds() {
        assert_eq!(Cmd::None.action_label(), None);
        assert_eq!(Cmd::FetchRepos.action_label(), None);

        assert_eq!(
            Cmd::FetchPulls {
                ctx: RepoContext::new("o", "r"),
                state: "open".into(),
            }
            .action_label(),
            None
        );

        assert_eq!(
            Cmd::FetchIssues {
                ctx: RepoContext::new("o", "r"),
                state: "open".into(),
            }
            .action_label(),
            None
        );

        assert_eq!(Cmd::FetchNotifications { all: false }.action_label(), None);

        assert_eq!(
            Cmd::FetchActions {
                ctx: RepoContext::new("o", "r"),
            }
            .action_label(),
            None
        );

        assert_eq!(
            Cmd::MarkNotificationRead { id: "1".into() }.action_label(),
            None
        );

        assert_eq!(
            Cmd::OpenRepoDetail {
                ctx: RepoContext::new("o", "r"),
            }
            .action_label(),
            None
        );

        assert_eq!(Cmd::CycleRepoSort.action_label(), None);
    }

    #[test]
    fn destructive_cmds_have_labels() {
        // For every variant where is_destructive() == true, action_label must be Some
        let all_cmds: Vec<Cmd> = vec![
            Cmd::None,
            Cmd::FetchRepos,
            Cmd::FetchPulls {
                ctx: RepoContext::new("", ""),
                state: "".into(),
            },
            Cmd::FetchIssues {
                ctx: RepoContext::new("", ""),
                state: "".into(),
            },
            Cmd::FetchNotifications { all: false },
            Cmd::FetchActions {
                ctx: RepoContext::new("", ""),
            },
            Cmd::MarkNotificationRead { id: "".into() },
            Cmd::MarkAllNotificationsRead,
            Cmd::ApprovePr {
                ctx: RepoContext::new("", ""),
                pr_number: 0,
            },
            Cmd::MergePr {
                ctx: RepoContext::new("", ""),
                pr_number: 0,
            },
            Cmd::RerunWorkflow {
                ctx: RepoContext::new("", ""),
                run_id: 0,
            },
            Cmd::FetchPrComments {
                ctx: RepoContext::new("", ""),
                pr_number: 0,
            },
            Cmd::ClosePr {
                ctx: RepoContext::new("", ""),
                pr_number: 0,
            },
            Cmd::ReopenPr {
                ctx: RepoContext::new("", ""),
                pr_number: 0,
            },
            Cmd::CancelWorkflow {
                ctx: RepoContext::new("", ""),
                run_id: 0,
            },
            Cmd::OpenRepoDetail {
                ctx: RepoContext::new("", ""),
            },
            Cmd::CycleRepoSort,
        ];

        for cmd in &all_cmds {
            if cmd.is_destructive() {
                assert!(
                    cmd.action_label().is_some(),
                    "Destructive command {:?} should have an action label",
                    cmd
                );
            }
        }
    }

    #[test]
    fn non_destructive_cmds_lack_labels() {
        let all_cmds: Vec<Cmd> = vec![
            Cmd::None,
            Cmd::FetchRepos,
            Cmd::FetchPulls {
                ctx: RepoContext::new("", ""),
                state: "".into(),
            },
            Cmd::FetchIssues {
                ctx: RepoContext::new("", ""),
                state: "".into(),
            },
            Cmd::FetchNotifications { all: false },
            Cmd::FetchActions {
                ctx: RepoContext::new("", ""),
            },
            Cmd::MarkNotificationRead { id: "".into() },
            Cmd::MarkAllNotificationsRead,
            Cmd::ApprovePr {
                ctx: RepoContext::new("", ""),
                pr_number: 0,
            },
            Cmd::MergePr {
                ctx: RepoContext::new("", ""),
                pr_number: 0,
            },
            Cmd::FetchPrComments {
                ctx: RepoContext::new("", ""),
                pr_number: 0,
            },
            Cmd::ClosePr {
                ctx: RepoContext::new("", ""),
                pr_number: 0,
            },
            Cmd::RerunWorkflow {
                ctx: RepoContext::new("", ""),
                run_id: 0,
            },
            Cmd::CancelWorkflow {
                ctx: RepoContext::new("", ""),
                run_id: 0,
            },
            Cmd::OpenRepoDetail {
                ctx: RepoContext::new("", ""),
            },
            Cmd::CycleRepoSort,
        ];

        for cmd in &all_cmds {
            if !cmd.is_destructive() {
                assert!(
                    cmd.action_label().is_none(),
                    "Non-destructive command {:?} should not have an action label",
                    cmd
                );
            }
        }
    }

    #[test]
    fn reopen_pr_is_destructive() {
        let reopen = Cmd::ReopenPr {
            ctx: RepoContext::new("o", "r"),
            pr_number: 1,
        };
        assert!(reopen.is_destructive());
    }

    #[test]
    fn reopen_pr_has_action_label() {
        let reopen = Cmd::ReopenPr {
            ctx: RepoContext::new("o", "r"),
            pr_number: 1,
        };
        assert_eq!(reopen.action_label(), Some("Reopen Pull Request"));
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
