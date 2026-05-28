use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use reqwest::{Client, Method};

use super::cache::{Cache, CacheKey};
use super::models::*;
use crate::domain;

const BASE_URL: &str = "https://api.github.com";
const USER_AGENT: &str = "ghtui";
const REPO_CACHE_TTL_SECS: u64 = 60;
const PULLS_CACHE_TTL_SECS: u64 = 30;
const ISSUES_CACHE_TTL_SECS: u64 = 30;
const NOTIFICATIONS_CACHE_TTL_SECS: u64 = 15;
const ACTIONS_CACHE_TTL_SECS: u64 = 30;
const PR_COMMENTS_CACHE_TTL_SECS: u64 = 60;

pub struct GitHubClient {
    http: Client,
    token: String,
    base_url: String,
    cache: Mutex<Cache>,
    pub rate_remaining: Mutex<Option<u32>>,
    pub rate_reset: Mutex<Option<Instant>>,
}

impl GitHubClient {
    pub fn new(token: String) -> Self {
        Self {
            http: Client::new(),
            token,
            base_url: BASE_URL.to_string(),
            cache: Mutex::new(Cache::new()),
            rate_remaining: Mutex::new(None),
            rate_reset: Mutex::new(None),
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(token: String, base_url: String) -> Self {
        Self {
            http: Client::new(),
            token,
            base_url,
            cache: Mutex::new(Cache::new()),
            rate_remaining: Mutex::new(None),
            rate_reset: Mutex::new(None),
        }
    }

    fn with_cache(&self, f: impl FnOnce(&mut Cache)) {
        match self.cache.lock() {
            Ok(mut cache) => f(&mut cache),
            Err(poisoned) => {
                tracing::warn!("cache mutex poisoned, recovering");
                f(&mut poisoned.into_inner())
            }
        }
    }

    fn invalidate_action_caches(&self, owner: &str, repo: &str) {
        self.with_cache(|cache| cache.invalidate_actions(owner, repo));
    }

    fn invalidate_pr_caches(&self, owner: &str, repo: &str, pr_number: u64) {
        self.with_cache(|cache| cache.invalidate_pr_related(owner, repo, pr_number));
    }

    pub fn invalidate_cache(&self, key: &CacheKey) {
        self.with_cache(|cache| cache.invalidate(key));
    }

    pub fn rate_info(&self) -> (Option<u32>, Option<Instant>) {
        let remaining = self.rate_remaining.lock().ok().and_then(|v| *v);
        let reset = self.rate_reset.lock().ok().and_then(|v| *v);
        (remaining, reset)
    }

    fn track_rate_limits(&self, headers: &reqwest::header::HeaderMap) {
        if let Some(remaining) = headers.get("x-ratelimit-remaining") {
            if let Ok(val) = remaining.to_str() {
                if let Ok(n) = val.parse::<u32>() {
                    *self
                        .rate_remaining
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = Some(n);
                }
            }
        }
        if let Some(reset) = headers.get("x-ratelimit-reset") {
            if let Ok(val) = reset.to_str() {
                if let Ok(ts) = val.parse::<u64>() {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    if ts > now {
                        *self.rate_reset.lock().unwrap_or_else(|e| e.into_inner()) =
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
            let hit: Option<T> = {
                let mut result = None;
                self.with_cache(|cache| {
                    result = cache.get::<T>(key).cloned();
                });
                result
            };
            if let Some(cached) = hit {
                return Ok(cached);
            }
        }

        let resp = self.send_and_check(self.request(Method::GET, path)).await?;

        let data = resp.json::<T>().await?;

        // Store in cache
        if let Some(key) = cache_key {
            self.with_cache(|cache| {
                cache.insert(key, data.clone(), cache_ttl);
            });
        }

        Ok(data)
    }

    async fn post_empty(&self, path: &str) -> Result<()> {
        self.send_and_check(
            self.request(Method::POST, path)
                .header("Content-Length", "0"),
        )
        .await?;
        Ok(())
    }

    async fn post_json<T: serde::Serialize + ?Sized>(&self, path: &str, body: &T) -> Result<()> {
        self.send_and_check(self.request(Method::POST, path).json(body))
            .await?;
        Ok(())
    }

    async fn patch_empty(&self, path: &str) -> Result<()> {
        self.send_and_check(self.request(Method::PATCH, path))
            .await?;
        Ok(())
    }

    async fn patch_json<T: serde::Serialize + ?Sized>(&self, path: &str, body: &T) -> Result<()> {
        self.send_and_check(self.request(Method::PATCH, path).json(body))
            .await?;
        Ok(())
    }

    pub async fn get_repos(&self) -> Result<Vec<domain::Repo>> {
        let models: Vec<Repo> = self
            .get(
                "/user/repos?sort=updated&per_page=100",
                Some(CacheKey::UserRepos),
                Duration::from_secs(REPO_CACHE_TTL_SECS),
            )
            .await?;
        Ok(models
            .into_iter()
            .map(|r| domain::Repo {
                name: r.full_name,
                description: r.description,
                language: r.language,
                stars: safe_u32(r.stargazers_count),
                forks: safe_u32(r.forks_count),
                updated_at: r.updated_at,
                visibility: r.visibility.unwrap_or_default(),
                default_branch: r.default_branch,
            })
            .collect())
    }

    pub async fn get_pulls(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
    ) -> Result<Vec<domain::PullRequest>> {
        let path = format!("/repos/{owner}/{repo}/pulls?state={state}&per_page=100");
        let models: Vec<PullRequest> = self
            .get(
                &path,
                Some(CacheKey::Pulls {
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    state: state.to_string(),
                }),
                Duration::from_secs(PULLS_CACHE_TTL_SECS),
            )
            .await?;
        Ok(models
            .into_iter()
            .map(|pr| domain::PullRequest {
                number: pr.number,
                title: pr.title,
                author: pr.user.login,
                state: pr.state,
                labels: pr.labels.into_iter().map(|l| l.name).collect(),
                created_at: pr.created_at,
                updated_at: pr.updated_at,
                body: pr.body,
                comments: None,
                reviews: None,
                code_comments: None,
                comment_errors: None,
                review_errors: None,
                code_comment_errors: None,
            })
            .collect())
    }

    pub async fn get_issues(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
    ) -> Result<Vec<domain::Issue>> {
        let path = format!("/repos/{owner}/{repo}/issues?state={state}&per_page=100");
        let models: Vec<Issue> = self
            .get(
                &path,
                Some(CacheKey::Issues {
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    state: state.to_string(),
                }),
                Duration::from_secs(ISSUES_CACHE_TTL_SECS),
            )
            .await?;
        // Filter out pull requests (GitHub returns them as issues too)
        Ok(models
            .into_iter()
            .filter(|i| i.pull_request.is_none())
            .map(|i| domain::Issue {
                number: i.number,
                title: i.title,
                author: i.user.login,
                state: i.state,
                labels: i.labels.into_iter().map(|l| l.name).collect(),
                assignees: i.assignees.into_iter().map(|u| u.login).collect(),
                created_at: i.created_at,
                updated_at: i.updated_at,
                body: i.body,
            })
            .collect())
    }

    pub async fn get_notifications(&self, all: bool) -> Result<Vec<domain::Notification>> {
        let path = if all {
            "/notifications?per_page=100&all=true"
        } else {
            "/notifications?per_page=100"
        };
        let models: Vec<Notification> = self
            .get(
                path,
                Some(CacheKey::Notifications),
                Duration::from_secs(NOTIFICATIONS_CACHE_TTL_SECS),
            )
            .await?;
        Ok(models
            .into_iter()
            .map(|n| domain::Notification {
                id: n.id,
                repo_name: n.repository.full_name,
                subject_title: n.subject.title,
                subject_type: n.subject.subject_type,
                reason: n.reason,
                unread: n.unread,
                updated_at: n.updated_at,
            })
            .collect())
    }

    pub async fn get_actions(&self, owner: &str, repo: &str) -> Result<Vec<domain::WorkflowRun>> {
        let path = format!("/repos/{owner}/{repo}/actions/runs?per_page=50");
        let resp: WorkflowRunsResponse = self
            .get(
                &path,
                Some(CacheKey::Actions {
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                }),
                Duration::from_secs(ACTIONS_CACHE_TTL_SECS),
            )
            .await?;
        Ok(resp
            .workflow_runs
            .into_iter()
            .map(|w| domain::WorkflowRun {
                id: w.id,
                name: w.name,
                status: w.status,
                conclusion: w.conclusion,
                branch: w.head_branch,
                event: w.event,
                created_at: w.created_at,
                run_number: safe_u32(w.run_number),
            })
            .collect())
    }

    pub async fn mark_notification_read(&self, id: &str) -> Result<()> {
        self.patch_empty(&format!("/notifications/threads/{id}")).await?;
        self.invalidate_cache(&CacheKey::Notifications);
        Ok(())
    }

    pub async fn mark_all_notifications_read(&self) -> Result<()> {
        self.post_empty("/notifications").await?;
        self.invalidate_cache(&CacheKey::Notifications);
        Ok(())
    }

    pub async fn approve_pr(&self, owner: &str, repo: &str, pr_number: u64) -> Result<()> {
        let body = serde_json::json!({ "event": "APPROVE" });
        self.post_json(
            &format!("/repos/{owner}/{repo}/pulls/{pr_number}/reviews"),
            &body,
        )
        .await?;
        self.invalidate_pr_caches(owner, repo, pr_number);
        Ok(())
    }

    pub async fn merge_pr(&self, owner: &str, repo: &str, pr_number: u64) -> Result<()> {
        self.put_empty(&format!("/repos/{owner}/{repo}/pulls/{pr_number}/merge"))
            .await?;
        self.invalidate_pr_caches(owner, repo, pr_number);
        Ok(())
    }

    pub async fn rerun_workflow(&self, owner: &str, repo: &str, run_id: u64) -> Result<()> {
        self.post_empty(&format!(
            "/repos/{owner}/{repo}/actions/runs/{run_id}/rerun"
        ))
        .await?;
        self.invalidate_action_caches(owner, repo);
        Ok(())
    }

    pub async fn cancel_workflow(&self, owner: &str, repo: &str, run_id: u64) -> Result<()> {
        self.post_empty(&format!(
            "/repos/{owner}/{repo}/actions/runs/{run_id}/cancel"
        ))
        .await?;
        self.invalidate_action_caches(owner, repo);
        Ok(())
    }

    pub async fn get_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<domain::PrComment>> {
        let path = format!("/repos/{owner}/{repo}/issues/{pr_number}/comments?per_page=100");
        let cache_key = CacheKey::PrComments {
            owner: owner.to_string(),
            repo: repo.to_string(),
            pr_number,
        };
        let models: Vec<IssueComment> = self
            .get(
                &path,
                Some(cache_key),
                Duration::from_secs(PR_COMMENTS_CACHE_TTL_SECS),
            )
            .await?;
        Ok(models
            .into_iter()
            .map(|c| domain::PrComment {
                id: c.id,
                author: c.user.login,
                body: c.body,
                created_at: c.created_at,
            })
            .collect())
    }

    pub async fn get_reviews(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<domain::PrReview>> {
        let path = format!("/repos/{owner}/{repo}/pulls/{pr_number}/reviews?per_page=100");
        let cache_key = CacheKey::PrReviews {
            owner: owner.to_string(),
            repo: repo.to_string(),
            pr_number,
        };
        let models: Vec<Review> = self
            .get(
                &path,
                Some(cache_key),
                Duration::from_secs(PR_COMMENTS_CACHE_TTL_SECS),
            )
            .await?;
        Ok(models
            .into_iter()
            .map(|r| domain::PrReview {
                id: r.id,
                author: r.user.login,
                state: r.state,
                body: r.body,
                submitted_at: r.submitted_at,
            })
            .collect())
    }

    pub async fn get_review_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<domain::PrCodeComment>> {
        let path = format!("/repos/{owner}/{repo}/pulls/{pr_number}/comments?per_page=100");
        let cache_key = CacheKey::PrCodeComments {
            owner: owner.to_string(),
            repo: repo.to_string(),
            pr_number,
        };
        let models: Vec<ReviewComment> = self
            .get(
                &path,
                Some(cache_key),
                Duration::from_secs(PR_COMMENTS_CACHE_TTL_SECS),
            )
            .await?;
        Ok(models
            .into_iter()
            .map(|c| domain::PrCodeComment {
                id: c.id,
                author: c.user.login,
                body: c.body,
                path: c.path,
                line: c.line,
                created_at: c.created_at,
            })
            .collect())
    }

    pub async fn close_pr(&self, owner: &str, repo: &str, pr_number: u64) -> Result<()> {
        let body = serde_json::json!({ "state": "closed" });
        self.patch_json(&format!("/repos/{owner}/{repo}/pulls/{pr_number}"), &body)
            .await?;
        self.invalidate_pr_caches(owner, repo, pr_number);
        Ok(())
    }

    pub async fn reopen_pr(&self, owner: &str, repo: &str, pr_number: u64) -> Result<()> {
        let body = serde_json::json!({ "state": "open" });
        self.patch_json(&format!("/repos/{owner}/{repo}/pulls/{pr_number}"), &body)
            .await?;
        self.invalidate_pr_caches(owner, repo, pr_number);
        Ok(())
    }

    async fn put_empty(&self, path: &str) -> Result<()> {
        self.send_and_check(self.request(Method::PUT, path)).await?;
        Ok(())
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        self.http
            .request(method, &url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", USER_AGENT)
    }

    async fn send_and_check(&self, builder: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let resp = builder.send().await?;
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
        Ok(resp)
    }
}

fn safe_u32(value: u64) -> u32 {
    u32::try_from(value).unwrap_or_else(|_| {
        tracing::warn!("integer overflow converting {value} to u32, clamping to u32::MAX");
        u32::MAX
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_u32_positive_value() {
        assert_eq!(safe_u32(42u64), 42);
    }

    #[test]
    fn safe_u32_zero() {
        assert_eq!(safe_u32(0u64), 0);
    }

    #[test]
    fn safe_u32_max_u32() {
        assert_eq!(safe_u32(u32::MAX as u64), u32::MAX);
    }

    #[test]
    fn safe_u32_overflow_returns_max() {
        assert_eq!(safe_u32(u32::MAX as u64 + 1), u32::MAX);
    }

    #[test]
    fn safe_u32_large_overflow() {
        assert_eq!(safe_u32(u64::MAX), u32::MAX);
    }

    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    async fn mock_and_client(method_match: &str, path_match: &str) -> (MockServer, GitHubClient) {
        let server = MockServer::start().await;
        Mock::given(method(method_match))
            .and(path(path_match))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri());
        (server, client)
    }

    #[tokio::test]
    async fn approve_pr_invalidates_pr_caches() {
        let (_s, client) = mock_and_client("POST", "/repos/o/r/pulls/1/reviews").await;
        { let mut c = client.cache.lock().unwrap();
          c.insert(CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "open".into() }, vec!["stale".to_string()], Duration::from_secs(60)); }
        client.approve_pr("o", "r", 1).await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "open".into() }).is_none());
    }

    #[tokio::test]
    async fn merge_pr_invalidates_pr_caches() {
        let (_s, client) = mock_and_client("PUT", "/repos/o/r/pulls/1/merge").await;
        { let mut c = client.cache.lock().unwrap();
          c.insert(CacheKey::PrComments { owner: "o".into(), repo: "r".into(), pr_number: 1 }, "stale".to_string(), Duration::from_secs(60)); }
        client.merge_pr("o", "r", 1).await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<String>(&CacheKey::PrComments { owner: "o".into(), repo: "r".into(), pr_number: 1 }).is_none());
    }

    #[tokio::test]
    async fn close_pr_invalidates_pr_caches() {
        let (_s, client) = mock_and_client("PATCH", "/repos/o/r/pulls/1").await;
        { let mut c = client.cache.lock().unwrap();
          c.insert(CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "closed".into() }, vec!["stale".to_string()], Duration::from_secs(60)); }
        client.close_pr("o", "r", 1).await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "closed".into() }).is_none());
    }

    #[tokio::test]
    async fn reopen_pr_invalidates_pr_caches() {
        let (_s, client) = mock_and_client("PATCH", "/repos/o/r/pulls/1").await;
        { let mut c = client.cache.lock().unwrap();
          c.insert(CacheKey::PrReviews { owner: "o".into(), repo: "r".into(), pr_number: 1 }, "stale".to_string(), Duration::from_secs(60)); }
        client.reopen_pr("o", "r", 1).await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<String>(&CacheKey::PrReviews { owner: "o".into(), repo: "r".into(), pr_number: 1 }).is_none());
    }

    #[tokio::test]
    async fn mark_notification_read_invalidates_notifications() {
        let (_s, client) = mock_and_client("PATCH", "/notifications/threads/abc").await;
        { let mut c = client.cache.lock().unwrap();
          c.insert(CacheKey::Notifications, vec!["stale".to_string()], Duration::from_secs(60)); }
        client.mark_notification_read("abc").await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Notifications).is_none());
    }

    #[tokio::test]
    async fn mark_all_notifications_read_invalidates_notifications() {
        let (_s, client) = mock_and_client("POST", "/notifications").await;
        { let mut c = client.cache.lock().unwrap();
          c.insert(CacheKey::Notifications, vec!["stale".to_string()], Duration::from_secs(60)); }
        client.mark_all_notifications_read().await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Notifications).is_none());
    }

    #[tokio::test]
    async fn rerun_workflow_invalidates_actions() {
        let (_s, client) = mock_and_client("POST", "/repos/o/r/actions/runs/99/rerun").await;
        { let mut c = client.cache.lock().unwrap();
          c.insert(CacheKey::Actions { owner: "o".into(), repo: "r".into() }, vec!["stale".to_string()], Duration::from_secs(60)); }
        client.rerun_workflow("o", "r", 99).await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Actions { owner: "o".into(), repo: "r".into() }).is_none());
    }

    #[tokio::test]
    async fn cancel_workflow_invalidates_actions() {
        let (_s, client) = mock_and_client("POST", "/repos/o/r/actions/runs/99/cancel").await;
        { let mut c = client.cache.lock().unwrap();
          c.insert(CacheKey::Actions { owner: "o".into(), repo: "r".into() }, vec!["stale".to_string()], Duration::from_secs(60)); }
        client.cancel_workflow("o", "r", 99).await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Actions { owner: "o".into(), repo: "r".into() }).is_none());
    }

    #[tokio::test]
    async fn approve_pr_preserves_cache_on_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST")).and(path("/repos/o/r/pulls/1/reviews"))
            .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri());
        { let mut c = client.cache.lock().unwrap();
          c.insert(CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "open".into() }, vec!["stale".to_string()], Duration::from_secs(60)); }
        let result = client.approve_pr("o", "r", 1).await;
        assert!(result.is_err());
        // Cache should still have the stale data
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "open".into() }).is_some());
    }

    #[tokio::test]
    async fn base_url_trailing_slash_produces_correct_url() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/repos/o/r/pulls"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server).await;
        
        // Client with trailing slash on base_url
        let base_url = format!("{}/", server.uri());
        let client = GitHubClient::new_for_test("tok".into(), base_url);
        
        // Should work - double slash would cause path mismatch
        let result = client.get_pulls("o", "r", "open").await;
        assert!(result.is_ok());
    }
}
