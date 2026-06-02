use std::time::Duration;

use color_eyre::Result;

use crate::api::cache::CacheKey;
use crate::api::models::*;
use super::GitHubClient;
use crate::domain;
use crate::constants::{PULLS_CACHE_TTL_SECS, PR_COMMENTS_CACHE_TTL_SECS, API_PER_PAGE};

impl GitHubClient {
    pub async fn get_pulls(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
    ) -> Result<Vec<domain::PullRequest>> {
        let path = format!("/repos/{owner}/{repo}/pulls?state={state}&per_page={API_PER_PAGE}");
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
        let path = format!("/repos/{owner}/{repo}/pulls/{pr_number}/files?per_page={API_PER_PAGE}&page=1");
        let models: Vec<PullRequestFile> = self
            .get_all(
                &path,
                Some(cache_key),
                Duration::from_secs(PR_COMMENTS_CACHE_TTL_SECS),
                Some(100),
                |v| v,
            )
            .await?;
        Ok(models
            .into_iter()
            .map(|f| domain::PrFile {
                filename: f.filename,
                status: f.status,
                additions: f.additions,
                deletions: f.deletions,
                patch: f.patch,
                previous_filename: f.previous_filename,
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
}

#[cfg(test)]
mod tests {
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, query_param};
    use super::*;
    use super::super::test_helpers::{seed, mock_and_client};

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
    async fn approve_pr_preserves_cache_on_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST")).and(path("/repos/o/r/pulls/1/reviews"))
            .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        seed(&client, CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "open".into() }, vec!["stale".to_string()]);
        let result = client.approve_pr("o", "r", 1).await;
        let err = result.unwrap_err();
        assert!(err.to_string().contains("500"), "Expected error to contain status code 500, got: {err}");
        let c = client.cache.lock().unwrap();
        assert!(c.get::<Vec<String>>(&CacheKey::Pulls { owner: "o".into(), repo: "r".into(), state: "open".into() }).is_some());
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
        let files = client.get_pr_files("o", "r", 1).await.unwrap();
        assert_eq!(files.len(), 1);
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

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "a.rs");
    }

    // --- Domain mapping edge-case tests (#13) ---

    fn minimal_pr_json() -> serde_json::Value {
        serde_json::json!({
            "number": 1,
            "title": "Test PR",
            "user": {"login": "author", "id": 1},
            "state": "open",
            "merged_at": null,
            "body": "PR body",
            "html_url": "https://github.com/o/r/pull/1",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-02T00:00:00Z",
            "labels": [{"name": "bug", "color": "red"}],
            "draft": false
        })
    }

    #[tokio::test]
    async fn pr_mapping_empty_labels() {
        let server = MockServer::start().await;
        let mut json = minimal_pr_json();
        json["labels"] = serde_json::json!([]);
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/pulls"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let prs = client.get_pulls("o", "r", "open").await.unwrap();
        assert_eq!(prs.len(), 1);
        assert!(prs[0].labels.is_empty(), "empty labels array should map to empty vec");
    }

    #[tokio::test]
    async fn pr_mapping_null_body() {
        let server = MockServer::start().await;
        let mut json = minimal_pr_json();
        json["body"] = serde_json::Value::Null;
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/pulls"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let prs = client.get_pulls("o", "r", "open").await.unwrap();
        assert_eq!(prs.len(), 1);
        assert!(prs[0].body.is_none(), "null body should map to None");
    }

    #[tokio::test]
    async fn pr_mapping_null_merged_at() {
        let server = MockServer::start().await;
        let mut json = minimal_pr_json();
        json["merged_at"] = serde_json::Value::Null;
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/pulls"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let prs = client.get_pulls("o", "r", "open").await.unwrap();
        assert_eq!(prs.len(), 1);
        assert!(prs[0].merged_at.is_none(), "null merged_at should map to None");
    }

    #[tokio::test]
    async fn pr_mapping_multiple_labels() {
        let server = MockServer::start().await;
        let mut json = minimal_pr_json();
        json["labels"] = serde_json::json!([
            {"name": "bug", "color": "red"},
            {"name": "priority:high", "color": "orange"},
            {"name": "needs-review", "color": "blue"}
        ]);
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/pulls"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let prs = client.get_pulls("o", "r", "open").await.unwrap();
        assert_eq!(prs[0].labels, vec!["bug", "priority:high", "needs-review"]);
    }

    #[tokio::test]
    async fn pr_mapping_null_draft_field() {
        let server = MockServer::start().await;
        let mut json = minimal_pr_json();
        json["draft"] = serde_json::Value::Null;
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/pulls"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let prs = client.get_pulls("o", "r", "open").await.unwrap();
        assert_eq!(prs.len(), 1, "null draft should not cause deserialization failure");
    }

}
