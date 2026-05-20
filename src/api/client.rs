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

    pub fn invalidate_cache(&self, key: &CacheKey) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.invalidate(key);
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

    async fn get<T: serde::de::DeserializeOwned + Clone + Send + Sync + 'static>(
        &self,
        path: &str,
        cache_key: Option<CacheKey>,
        cache_ttl: Duration,
    ) -> Result<T> {
        // Check cache first
        if let Some(ref key) = cache_key {
            if let Ok(cache) = self.cache.lock() {
                if let Some(cached) = cache.get::<T>(key) {
                    return Ok(cached.clone());
                }
            }
        }

        let url = format!("{BASE_URL}{path}");
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
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

        // Store in cache
        if let Some(key) = cache_key {
            if let Ok(mut cache) = self.cache.lock() {
                cache.insert(key, data.clone(), cache_ttl);
            }
        }

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
        let models: Vec<Repo> = self
            .get("/user/repos?sort=updated&per_page=100", Some(CacheKey::UserRepos), Duration::from_secs(60))
            .await?;
        Ok(models.into_iter().map(app::Repo::from).collect())
    }

    pub async fn get_pulls(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
    ) -> Result<Vec<app::PullRequest>> {
        let path = format!("/repos/{owner}/{repo}/pulls?state={state}&per_page=100");
        let models: Vec<PullRequest> = self
            .get(&path, Some(CacheKey::Pulls { owner: owner.to_string(), repo: repo.to_string(), state: state.to_string() }), Duration::from_secs(30))
            .await?;
        Ok(models.into_iter().map(app::PullRequest::from).collect())
    }

    pub async fn get_issues(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
    ) -> Result<Vec<app::Issue>> {
        let path = format!("/repos/{owner}/{repo}/issues?state={state}&per_page=100");
        let models: Vec<Issue> = self
            .get(&path, Some(CacheKey::Issues { owner: owner.to_string(), repo: repo.to_string(), state: state.to_string() }), Duration::from_secs(30))
            .await?;
        // Filter out pull requests (GitHub returns them as issues too)
        Ok(models
            .into_iter()
            .filter(|i| i.pull_request.is_none())
            .map(app::Issue::from)
            .collect())
    }

    pub async fn get_notifications(&self, all: bool) -> Result<Vec<app::Notification>> {
        let path = if all {
            "/notifications?per_page=100&all=true"
        } else {
            "/notifications?per_page=100"
        };
        let models: Vec<Notification> = self
            .get(path, Some(CacheKey::Notifications), Duration::from_secs(15))
            .await?;
        Ok(models.into_iter().map(app::Notification::from).collect())
    }

    pub async fn get_actions(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<app::WorkflowRun>> {
        let path = format!("/repos/{owner}/{repo}/actions/runs?per_page=50");
        let resp: WorkflowRunsResponse = self
            .get(&path, Some(CacheKey::Actions { owner: owner.to_string(), repo: repo.to_string() }), Duration::from_secs(30))
            .await?;
        Ok(resp.workflow_runs.into_iter().map(app::WorkflowRun::from).collect())
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
