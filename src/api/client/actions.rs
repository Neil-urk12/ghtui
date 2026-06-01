use std::time::Duration;

use color_eyre::Result;

use crate::api::cache::CacheKey;
use crate::api::models::*;
use super::GitHubClient;
use crate::domain;
use crate::constants::{ACTIONS_CACHE_TTL_SECS, ACTIONS_PER_PAGE, ACTIONS_MAX_PAGES};

impl GitHubClient {
    pub async fn get_actions(&self, owner: &str, repo: &str) -> Result<Vec<domain::WorkflowRun>> {
        let path = format!("/repos/{owner}/{repo}/actions/runs?per_page={ACTIONS_PER_PAGE}");
        let runs: Vec<WorkflowRun> = self
            .get_all(
                &path,
                Some(CacheKey::Actions {
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                }),
                Duration::from_secs(ACTIONS_CACHE_TTL_SECS),
                Some(ACTIONS_MAX_PAGES),
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
                run_number: super::safe_u32(w.run_number),
            })
            .collect())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_helpers::{seed, mock_and_client};

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
}
