pub enum Cmd {
    None,
    FetchRepos,
    FetchPulls { owner: String, repo: String, state: String },
    FetchIssues { owner: String, repo: String, state: String },
    FetchNotifications { all: bool },
    FetchActions { owner: String, repo: String },
    MarkNotificationRead { id: String },
    MarkAllNotificationsRead,
    ApprovePr { owner: String, repo: String, pr_number: u64 },
    MergePr { owner: String, repo: String, pr_number: u64 },
    RerunWorkflow { owner: String, repo: String, run_id: u64 },
    CancelWorkflow { owner: String, repo: String, run_id: u64 },
    OpenRepoDetail { owner: String, repo: String },
    CycleRepoSort,
}

impl std::fmt::Debug for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::None => write!(f, "None"),
            Cmd::FetchRepos => write!(f, "FetchRepos"),
            Cmd::FetchPulls { owner, repo, state } => write!(f, "FetchPulls({}, {}, {})", owner, repo, state),
            Cmd::FetchIssues { owner, repo, state } => write!(f, "FetchIssues({}, {}, {})", owner, repo, state),
            Cmd::FetchNotifications { all } => write!(f, "FetchNotifications(all={})", all),
            Cmd::FetchActions { owner, repo } => write!(f, "FetchActions({}, {})", owner, repo),
            Cmd::MarkNotificationRead { id } => write!(f, "MarkNotificationRead({})", id),
            Cmd::MarkAllNotificationsRead => write!(f, "MarkAllNotificationsRead"),
            Cmd::ApprovePr { owner, repo, pr_number } => write!(f, "ApprovePr({}/{}/{})", owner, repo, pr_number),
            Cmd::MergePr { owner, repo, pr_number } => write!(f, "MergePr({}/{}/{})", owner, repo, pr_number),
            Cmd::RerunWorkflow { owner, repo, run_id } => write!(f, "RerunWorkflow({}/{}/{})", owner, repo, run_id),
            Cmd::CancelWorkflow { owner, repo, run_id } => write!(f, "CancelWorkflow({}/{}/{})", owner, repo, run_id),
            Cmd::OpenRepoDetail { owner, repo } => write!(f, "OpenRepoDetail({}/{})", owner, repo),
            Cmd::CycleRepoSort => write!(f, "CycleRepoSort"),
        }
    }
}


impl Cmd {
    /// Returns true if this command is destructive and requires confirmation.
    pub fn is_destructive(&self) -> bool {
        matches!(
            self,
            Cmd::MergePr { .. }
                | Cmd::ApprovePr { .. }
                | Cmd::CancelWorkflow { .. }
                | Cmd::RerunWorkflow { .. }
                | Cmd::MarkAllNotificationsRead
        )
    }

    /// Human-readable action label for confirmation dialogs.
    pub fn action_label(&self) -> &'static str {
        match self {
            Cmd::MergePr { .. } => "Merge Pull Request",
            Cmd::ApprovePr { .. } => "Approve Pull Request",
            Cmd::CancelWorkflow { .. } => "Cancel Workflow",
            Cmd::RerunWorkflow { .. } => "Rerun Workflow",
            Cmd::MarkAllNotificationsRead => "Mark All Notifications as Read",
            _ => "",
        }
    }
}
