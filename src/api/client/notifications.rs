use std::time::Duration;

use color_eyre::Result;

use crate::api::cache::CacheKey;
use crate::api::models::*;
use super::GitHubClient;
use crate::domain;
use crate::constants::{NOTIFICATIONS_CACHE_TTL_SECS, API_PER_PAGE};

impl GitHubClient {
    pub async fn get_notifications(&self, all: bool) -> Result<Vec<domain::Notification>> {
        let path = if all {
            &format!("/notifications?per_page={API_PER_PAGE}&all=true")
        } else {
            &format!("/notifications?per_page={API_PER_PAGE}")
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_helpers::{seed, mock_and_client};

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
}
