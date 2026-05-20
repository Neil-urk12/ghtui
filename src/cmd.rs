#[derive(Debug)]
pub enum Cmd {
    None,
    FetchRepos,
    FetchPulls { owner: String, repo: String },
    FetchIssues { owner: String, repo: String },
    FetchNotifications,
    FetchActions { owner: String, repo: String },
}
