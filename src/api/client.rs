use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use reqwest::{Client, Method, redirect::Policy};
use url::Url;

use super::cache::{Cache, CacheKey};
use super::models::*;
use crate::domain;
use crate::constants::{REPO_CACHE_TTL_SECS, PULLS_CACHE_TTL_SECS, ISSUES_CACHE_TTL_SECS, NOTIFICATIONS_CACHE_TTL_SECS, ACTIONS_CACHE_TTL_SECS, PR_COMMENTS_CACHE_TTL_SECS};

const BASE_URL: &str = "https://api.github.com";
const USER_AGENT: &str = "ghtui";

pub struct GitHubClient {
    http: Client,
    token: String,
    base_url: String,
    cache: Mutex<Cache>,
    pub rate_remaining: Mutex<Option<u32>>,
    pub rate_reset: Mutex<Option<Instant>>,
}

pub(crate) fn build_http_client() -> color_eyre::Result<Client> {
    Client::builder()
        .redirect(Policy::custom(|attempt| {
            if attempt.previous().last().and_then(|u| u.host_str()) == attempt.url().host_str() {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(Into::into)
}
impl GitHubClient {
    pub fn new(token: String) -> color_eyre::Result<Self> {
        Ok(Self {
            http: build_http_client()?,
            token,
            base_url: BASE_URL.to_string(),
            cache: Mutex::new(Cache::new()),
            rate_remaining: Mutex::new(None),
            rate_reset: Mutex::new(None),
        })
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(token: String, base_url: String) -> color_eyre::Result<Self> {
        Ok(Self {
            http: build_http_client()?,
            token,
            base_url,
            cache: Mutex::new(Cache::new()),
            rate_remaining: Mutex::new(None),
            rate_reset: Mutex::new(None),
        })
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

    pub(crate) fn invalidate_cache(&self, key: &CacheKey) {
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

    /// Fetch all pages of a paginated GitHub API endpoint, collecting items into a `Vec<T>`.
    ///
    /// Follows RFC 5988 `Link` headers for pagination. Results are cached when `cache_key` is provided.
    ///
    /// **Partial failure behavior:** If a page beyond the first fails, the method returns
    /// whatever items were collected so far rather than failing entirely. These partial results
    /// ARE cached — subsequent calls within the TTL return the same truncated data. This is
    /// intentional: partial data is often more useful than no data for UI display.
    async fn get_all<T, W, F>(
        &self,
        path: &str,
        cache_key: Option<CacheKey>,
        cache_ttl: Duration,
        max_pages: Option<usize>,
        extract: F,
    ) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + Clone + Send + Sync + 'static,
        W: serde::de::DeserializeOwned,
        F: Fn(W) -> Vec<T>,
    {
        // Check cache first
        if let Some(ref key) = cache_key {
            let hit: Option<Vec<T>> = {
                let mut result = None;
                self.with_cache(|cache| {
                    result = cache.get::<Vec<T>>(key).cloned();
                });
                result
            };
            if let Some(cached) = hit {
                return Ok(cached);
            }
        }

        let mut all_items: Vec<T> = Vec::new();
        let mut current_path = path.to_string();
        let mut page_count = 0usize;

        loop {
            if let Some(max) = max_pages {
                if page_count >= max {
                    break;
                }
            }

            let resp = match self.send_and_check(self.request(Method::GET, &current_path)).await {
                Ok(resp) => resp,
                Err(e) => {
                    if page_count > 0 {
                        tracing::warn!("get_all: page {} failed, returning partial results: {}", page_count + 1, e);
                        break;
                    } else {
                        return Err(e);
                    }
                }
            };

            let next_link = resp
                .headers()
                .get("link")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| parse_next_link(v, Some(&self.base_url)));

            let wrapper: W = resp.json().await?;
            let mut items = extract(wrapper);
            all_items.append(&mut items);
            page_count += 1;

            match next_link {
                Some(url) => {
                    // Strip base_url prefix to get path portion
                    current_path = url
                        .strip_prefix(&self.base_url)
                        .unwrap_or(&url)
                        .to_string();
                }
                None => break,
            }
        }

        // Store in cache
        if let Some(key) = cache_key {
            self.with_cache(|cache| {
                cache.insert(key, all_items.clone(), cache_ttl);
            });
        }

        Ok(all_items)
    }

    /// Fetch all pages of a paginated endpoint that returns a wrapper object containing `Vec<T>`.
    ///
    /// Like [`get_all`] but for endpoints such as GitHub Actions (`{ total_count, workflow_runs }`).
    /// The `extract` closure pulls the `Vec<T>` from the wrapper on each page.
    ///
    /// **Partial failure behavior:** Same as [`get_all`] — mid-pagination failures return
    /// partial results and cache them. See [`get_all`] for details.
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
            .get_all(
                "/user/repos?sort=updated&per_page=100",
                Some(CacheKey::UserRepos),
                Duration::from_secs(REPO_CACHE_TTL_SECS),
                None,
                |v| v,
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
            .get_all(
                &path,
                Some(CacheKey::Pulls {
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    state: state.to_string(),
                }),
                Duration::from_secs(PULLS_CACHE_TTL_SECS),
                None,
                |v| v,
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
                merged_at: pr.merged_at,
                comments: None,
                reviews: None,
                code_comments: None,
                comment_errors: None,
                review_errors: None,
                code_comment_errors: None,
                files: None,
                file_errors: None,
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
            .get_all(
                &path,
                Some(CacheKey::Issues {
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    state: state.to_string(),
                }),
                Duration::from_secs(ISSUES_CACHE_TTL_SECS),
                None,
                |v| v,
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
            .get_all(
                path,
                Some(CacheKey::Notifications),
                Duration::from_secs(NOTIFICATIONS_CACHE_TTL_SECS),
                None,
                |v| v,
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
        let runs: Vec<WorkflowRun> = self
            .get_all(
                &path,
                Some(CacheKey::Actions {
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                }),
                Duration::from_secs(ACTIONS_CACHE_TTL_SECS),
                Some(3),
                |resp: WorkflowRunsResponse| resp.workflow_runs,
            )
            .await?;
        Ok(runs
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

    pub async fn get_pr_files(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<domain::PrFile>> {
        let cache_key = CacheKey::PrFiles {
            owner: owner.to_string(),
            repo: repo.to_string(),
            pr_number,
        };
        {
            let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(cached) = cache.get::<Vec<domain::PrFile>>(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let mut all_models: Vec<PullRequestFile> = Vec::new();
        let mut page = 1u32;
        const MAX_PAGES: u32 = 100;
        loop {
            if page > MAX_PAGES {
                tracing::warn!(max_pages = MAX_PAGES, owner, repo, pr_number, "pr files pagination exceeded");
                break;
            }
            let path = format!(
                "/repos/{owner}/{repo}/pulls/{pr_number}/files?per_page=100&page={page}"
            );
            let resp = self
                .send_and_check(self.request(Method::GET, &path))
                .await?;
            let has_next = resp
                .headers()
                .get("link")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| parse_next_link(v, Some(&self.base_url)))
                .is_some();
            let mut models: Vec<PullRequestFile> = resp.json().await?;
            all_models.append(&mut models);
            if has_next {
                page += 1;
            } else {
                break;
            }
        }

        let result: Vec<domain::PrFile> = all_models
            .into_iter()
            .map(|f| domain::PrFile {
                filename: f.filename,
                status: f.status,
                additions: f.additions,
                deletions: f.deletions,
                patch: f.patch,
                previous_filename: f.previous_filename,
            })
            .collect();

        {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            cache.insert(cache_key, result.clone(), Duration::from_secs(PR_COMMENTS_CACHE_TTL_SECS));
        }
        Ok(result)
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
        tracing::warn!(value, "integer overflow converting to u32, clamping to u32::MAX");
        u32::MAX
    })
}

fn parse_next_link(header_value: &str, expected_prefix: Option<&str>) -> Option<String> {
    for link in header_value.split(',').map(str::trim) {
        if !link.contains("rel=\"next\"") {
            continue;
        }
        if let Some(start) = link.find('<') {
            if let Some(end) = link[start + 1..].find('>') {
                let url = &link[start + 1..start + 1 + end];
                if url.starts_with("https://") || url.starts_with("http://") {
                    if let Some(prefix) = expected_prefix {
                        // Parse once, extract host+port for comparison
                        // url::Url normalizes default ports (443 for https, 80 for http)
                        let url_parsed = Url::parse(url).ok();
                        let prefix_parsed = Url::parse(prefix).ok();
                        // Reject userinfo URLs (e.g. https://user@api.github.com)
                        if url_parsed.as_ref().is_some_and(|u| !u.username().is_empty()) {
                            tracing::warn!("parse_next_link: rejecting URL with userinfo: {}", url);
                            return None;
                        }
                        let url_host = url_parsed.as_ref().and_then(|u| u.host_str().map(String::from));
                        let prefix_host = prefix_parsed.as_ref().and_then(|u| u.host_str().map(String::from));
                        match (url_host, prefix_host) {
                            (Some(uh), Some(ph)) if uh == ph => {
                                let url_port = url_parsed.as_ref().and_then(|u| u.port());
                                let prefix_port = prefix_parsed.as_ref().and_then(|u| u.port());
                                if url_port != prefix_port {
                                    tracing::warn!("parse_next_link: rejecting URL with different port: {}", url);
                                    return None;
                                }
                            }
                            _ => {
                                tracing::warn!("parse_next_link: rejecting URL with unexpected host: {}", url);
                                return None;
                            }
                        }
                    }
                    return Some(url.to_string());
                }
            }
        }
    }
    None
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
    use wiremock::matchers::{method, path, query_param};

    async fn mock_and_client(method_match: &str, path_match: &str) -> (MockServer, GitHubClient) {
        let server = MockServer::start().await;
        Mock::given(method(method_match))
            .and(path(path_match))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        (server, client)
    }

    fn seed<T: Send + Sync + 'static>(client: &GitHubClient, key: CacheKey, val: T) {
        let mut c = client.cache.lock().unwrap();
        c.insert(key, val, crate::constants::TEST_CACHE_TTL);
    }

    #[tokio::test]
    async fn approve_pr_invalidates_pr_caches() {
        let (_s, client) = mock_and_client("POST", "/repos/o/r/pulls/1/reviews").await;
        seed(&client, CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "open".into() }, vec!["stale".to_string()]);
        client.approve_pr("o", "r", 1).await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "open".into() }).is_none());
    }

    #[tokio::test]
    async fn merge_pr_invalidates_pr_caches() {
        let (_s, client) = mock_and_client("PUT", "/repos/o/r/pulls/1/merge").await;
        seed(&client, CacheKey::PrComments { owner: "o".into(), repo: "r".into(), pr_number: 1 }, "stale".to_string());
        client.merge_pr("o", "r", 1).await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<String>(&CacheKey::PrComments { owner: "o".into(), repo: "r".into(), pr_number: 1 }).is_none());
    }

    #[tokio::test]
    async fn close_pr_invalidates_pr_caches() {
        let (_s, client) = mock_and_client("PATCH", "/repos/o/r/pulls/1").await;
        seed(&client, CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "closed".into() }, vec!["stale".to_string()]);
        client.close_pr("o", "r", 1).await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "closed".into() }).is_none());
    }

    #[tokio::test]
    async fn reopen_pr_invalidates_pr_caches() {
        let (_s, client) = mock_and_client("PATCH", "/repos/o/r/pulls/1").await;
        seed(&client, CacheKey::PrReviews { owner: "o".into(), repo: "r".into(), pr_number: 1 }, "stale".to_string());
        client.reopen_pr("o", "r", 1).await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<String>(&CacheKey::PrReviews { owner: "o".into(), repo: "r".into(), pr_number: 1 }).is_none());
    }

    #[tokio::test]
    async fn mark_notification_read_invalidates_notifications() {
        let (_s, client) = mock_and_client("PATCH", "/notifications/threads/abc").await;
        seed(&client, CacheKey::Notifications, vec!["stale".to_string()]);
        client.mark_notification_read("abc").await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Notifications).is_none());
    }

    #[tokio::test]
    async fn mark_all_notifications_read_invalidates_notifications() {
        let (_s, client) = mock_and_client("POST", "/notifications").await;
        seed(&client, CacheKey::Notifications, vec!["stale".to_string()]);
        client.mark_all_notifications_read().await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Notifications).is_none());
    }

    #[tokio::test]
    async fn rerun_workflow_invalidates_actions() {
        let (_s, client) = mock_and_client("POST", "/repos/o/r/actions/runs/99/rerun").await;
        seed(&client, CacheKey::Actions { owner: "o".into(), repo: "r".into() }, vec!["stale".to_string()]);
        client.rerun_workflow("o", "r", 99).await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Actions { owner: "o".into(), repo: "r".into() }).is_none());
    }

    #[tokio::test]
    async fn cancel_workflow_invalidates_actions() {
        let (_s, client) = mock_and_client("POST", "/repos/o/r/actions/runs/99/cancel").await;
        seed(&client, CacheKey::Actions { owner: "o".into(), repo: "r".into() }, vec!["stale".to_string()]);
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
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        seed(&client, CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "open".into() }, vec!["stale".to_string()]);
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
        let client = GitHubClient::new_for_test("tok".into(), base_url).unwrap();
        
        // Should work - double slash would cause path mismatch
        let result = client.get_pulls("o", "r", "open").await;
        assert!(result.is_ok());
    }

    #[test]
    #[tracing_test::traced_test]
    fn poisoned_mutex_logs_warn_level() {
        let client = GitHubClient::new_for_test("tok".into(), "http://localhost".into()).unwrap();
        // Poison the mutex
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = client.cache.lock().unwrap();
            panic!("intentional poison");
        }));
        // Call with_cache — should recover and log
        client.with_cache(|_cache| {});
        // Assert message logged at correct level
        logs_assert(|lines: &[&str]| {
            let relevant: Vec<_> = lines.iter()
                .filter(|l| l.contains("cache mutex poisoned"))
                .collect();
            if relevant.is_empty() {
                Err("Expected 'cache mutex poisoned' log message".into())
            } else if relevant.iter().any(|l| l.contains("ERROR")) {
                Err("Should not log at ERROR level — recovery succeeds".into())
            } else if !relevant.iter().any(|l| l.contains("WARN")) {
                Err("Expected WARN level for mutex poison recovery".into())
            } else {
                Ok(())
            }
        });
    }

    #[tokio::test]
    async fn get_pr_files_returns_mapped_data() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/repos/o/r/pulls/1/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "filename": "src/main.rs",
                    "status": "modified",
                    "additions": 10,
                    "deletions": 5,
                    "patch": "@@ -1,3 +1,3 @@",
                    "previous_filename": null
                },
                {
                    "filename": "old.txt",
                    "status": "renamed",
                    "additions": 0,
                    "deletions": 0,
                    "patch": null,
                    "previous_filename": "new.txt"
                }
            ])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let files = client.get_pr_files("o", "r", 1).await.unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].filename, "src/main.rs");
        assert_eq!(files[0].status, "modified");
        assert_eq!(files[0].additions, 10);
        assert_eq!(files[0].deletions, 5);
        assert_eq!(files[0].patch.as_deref(), Some("@@ -1,3 +1,3 @@"));
        assert!(files[0].previous_filename.is_none());
        assert_eq!(files[1].filename, "old.txt");
        assert_eq!(files[1].previous_filename.as_deref(), Some("new.txt"));
    }

    #[tokio::test]
    async fn get_pr_files_cache_hit() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/repos/o/r/pulls/1/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "filename": "a.rs", "status": "added", "additions": 1, "deletions": 0, "patch": null, "previous_filename": null }
            ])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        // First call populates cache
        let files = client.get_pr_files("o", "r", 1).await.unwrap();
        assert_eq!(files.len(), 1);
        // Second call should return cached data
        let files2 = client.get_pr_files("o", "r", 1).await.unwrap();
        assert_eq!(files2.len(), 1);
        assert_eq!(files2[0].filename, "a.rs");
    }

    #[tokio::test]
    async fn get_pr_files_cache_invalidation() {
        let (_s, client) = mock_and_client("POST", "/repos/o/r/pulls/1/reviews").await;
        seed(&client, CacheKey::PrFiles { owner: "o".into(), repo: "r".into(), pr_number: 1 }, vec!["stale".to_string()]);
        client.approve_pr("o", "r", 1).await.unwrap();
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::PrFiles { owner: "o".into(), repo: "r".into(), pr_number: 1 }).is_none());
    }

    #[tokio::test]
    async fn get_pr_files_paginates_when_link_header_present() {
        let server = MockServer::start().await;

        // Page 1: 2 files + Link header pointing to page 2
        Mock::given(method("GET")).and(path("/repos/o/r/pulls/1/files")).and(query_param("page", "1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([
                        { "filename": "a.rs", "status": "added", "additions": 1, "deletions": 0, "patch": null, "previous_filename": null },
                        { "filename": "b.rs", "status": "modified", "additions": 2, "deletions": 1, "patch": "@@ -1 +1 @@", "previous_filename": null }
                    ]))
                    .insert_header("Link", format!(r#"<{}/repos/o/r/pulls/1/files?page=2&per_page=100>; rel="next""#, server.uri()))
            )
            .mount(&server).await;

        // Page 2: 1 file, no Link header
        Mock::given(method("GET")).and(path("/repos/o/r/pulls/1/files"))
            .and(query_param("page", "2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([
                        { "filename": "c.rs", "status": "deleted", "additions": 0, "deletions": 5, "patch": null, "previous_filename": null }
                    ]))
            )
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let files = client.get_pr_files("o", "r", 1).await.unwrap();

        assert_eq!(files.len(), 3, "should return all files from both pages");
        assert_eq!(files[0].filename, "a.rs");
        assert_eq!(files[1].filename, "b.rs");
        assert_eq!(files[2].filename, "c.rs");
    }

    #[tokio::test]
    async fn get_pr_files_rejects_cross_host_next_link() {
        let server = MockServer::start().await;

        // Page 1 returns files with a next-link pointing to a different host
        Mock::given(method("GET")).and(path("/repos/o/r/pulls/1/files"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([
                        { "filename": "a.rs", "status": "modified", "additions": 1, "deletions": 0, "patch": null, "previous_filename": null }
                    ]))
                    .insert_header("Link", r#"<https://evil.com/repos/o/r/pulls/1/files?page=2>; rel="next""#)
            )
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let files = client.get_pr_files("o", "r", 1).await.unwrap();

        // Should return only page 1 files — cross-host link is ignored
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "a.rs");
    }

    // --- parse_next_link tests ---

    #[test]
    fn parse_next_link_with_next() {
        let header = r#"<https://api.github.com/repos?page=2>; rel="next", <https://api.github.com/repos?page=5>; rel="last""#;
        let result = parse_next_link(header, None);
        assert_eq!(result.as_deref(), Some("https://api.github.com/repos?page=2"));
    }

    #[test]
    fn parse_next_link_without_next() {
        let header = r#"<https://api.github.com/repos?page=5>; rel="last""#;
        let result = parse_next_link(header, None);
        assert!(result.is_none());
    }

    #[test]
    fn parse_next_link_empty_string() {
        assert!(parse_next_link("", None).is_none());
    }

    #[test]
    fn parse_next_link_multiple_next_picks_first() {
        let header = r#"<https://api.github.com/repos?page=2>; rel="next", <https://api.github.com/repos?page=3>; rel="next""#;
        let result = parse_next_link(header, None);
        assert_eq!(result.as_deref(), Some("https://api.github.com/repos?page=2"));
    }

    #[test]
    fn parse_next_link_rejects_non_http_scheme() {
        let header = r#"<ftp://evil.com/page>; rel="next""#;
        assert!(parse_next_link(header, None).is_none());
    }

    #[test]
    fn parse_next_link_rejects_no_scheme() {
        let header = r#"//evil.com/page>; rel="next""#;
        assert!(parse_next_link(header, None).is_none());
    }

    #[test]
    fn parse_next_link_rejects_different_host_prefix() {
        let header = r#"<https://evil.com/repos?page=2>; rel="next""#;
        assert_eq!(parse_next_link(header, Some("https://api.github.com")), None);
    }

    #[test]
    fn parse_next_link_accepts_matching_host_prefix() {
        let header = r#"<https://api.github.com/repos?page=2>; rel="next""#;
        assert_eq!(
            parse_next_link(header, Some("https://api.github.com")),
            Some("https://api.github.com/repos?page=2".to_string())
        );
    }

    #[test]
    fn parse_next_link_rejects_subdomain_injection() {
        let header = r#"<https://api.github.com.evil.com/repos?page=2>; rel="next""#;
        assert_eq!(parse_next_link(header, Some("https://api.github.com")), None);
    }
    // --- get_all tests ---

    #[tokio::test]
    async fn get_all_single_page() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/items"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!(["a", "b"]))
            )
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items", None, Duration::from_secs(60), None, |v| v).await.unwrap();
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn get_all_multi_page() {
        let server = MockServer::start().await;

        Mock::given(method("GET")).and(path("/items")).and(query_param("page", "1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!(["a", "b"]))
                    .insert_header("Link", format!(r#"<{}/items?page=2>; rel="next""#, server.uri()))
            )
            .mount(&server).await;

        Mock::given(method("GET")).and(path("/items")).and(query_param("page", "2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!(["c"]))
            )
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items?page=1", None, Duration::from_secs(60), None, |v| v).await.unwrap();
        assert_eq!(result, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[tokio::test]
    async fn get_all_max_pages_cap() {
        let server = MockServer::start().await;

        Mock::given(method("GET")).and(path("/items")).and(query_param("page", "1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!(["a"]))
                    .insert_header("Link", format!(r#"<{}/items?page=2>; rel="next""#, server.uri()))
            )
            .mount(&server).await;

        Mock::given(method("GET")).and(path("/items")).and(query_param("page", "2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!(["b"]))
                    .insert_header("Link", format!(r#"<{}/items?page=3>; rel="next""#, server.uri()))
            )
            .mount(&server).await;

        Mock::given(method("GET")).and(path("/items")).and(query_param("page", "3"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!(["c"]))
            )
            .expect(0)
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items?page=1", None, Duration::from_secs(60), Some(2), |v| v).await.unwrap();
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn get_all_rejects_cross_host_next_link() {
        let server = MockServer::start().await;

        // Page 1 returns items with a next-link pointing to a DIFFERENT host
        Mock::given(method("GET")).and(path("/items"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!(["a", "b"]))
                    .insert_header("Link", r#"<https://evil.com/items?page=2>; rel="next""#)
            )
            .mount(&server).await;

        // No second mock needed — the cross-host link is rejected by parse_next_link,
        // so the server only receives one request (page 1).

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items", None, Duration::from_secs(60), None, |v| v).await.unwrap();
        // Should return only page 1 items — cross-host link is ignored
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn get_all_cache_hit_skips_fetch() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/items"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!(["fetched"]))
            )
            .expect(0)
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        seed(&client, CacheKey::UserRepos, vec!["cached".to_string()]);

        let result: Vec<String> = client.get_all("/items", Some(CacheKey::UserRepos), Duration::from_secs(60), None, |v| v).await.unwrap();
        assert_eq!(result, vec!["cached".to_string()]);
    }

    #[tokio::test]
    async fn get_all_page1_failure_propagates() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/items"))
            .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Result<Vec<String>> = client.get_all("/items", None, Duration::from_secs(60), None, |v| v).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn get_all_partial_failure_returns_partial() {
        let server = MockServer::start().await;

        Mock::given(method("GET")).and(path("/items")).and(query_param("page", "1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!(["a"]))
                    .insert_header("Link", format!(r#"<{}/items?page=2>; rel="next""#, server.uri()))
            )
            .mount(&server).await;

        Mock::given(method("GET")).and(path("/items")).and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items?page=1", None, Duration::from_secs(60), None, |v| v).await.unwrap();
        assert_eq!(result, vec!["a".to_string()]);
    }

    #[derive(serde::Deserialize)]
    struct TestWrapper {
        items: Vec<String>,
    }

    #[tokio::test]
    async fn get_all_extract_single_page() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "items": ["a", "b"] })))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items", None, Duration::from_secs(60), None, |w: TestWrapper| w.items).await.unwrap();
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn get_all_extract_multi_page() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/items")).and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "items": ["a"] }))
                .insert_header("Link", format!(r#"<{}/items?page=2>; rel="next""#, server.uri())))
            .mount(&server).await;
        Mock::given(method("GET")).and(path("/items")).and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "items": ["b"] })))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items?page=1", None, Duration::from_secs(60), None, |w: TestWrapper| w.items).await.unwrap();
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn get_all_extract_cache_hit() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "items": ["fetched"] })))
            .expect(0)
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        seed(&client, CacheKey::Actions { owner: "o".into(), repo: "r".into() }, vec!["cached".to_string()]);
        let result: Vec<String> = client.get_all("/items", Some(CacheKey::Actions { owner: "o".into(), repo: "r".into() }), Duration::from_secs(60), None, |w: TestWrapper| w.items).await.unwrap();
        assert_eq!(result, vec!["cached".to_string()]);
    }

    #[tokio::test]
    async fn get_all_extract_partial_failure() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/items")).and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "items": ["a"] }))
                .insert_header("Link", format!(r#"<{}/items?page=2>; rel="next""#, server.uri())))
            .mount(&server).await;
        Mock::given(method("GET")).and(path("/items")).and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items?page=1", None, Duration::from_secs(60), None, |w: TestWrapper| w.items).await.unwrap();
        assert_eq!(result, vec!["a".to_string()]);
    }

    #[tokio::test]
    async fn get_all_empty_response() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items", None, Duration::from_secs(60), None, |v| v).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn get_all_extract_empty_response() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "items": [] })))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items", None, Duration::from_secs(60), None, |w: TestWrapper| w.items).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn get_all_extract_rejects_cross_host_next_link() {
        let server = MockServer::start().await;

        // Page 1 returns wrapped data with a next-link pointing to a different host
        Mock::given(method("GET")).and(path("/runs"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "items": ["run1"] }))
                    .insert_header("Link", r#"<https://evil.com/runs?page=2>; rel="next""#)
            )
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client
            .get_all(
                "/runs",
                None,
                Duration::from_secs(60),
                None,
                |w: TestWrapper| w.items,
            )
            .await
            .unwrap();

        // Should return only page 1 — cross-host link is ignored
        assert_eq!(result, vec!["run1".to_string()]);
    }

    #[tokio::test]
    async fn get_all_blocks_cross_host_redirect() {
        let server = MockServer::start().await;

        // Page 1: returns a next-link pointing to a redirect endpoint on the same mock server
        Mock::given(method("GET")).and(path("/items"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!(["a"]))
                    .insert_header("Link", format!(r#"<{}/redirect>; rel="next""#, server.uri()))
            )
            .mount(&server).await;

        // The redirect endpoint sends a 302 to an external host
        Mock::given(method("GET")).and(path("/redirect"))
            .respond_with(
                ResponseTemplate::new(302)
                    .insert_header("Location", "https://evil.com/steal-token")
            )
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items", None, Duration::from_secs(60), None, |v| v).await.unwrap();
        // Should return only page 1 — redirect to different host is blocked
        assert_eq!(result, vec!["a".to_string()]);
    }

    #[tokio::test]
    async fn redirect_policy_blocks_different_host() {
        let server = MockServer::start().await;

        Mock::given(method("GET")).and(path("/redirect"))
            .respond_with(
                ResponseTemplate::new(302)
                    .insert_header("Location", "https://evil.com/target")
            )
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result = client.send_and_check(
            client.request(Method::GET, "/redirect")
        ).await;

        // Should fail because redirect to different host is blocked
        // reqwest returns the 302 response, which send_and_check treats as non-2xx
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn redirect_policy_allows_same_host() {
        let server = MockServer::start().await;

        // Redirect to same host
        Mock::given(method("GET")).and(path("/redirect"))
            .respond_with(
                ResponseTemplate::new(302)
                    .insert_header("Location", format!("{}/final", server.uri()))
            )
            .mount(&server).await;

        // Final destination returns 200
        Mock::given(method("GET")).and(path("/final"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!("ok"))
            )
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result = client.send_and_check(
            client.request(Method::GET, "/redirect")
        ).await;

        // Should succeed — same-host redirect is followed
        assert!(result.is_ok());
    }

    #[test]
    fn parse_next_link_rejects_userinfo_url() {
        let header = r#"<https://user@evil.com/repos?page=2>; rel="next""#;
        assert_eq!(parse_next_link(header, Some("https://api.github.com")), None);
    }

    #[test]
    fn parse_next_link_rejects_same_host_userinfo_url() {
        // Same host but with userinfo — should be rejected
        let header = r#"<https://user@api.github.com/repos?page=2>; rel="next""#;
        assert_eq!(parse_next_link(header, Some("https://api.github.com")), None);
    }

    #[test]
    fn parse_next_link_rejects_different_port() {
        let header = r#"<https://api.github.com:8080/repos?page=2>; rel="next""#;
        assert_eq!(parse_next_link(header, Some("https://api.github.com")), None);
    }

    #[test]
    fn parse_next_link_accepts_default_port() {
        // Manual string-splitting treats "api.github.com:443" != "api.github.com"
        // url::Url normalizes default ports (443 for https) so they match
        let header = r#"<https://api.github.com:443/repos?page=2>; rel="next""#;
        assert_eq!(
            parse_next_link(header, Some("https://api.github.com")),
            Some("https://api.github.com:443/repos?page=2".to_string())
        );
    }

    // --- TDD: get_all with extract closure (unified API) ---

    #[tokio::test]
    async fn get_all_with_extract_identity() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/items"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!(["a", "b"]))
            )
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items", None, Duration::from_secs(60), None, |v| v).await.unwrap();
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn get_all_with_extract_wrapper() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/items"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "items": ["x", "y"] }))
            )
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/items", None, Duration::from_secs(60), None, |w: TestWrapper| w.items).await.unwrap();
        assert_eq!(result, vec!["x".to_string(), "y".to_string()]);
    }

    #[tokio::test]
    async fn get_all_with_extract_multi_page() {
        let server = MockServer::start().await;

        Mock::given(method("GET")).and(path("/runs")).and(query_param("page", "1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "items": ["a"] }))
                    .insert_header("Link", format!(r#"<{}/runs?page=2>; rel="next""#, server.uri()))
            )
            .mount(&server).await;

        Mock::given(method("GET")).and(path("/runs")).and(query_param("page", "2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "items": ["b"] }))
            )
            .mount(&server).await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let result: Vec<String> = client.get_all("/runs?page=1", None, Duration::from_secs(60), None, |w: TestWrapper| w.items).await.unwrap();
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }
}
