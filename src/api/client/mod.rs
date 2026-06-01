mod actions;
mod issues;
mod notifications;
mod pr_details;
mod pulls;
mod repos;

#[cfg(test)]
mod test_helpers;

use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use reqwest::{Client, Method, redirect::Policy};
use url::Url;

use super::cache::{Cache, CacheKey};

const BASE_URL: &str = "https://api.github.com";
const USER_AGENT: &str = "ghtui";

pub struct GitHubClient {
    http: Client,
    token: String,
    base_url: String,
    pub(crate) cache: Mutex<Cache>,
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
                tracing::warn!(reason = "poisoned", "cache mutex poisoned, recovering");
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

    pub(crate) async fn get<T: serde::de::DeserializeOwned + Clone + Send + Sync + 'static>(
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
    pub(crate) async fn get_all<T, W, F>(
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
                        tracing::warn!(page = page_count + 1, error = %e, "get_all: page failed, returning partial results");
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

    pub(crate) async fn post_empty(&self, path: &str) -> Result<()> {
        self.send_and_check(
            self.request(Method::POST, path)
                .header("Content-Length", "0"),
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn post_json<T: serde::Serialize + ?Sized>(&self, path: &str, body: &T) -> Result<()> {
        self.send_and_check(self.request(Method::POST, path).json(body))
            .await?;
        Ok(())
    }

    pub(crate) async fn patch_empty(&self, path: &str) -> Result<()> {
        self.send_and_check(self.request(Method::PATCH, path))
            .await?;
        Ok(())
    }

    pub(crate) async fn patch_json<T: serde::Serialize + ?Sized>(&self, path: &str, body: &T) -> Result<()> {
        self.send_and_check(self.request(Method::PATCH, path).json(body))
            .await?;
        Ok(())
    }

    pub(crate) async fn put_empty(&self, path: &str) -> Result<()> {
        self.send_and_check(self.request(Method::PUT, path)).await?;
        Ok(())
    }

    pub(crate) fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        self.http
            .request(method, &url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", USER_AGENT)
    }

    pub(crate) async fn send_and_check(&self, builder: reqwest::RequestBuilder) -> Result<reqwest::Response> {
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

pub(crate) fn parse_next_link(header_value: &str, expected_prefix: Option<&str>) -> Option<String> {
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
                            tracing::warn!(url, "parse_next_link: rejecting URL with userinfo");
                            return None;
                        }
                        let url_host = url_parsed.as_ref().and_then(|u| u.host_str().map(String::from));
                        let prefix_host = prefix_parsed.as_ref().and_then(|u| u.host_str().map(String::from));
                        match (url_host, prefix_host) {
                            (Some(uh), Some(ph)) if uh == ph => {
                                let url_port = url_parsed.as_ref().and_then(|u| u.port());
                                let prefix_port = prefix_parsed.as_ref().and_then(|u| u.port());
                                if url_port != prefix_port {
                                    tracing::warn!(url, "parse_next_link: rejecting URL with different port");
                                    return None;
                                }
                            }
                            _ => {
                                tracing::warn!(url, "parse_next_link: rejecting URL with unexpected host");
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
    use super::test_helpers::seed;


    #[tokio::test]
    #[tracing_test::traced_test]
    async fn poisoned_mutex_logs_warn_level() {
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
        let err = result.unwrap_err();
        assert!(err.to_string().contains("500"), "Expected error to contain status code 500, got: {err}");
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
        let err = result.unwrap_err();
        assert!(err.to_string().contains("302"), "Expected error to contain status code 302, got: {err}");
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

    #[tokio::test]
    async fn post_empty_sends_post_with_content_length_zero() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/test/action"))
            .and(wiremock::matchers::header("Content-Length", "0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .expect(1)
            .mount(&server)
            .await;

        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        client.post_empty("/test/action").await.unwrap();
        // Mock verifies: POST method, correct path, Content-Length: 0 header, exactly 1 request
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


#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn valid_next_link_returns_url(
            host in r"[a-z]{3,10}\.[a-z]{2,4}",
            path in r"/[a-z/]{1,30}"
        ) {
            let url = format!("https://{}{}", host, path);
            let header = format!("<{}>; rel=\"next\"", url);
            let result = parse_next_link(&header, None);
            prop_assert_eq!(result, Some(url));
        }

        #[test]
        fn no_next_link_returns_none(
            url in r"https://[a-z]{3,10}\.[a-z]{2,4}/[a-z/]{1,30}",
            rel in r"(last|prev|first)"
        ) {
            let header = format!("<{}>; rel=\"{}\"", url, rel);
            let result = parse_next_link(&header, None);
            prop_assert_eq!(result, None);
        }

        #[test]
        fn empty_string_returns_none(s in "") {
            prop_assert_eq!(parse_next_link(&s, None), None);
        }

        #[test]
        fn non_http_scheme_rejected(
            scheme in r"(ftp|file|data|javascript)",
            host in r"[a-z]{3,10}\.[a-z]{2,4}",
            path in r"/[a-z/]{1,20}"
        ) {
            let url = format!("{}://{}{}", scheme, host, path);
            let header = format!("<{}>; rel=\"next\"", url);
            let result = parse_next_link(&header, None);
            prop_assert_eq!(result, None);
        }
    }
}
