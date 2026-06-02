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

#[cfg(test)]
mod tests {
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::method;
    use super::*;

    fn minimal_issue_json() -> serde_json::Value {
        serde_json::json!({
            "number": 1,
            "title": "Test Issue",
            "user": {"login": "author", "id": 1},
            "state": "open",
            "body": "Issue body",
            "html_url": "https://github.com/o/r/issues/1",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-02T00:00:00Z",
            "labels": [{"name": "bug", "color": "red"}],
            "assignees": []
        })
    }

    #[tokio::test]
    async fn issue_mapping_empty_assignees() {
        let server = MockServer::start().await;
        let json = minimal_issue_json();
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let issues = client.get_issues("o", "r", "open").await.unwrap();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].assignees.is_empty(), "empty assignees should map to empty vec");
    }

    #[tokio::test]
    async fn issue_mapping_null_body() {
        let server = MockServer::start().await;
        let mut json = minimal_issue_json();
        json["body"] = serde_json::Value::Null;
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let issues = client.get_issues("o", "r", "open").await.unwrap();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].body.is_none(), "null body should map to None");
    }

    #[tokio::test]
    async fn issue_mapping_filters_out_pull_requests() {
        let server = MockServer::start().await;
        let issue_json = minimal_issue_json();
        let mut pr_as_issue = minimal_issue_json();
        pr_as_issue["number"] = serde_json::json!(2);
        pr_as_issue["pull_request"] = serde_json::json!({"url": "https://api.github.com/repos/o/r/pulls/2"});
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([issue_json, pr_as_issue])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let issues = client.get_issues("o", "r", "open").await.unwrap();
        assert_eq!(issues.len(), 1, "pull requests should be filtered out");
        assert_eq!(issues[0].number, 1);
    }

    #[tokio::test]
    async fn issue_mapping_with_assignees() {
        let server = MockServer::start().await;
        let mut json = minimal_issue_json();
        json["assignees"] = serde_json::json!([
            {"login": "dev1", "id": 2},
            {"login": "dev2", "id": 3}
        ]);
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let issues = client.get_issues("o", "r", "open").await.unwrap();
        assert_eq!(issues[0].assignees, vec!["dev1", "dev2"]);
    }

    #[tokio::test]
    async fn issue_mapping_empty_labels() {
        let server = MockServer::start().await;
        let mut json = minimal_issue_json();
        json["labels"] = serde_json::json!([]);
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let issues = client.get_issues("o", "r", "open").await.unwrap();
        assert!(issues[0].labels.is_empty(), "empty labels should map to empty vec");
    }

    #[tokio::test]
    async fn issue_mapping_multiple_labels() {
        let server = MockServer::start().await;
        let mut json = minimal_issue_json();
        json["labels"] = serde_json::json!([
            {"name": "bug", "color": "red"},
            {"name": "enhancement", "color": "blue"}
        ]);
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let issues = client.get_issues("o", "r", "open").await.unwrap();
        assert_eq!(issues[0].labels, vec!["bug", "enhancement"]);
    }

    #[tokio::test]
    async fn issue_mapping_closed_state() {
        let server = MockServer::start().await;
        let mut json = minimal_issue_json();
        json["state"] = serde_json::json!("closed");
        Mock::given(method("GET")).and(wiremock::matchers::path("/repos/o/r/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([json])))
            .mount(&server).await;
        let client = GitHubClient::new_for_test("tok".into(), server.uri()).unwrap();
        let issues = client.get_issues("o", "r", "closed").await.unwrap();
        assert_eq!(issues[0].state, "closed");
    }
}
