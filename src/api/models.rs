use serde::{Deserialize, Serialize};

use crate::app;

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

impl From<Repo> for app::Repo {
    fn from(r: Repo) -> Self {
        Self {
            name: r.full_name,
            owner: r.owner.login,
            description: r.description,
            language: r.language,
            stars: r.stargazers_count as u32,
            forks: r.forks_count as u32,
            updated_at: r.updated_at,
            visibility: r.visibility.unwrap_or_default(),
            default_branch: r.default_branch,
        }
    }
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

impl From<PullRequest> for app::PullRequest {
    fn from(pr: PullRequest) -> Self {
        Self {
            number: pr.number as u32,
            title: pr.title,
            author: pr.user.login,
            state: pr.state,
            labels: pr.labels.into_iter().map(|l| l.name).collect(),
            created_at: pr.created_at,
            updated_at: pr.updated_at,
            body: pr.body,
        }
    }
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

impl From<Issue> for app::Issue {
    fn from(i: Issue) -> Self {
        Self {
            number: i.number as u32,
            title: i.title,
            author: i.user.login,
            state: i.state,
            labels: i.labels.into_iter().map(|l| l.name).collect(),
            assignees: i.assignees.into_iter().map(|u| u.login).collect(),
            created_at: i.created_at,
            updated_at: i.updated_at,
            body: i.body,
        }
    }
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

impl From<Notification> for app::Notification {
    fn from(n: Notification) -> Self {
        Self {
            repo_name: n.repository.full_name,
            subject_title: n.subject.title,
            subject_type: n.subject.subject_type,
            reason: n.reason,
            unread: n.unread,
            updated_at: n.updated_at,
        }
    }
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

impl From<WorkflowRun> for app::WorkflowRun {
    fn from(w: WorkflowRun) -> Self {
        Self {
            name: w.name,
            status: w.status,
            conclusion: w.conclusion,
            branch: w.head_branch,
            event: w.event,
            created_at: w.created_at,
            run_number: w.run_number as u32,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunsResponse {
    pub total_count: u64,
    pub workflow_runs: Vec<WorkflowRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowJob {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowJobsResponse {
    pub total_count: u64,
    pub jobs: Vec<WorkflowJob>,
}
