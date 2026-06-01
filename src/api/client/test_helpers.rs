use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

use super::GitHubClient;
use crate::api::cache::CacheKey;

pub fn seed<T: Send + Sync + 'static>(client: &GitHubClient, key: CacheKey, val: T) {
    let mut c = client.cache.lock().unwrap();
    c.insert(key, val, crate::constants::TEST_CACHE_TTL);
}

pub async fn mock_and_client(method_match: &str, path_match: &str) -> (MockServer, GitHubClient) {
    let server = MockServer::start().await;
    Mock::given(method(method_match))
        .and(path(path_match))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;
    let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
    (server, client)
}
