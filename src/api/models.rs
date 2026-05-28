use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub id: u64,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: User,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stargazers_count: u64,
    pub forks_count: u64,
    pub default_branch: String,
    pub visibility: Option<String>,
    pub updated_at: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub user: User,
    pub state: String,
    pub merged_at: Option<String>,
    pub body: Option<String>,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub labels: Vec<Label>,
    pub draft: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub user: User,
    pub state: String,
    pub body: Option<String>,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub labels: Vec<Label>,
    pub assignees: Vec<User>,
    #[serde(default)]
    pub pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRepo {
    pub full_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSubject {
    pub title: String,
    #[serde(rename = "type")]
    pub subject_type: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub repository: NotificationRepo,
    pub subject: NotificationSubject,
    pub reason: String,
    pub unread: bool,
    pub updated_at: String,
    pub last_read_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub head_branch: String,
    pub event: String,
    pub created_at: String,
    pub run_number: u64,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunsResponse {
    pub total_count: u64,
    pub workflow_runs: Vec<WorkflowRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueComment {
    pub id: u64,
    pub user: User,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: u64,
    pub user: User,
    pub state: String,
    pub body: Option<String>,
    pub submitted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub id: u64,
    pub user: User,
    pub body: String,
    pub path: String,
    pub line: Option<u32>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestFile {
    pub filename: String,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
    pub patch: Option<String>,
    pub previous_filename: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pull_request_deserializes_large_number() {
        let json = r#"{
            "number": 5000000000,
            "title": "Big PR",
            "user": {"login": "u", "id": 1},
            "state": "open",
            "html_url": "",
            "created_at": "",
            "updated_at": "",
            "labels": []
        }"#;
        let pr: PullRequest = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 5_000_000_000);
    }

    #[test]
    fn issue_deserializes_large_number() {
        let json = r#"{
            "number": 5000000000,
            "title": "Big Issue",
            "user": {"login": "u", "id": 1},
            "state": "open",
            "html_url": "",
            "created_at": "",
            "updated_at": "",
            "labels": [],
            "assignees": []
        }"#;
        let issue: Issue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.number, 5_000_000_000);
    }
}
