use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use reqwest::Client;

use super::cache::{Cache, CacheKey};
use super::models::*;
use crate::app;

const BASE_URL: &str = "https://api.github.com";
const USER_AGENT: &str = "ghtui";

pub struct GitHubClient {
    http: Client,
    token: String,
    cache: Mutex<Cache>,
    pub rate_remaining: Mutex<Option<u32>>,
    pub rate_reset: Mutex<Option<Instant>>,
}

impl GitHubClient {
    pub fn new(token: String) -> Self {
        Self {
            http: Client::new(),
            token,
            cache: Mutex::new(Cache::new()),
            rate_remaining: Mutex::new(None),
            rate_reset: Mutex::new(None),
        }
    }

    fn track_rate_limits(&self, headers: &reqwest::header::HeaderMap) {
        if let Some(remaining) = headers.get("x-ratelimit-remaining") {
            if let Ok(val) = remaining.to_str() {
                if let Ok(n) = val.parse::<u32>() {
                    *self.rate_remaining.lock().unwrap() = Some(n);
                }
            }
        }
        if let Some(reset) = headers.get("x-ratelimit-reset") {
            if let Ok(val) = reset.to_str() {
                if let Ok(ts) = val.parse::<u64>() {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    if ts > now {
                        *self.rate_reset.lock().unwrap() =
                            Some(Instant::now() + Duration::from_secs(ts - now));
                    }
                }
            }
        }
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{BASE_URL}{path}");
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", USER_AGENT)
            .send()
            .await?;

        self.track_rate_limits(resp.headers());

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(color_eyre::eyre::eyre!(
                "GitHub API error {}: {}",
                status,
                body
            ));
        }

        let data = resp.json::<T>().await?;
        Ok(data)
    }

    async fn post_empty(&self, path: &str) -> Result<()> {
        let url = format!("{BASE_URL}{path}");
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", USER_AGENT)
            .header("Content-Length", "0")
            .send()
            .await?;

        self.track_rate_limits(resp.headers());

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(color_eyre::eyre::eyre!(
                "GitHub API error {}: {}",
                status,
                body
            ));
        }
        Ok(())
    }

    async fn post_json<T: serde::Serialize + ?Sized>(&self, path: &str, body: &T) -> Result<()> {
        let url = format!("{BASE_URL}{path}");
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", USER_AGENT)
            .json(body)
            .send()
            .await?;

        self.track_rate_limits(resp.headers());

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(color_eyre::eyre::eyre!(
                "GitHub API error {}: {}",
                status,
                body_text
            ));
        }
        Ok(())
    }

    async fn patch_empty(&self, path: &str) -> Result<()> {
        let url = format!("{BASE_URL}{path}");
        let resp = self
            .http
            .patch(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", USER_AGENT)
            .send()
            .await?;

        self.track_rate_limits(resp.headers());

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(color_eyre::eyre::eyre!(
                "GitHub API error {}: {}",
                status,
                body
            ));
        }
        Ok(())
    }

    pub async fn get_repos(&self) -> Result<Vec<app::Repo>> {
        let repos: Vec<Repo> = self
            .get("/user/repos?sort=updated&per_page=100")
            .await?;
        Ok(repos.into_iter().map(app::Repo::from).collect())
    }

    pub async fn get_pulls(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
    ) -> Result<Vec<app::PullRequest>> {
        let prs: Vec<PullRequest> = self
            .get(&format!(
                "/repos/{owner}/{repo}/pulls?state={state}&per_page=100"
            ))
            .await?;
        Ok(prs.into_iter().map(app::PullRequest::from).collect())
    }

    pub async fn get_issues(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
    ) -> Result<Vec<app::Issue>> {
        let issues: Vec<Issue> = self
            .get(&format!(
                "/repos/{owner}/{repo}/issues?state={state}&per_page=100"
            ))
            .await?;
        Ok(issues
            .into_iter()
            .filter(|i| i.pull_request.is_none())
            .map(app::Issue::from)
            .collect())
    }

    pub async fn get_notifications(&self, all: bool) -> Result<Vec<app::Notification>> {
        let param = if all { "all=true" } else { "" };
        let path = if param.is_empty() {
            "/notifications?per_page=100".to_string()
        } else {
            format!("/notifications?{param}&per_page=100")
        };
        let notifs: Vec<Notification> = self.get(&path).await?;
        Ok(notifs.into_iter().map(app::Notification::from).collect())
    }

    pub async fn get_actions(&self, owner: &str, repo: &str) -> Result<Vec<app::WorkflowRun>> {
        let resp: WorkflowRunsResponse = self
            .get(&format!(
                "/repos/{owner}/{repo}/actions/runs?per_page=50"
            ))
            .await?;
        Ok(resp
            .workflow_runs
            .into_iter()
            .map(app::WorkflowRun::from)
            .collect())
    }

    pub async fn mark_notification_read(&self, id: &str) -> Result<()> {
        self.patch_empty(&format!("/notifications/threads/{id}"))
            .await
    }

    pub async fn mark_all_notifications_read(&self) -> Result<()> {
        self.post_empty("/notifications").await
    }

    pub async fn approve_pr(&self, owner: &str, repo: &str, pr_number: u64) -> Result<()> {
        let body = serde_json::json!({ "event": "APPROVE" });
        self.post_json(
            &format!("/repos/{owner}/{repo}/pulls/{pr_number}/reviews"),
            &body,
        )
        .await
    }

    pub async fn merge_pr(&self, owner: &str, repo: &str, pr_number: u64) -> Result<()> {
        self.put_empty(&format!("/repos/{owner}/{repo}/pulls/{pr_number}/merge"))
            .await
    }

    pub async fn add_comment(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<()> {
        let payload = serde_json::json!({ "body": body });
        self.post_json(
            &format!("/repos/{owner}/{repo}/issues/{issue_number}/comments"),
            &payload,
        )
        .await
    }

    pub async fn rerun_workflow(&self, owner: &str, repo: &str, run_id: u64) -> Result<()> {
        self.post_empty(&format!(
            "/repos/{owner}/{repo}/actions/runs/{run_id}/rerun"
        ))
        .await
    }

    pub async fn cancel_workflow(&self, owner: &str, repo: &str, run_id: u64) -> Result<()> {
        self.post_empty(&format!(
            "/repos/{owner}/{repo}/actions/runs/{run_id}/cancel"
        ))
        .await
    }

    async fn put_empty(&self, path: &str) -> Result<()> {
        let url = format!("{BASE_URL}{path}");
        let resp = self
            .http
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", USER_AGENT)
            .send()
            .await?;

        self.track_rate_limits(resp.headers());

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(color_eyre::eyre::eyre!(
                "GitHub API error {}: {}",
                status,
                body
            ));
        }
        Ok(())
    }
}
