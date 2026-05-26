pub enum Cmd {
    None,
    FetchRepos,
    FetchPulls {
        owner: String,
        repo: String,
        state: String,
    },
    FetchIssues {
        owner: String,
        repo: String,
        state: String,
    },
    FetchNotifications {
        all: bool,
    },
    FetchActions {
        owner: String,
        repo: String,
    },
    MarkNotificationRead {
        id: String,
    },
    MarkAllNotificationsRead,
    ApprovePr {
        owner: String,
        repo: String,
        pr_number: u64,
    },
    MergePr {
        owner: String,
        repo: String,
        pr_number: u64,
    },
    RerunWorkflow {
        owner: String,
        repo: String,
        run_id: u64,
    },
    CancelWorkflow {
        owner: String,
        repo: String,
        run_id: u64,
    },
    OpenRepoDetail {
        owner: String,
        repo: String,
    },
    CycleRepoSort,
    FetchPrComments {
        owner: String,
        repo: String,
        pr_number: u64,
    },
    ClosePr {
        owner: String,
        repo: String,
        pr_number: u64,
    },
    ReopenPr {
        owner: String,
        repo: String,
        pr_number: u64,
    },
}

impl std::fmt::Debug for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::None => write!(f, "None"),
            Cmd::FetchRepos => write!(f, "FetchRepos"),
            Cmd::FetchPulls { owner, repo, state } => {
                write!(f, "FetchPulls({}, {}, {})", owner, repo, state)
            }
            Cmd::FetchIssues { owner, repo, state } => {
                write!(f, "FetchIssues({}, {}, {})", owner, repo, state)
            }
            Cmd::FetchNotifications { all } => write!(f, "FetchNotifications(all={})", all),
            Cmd::FetchActions { owner, repo } => write!(f, "FetchActions({}, {})", owner, repo),
            Cmd::MarkNotificationRead { id } => write!(f, "MarkNotificationRead({})", id),
            Cmd::MarkAllNotificationsRead => write!(f, "MarkAllNotificationsRead"),
            Cmd::ApprovePr {
                owner,
                repo,
                pr_number,
            } => write!(f, "ApprovePr({}/{}/{})", owner, repo, pr_number),
            Cmd::MergePr {
                owner,
                repo,
                pr_number,
            } => write!(f, "MergePr({}/{}/{})", owner, repo, pr_number),
            Cmd::RerunWorkflow {
                owner,
                repo,
                run_id,
            } => write!(f, "RerunWorkflow({}/{}/{})", owner, repo, run_id),
            Cmd::CancelWorkflow {
                owner,
                repo,
                run_id,
            } => write!(f, "CancelWorkflow({}/{}/{})", owner, repo, run_id),
            Cmd::OpenRepoDetail { owner, repo } => write!(f, "OpenRepoDetail({}/{})", owner, repo),
            Cmd::CycleRepoSort => write!(f, "CycleRepoSort"),
            Cmd::FetchPrComments {
                owner,
                repo,
                pr_number,
            } => write!(f, "FetchPrComments({}/{}/{})", owner, repo, pr_number),
            Cmd::ClosePr {
                owner,
                repo,
                pr_number,
            } => write!(f, "ClosePr({}/{}/{})", owner, repo, pr_number),
            Cmd::ReopenPr {
                owner,
                repo,
                pr_number,
            } => write!(f, "ReopenPr({}/{}/{})", owner, repo, pr_number),
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
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        assert_eq!(merge.action_label(), Some("Merge Pull Request"));

        let approve = Cmd::ApprovePr {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        assert_eq!(approve.action_label(), Some("Approve Pull Request"));

        let cancel = Cmd::CancelWorkflow {
            owner: "o".into(),
            repo: "r".into(),
            run_id: 1,
        };
        assert_eq!(cancel.action_label(), Some("Cancel Workflow"));

        let rerun = Cmd::RerunWorkflow {
            owner: "o".into(),
            repo: "r".into(),
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
            owner: "o".into(),
            repo: "r".into(),
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
                owner: "o".into(),
                repo: "r".into(),
                state: "open".into(),
            }
            .action_label(),
            None
        );

        assert_eq!(
            Cmd::FetchIssues {
                owner: "o".into(),
                repo: "r".into(),
                state: "open".into(),
            }
            .action_label(),
            None
        );

        assert_eq!(Cmd::FetchNotifications { all: false }.action_label(), None);

        assert_eq!(
            Cmd::FetchActions {
                owner: "o".into(),
                repo: "r".into(),
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
                owner: "o".into(),
                repo: "r".into(),
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
                owner: "".into(),
                repo: "".into(),
                state: "".into(),
            },
            Cmd::FetchIssues {
                owner: "".into(),
                repo: "".into(),
                state: "".into(),
            },
            Cmd::FetchNotifications { all: false },
            Cmd::FetchActions {
                owner: "".into(),
                repo: "".into(),
            },
            Cmd::MarkNotificationRead { id: "".into() },
            Cmd::MarkAllNotificationsRead,
            Cmd::ApprovePr {
                owner: "".into(),
                repo: "".into(),
                pr_number: 0,
            },
            Cmd::MergePr {
                owner: "".into(),
                repo: "".into(),
                pr_number: 0,
            },
            Cmd::RerunWorkflow {
                owner: "".into(),
                repo: "".into(),
                run_id: 0,
            },
            Cmd::FetchPrComments {
                owner: "".into(),
                repo: "".into(),
                pr_number: 0,
            },
            Cmd::ClosePr {
                owner: "".into(),
                repo: "".into(),
                pr_number: 0,
            },
            Cmd::ReopenPr {
                owner: "".into(),
                repo: "".into(),
                pr_number: 0,
            },
            Cmd::CancelWorkflow {
                owner: "".into(),
                repo: "".into(),
                run_id: 0,
            },
            Cmd::OpenRepoDetail {
                owner: "".into(),
                repo: "".into(),
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
                owner: "".into(),
                repo: "".into(),
                state: "".into(),
            },
            Cmd::FetchIssues {
                owner: "".into(),
                repo: "".into(),
                state: "".into(),
            },
            Cmd::FetchNotifications { all: false },
            Cmd::FetchActions {
                owner: "".into(),
                repo: "".into(),
            },
            Cmd::MarkNotificationRead { id: "".into() },
            Cmd::MarkAllNotificationsRead,
            Cmd::ApprovePr {
                owner: "".into(),
                repo: "".into(),
                pr_number: 0,
            },
            Cmd::MergePr {
                owner: "".into(),
                repo: "".into(),
                pr_number: 0,
            },
            Cmd::FetchPrComments {
                owner: "".into(),
                repo: "".into(),
                pr_number: 0,
            },
            Cmd::ClosePr {
                owner: "".into(),
                repo: "".into(),
                pr_number: 0,
            },
            Cmd::RerunWorkflow {
                owner: "".into(),
                repo: "".into(),
                run_id: 0,
            },
            Cmd::CancelWorkflow {
                owner: "".into(),
                repo: "".into(),
                run_id: 0,
            },
            Cmd::OpenRepoDetail {
                owner: "".into(),
                repo: "".into(),
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
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        assert!(reopen.is_destructive());
    }

    #[test]
    fn reopen_pr_has_action_label() {
        let reopen = Cmd::ReopenPr {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        assert_eq!(reopen.action_label(), Some("Reopen Pull Request"));
    }

    #[test]
    fn debug_format_has_closing_paren() {
        let cmd = Cmd::FetchPrComments {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        let debug = format!("{:?}", cmd);
        assert!(
            debug.ends_with(')'),
            "Debug should have closing paren: {}",
            debug
        );

        let cmd = Cmd::ClosePr {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        let debug = format!("{:?}", cmd);
        assert!(
            debug.ends_with(')'),
            "Debug should have closing paren: {}",
            debug
        );

        let cmd = Cmd::ReopenPr {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        let debug = format!("{:?}", cmd);
        assert!(
            debug.ends_with(')'),
            "Debug should have closing paren: {}",
            debug
        );
    }
}
