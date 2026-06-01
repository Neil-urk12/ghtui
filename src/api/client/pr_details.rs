use std::time::Duration;

use color_eyre::Result;

use crate::api::cache::CacheKey;
use crate::api::models::*;
use super::GitHubClient;
use crate::domain;
use crate::constants::{PR_COMMENTS_CACHE_TTL_SECS, API_PER_PAGE};

impl GitHubClient {
    pub async fn get_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<domain::PrComment>> {
        let path = format!("/repos/{owner}/{repo}/issues/{pr_number}/comments?per_page={API_PER_PAGE}");
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
        let path = format!("/repos/{owner}/{repo}/pulls/{pr_number}/reviews?per_page={API_PER_PAGE}");
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
        let path = format!("/repos/{owner}/{repo}/pulls/{pr_number}/comments?per_page={API_PER_PAGE}");
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
}
