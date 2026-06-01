use std::time::Duration;

use color_eyre::Result;

use crate::api::cache::CacheKey;
use crate::api::models::*;
use super::GitHubClient;
use crate::domain;
use crate::constants::{ISSUES_CACHE_TTL_SECS, API_PER_PAGE};

impl GitHubClient {
    pub async fn get_issues(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
    ) -> Result<Vec<domain::Issue>> {
        let path = format!("/repos/{owner}/{repo}/issues?state={state}&per_page={API_PER_PAGE}");
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
}
