use std::time::Duration;

use color_eyre::Result;

use crate::api::cache::CacheKey;
use crate::api::models::*;
use super::GitHubClient;
use crate::domain;
use crate::constants::{REPO_CACHE_TTL_SECS, API_PER_PAGE};

impl GitHubClient {
    pub async fn get_repos(&self) -> Result<Vec<domain::Repo>> {
        let models: Vec<Repo> = self
            .get_all(
                &format!("/user/repos?sort=updated&per_page={API_PER_PAGE}"),
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
                stars: super::safe_u32(r.stargazers_count),
                forks: super::safe_u32(r.forks_count),
                updated_at: r.updated_at,
                visibility: r.visibility.unwrap_or_default(),
                default_branch: r.default_branch,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::method;
    use super::*;

    #[tokio::test]
    async fn base_url_trailing_slash_produces_correct_url() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/pulls"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server).await;

        let base_url = format!("{}/", server.uri());
        let client = GitHubClient::new_for_test("tok".into(), base_url).unwrap();

        let result = client.get_pulls("o", "r", "open").await;
        assert!(result.is_ok());
    }

    fn minimal_repo_json() -> serde_json::Value {
        serde_json::json!({
            "id": 1,
            "name": "myrepo",
            "full_name": "owner/myrepo",
            "owner": {"login": "owner", "id": 1},
            "description": "desc",
            "language": "Rust",
            "stargazers_count": 10,
            "forks_count": 5,
            "default_branch": "main",
            "visibility": "public",
            "updated_at": "2024-01-01T00:00:00Z",
            "html_url": "https://github.com/owner/myrepo"
        })
    }

    #[tokio::test]
    async fn repo_mapping_null_visibility_defaults_to_empty() {
        let server = MockServer::start().await;
        let mut json = minimal_repo_json();
        json["visibility"] = serde_json::Value::Null;
        Mock::given(method("GET")).and(wiremock::matchers::path("/user/repos"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let repos = client.get_repos().await.unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].visibility, "", "null visibility should map to empty string");
    }

    #[tokio::test]
    async fn repo_mapping_null_description_maps_to_none() {
        let server = MockServer::start().await;
        let mut json = minimal_repo_json();
        json["description"] = serde_json::Value::Null;
        Mock::given(method("GET")).and(wiremock::matchers::path("/user/repos"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let repos = client.get_repos().await.unwrap();
        assert_eq!(repos.len(), 1);
        assert!(repos[0].description.is_none(), "null description should map to None");
    }

    #[tokio::test]
    async fn repo_mapping_null_language_maps_to_none() {
        let server = MockServer::start().await;
        let mut json = minimal_repo_json();
        json["language"] = serde_json::Value::Null;
        Mock::given(method("GET")).and(wiremock::matchers::path("/user/repos"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let repos = client.get_repos().await.unwrap();
        assert_eq!(repos.len(), 1);
        assert!(repos[0].language.is_none(), "null language should map to None");
    }

    #[tokio::test]
    async fn repo_mapping_large_star_count_clamps_to_u32_max() {
        let server = MockServer::start().await;
        let mut json = minimal_repo_json();
        json["stargazers_count"] = serde_json::json!(5_000_000_000u64);
        Mock::given(method("GET")).and(wiremock::matchers::path("/user/repos"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let repos = client.get_repos().await.unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].stars, u32::MAX, "large star count should clamp to u32::MAX");
    }

    #[tokio::test]
    async fn repo_mapping_zero_stars_and_forks() {
        let server = MockServer::start().await;
        let mut json = minimal_repo_json();
        json["stargazers_count"] = serde_json::json!(0);
        json["forks_count"] = serde_json::json!(0);
        Mock::given(method("GET")).and(wiremock::matchers::path("/user/repos"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let repos = client.get_repos().await.unwrap();
        assert_eq!(repos[0].stars, 0);
        assert_eq!(repos[0].forks, 0);
    }
}
