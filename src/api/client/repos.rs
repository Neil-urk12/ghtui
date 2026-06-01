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
}
