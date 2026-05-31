use std::any::Any;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::domain::FilterState;

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum CacheKey {
    UserRepos,
    Pulls {
        owner: String,
        repo: String,
        state: String,
    },
    Issues {
        owner: String,
        repo: String,
        state: String,
    },
    Notifications,
    Actions {
        owner: String,
        repo: String,
    },
    PrComments {
        owner: String,
        repo: String,
        pr_number: u64,
    },
    PrReviews {
        owner: String,
        repo: String,
        pr_number: u64,
    },
    PrCodeComments {
        owner: String,
        repo: String,
        pr_number: u64,
    },
    PrFiles {
        owner: String,
        repo: String,
        pr_number: u64,
    },
}

struct CacheEntry {
    data: Box<dyn Any + Send + Sync>,
    inserted_at: Instant,
    ttl: Duration,
}

pub struct Cache {
    entries: HashMap<CacheKey, CacheEntry>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get<T: 'static>(&self, key: &CacheKey) -> Option<&T> {
        let entry = self.entries.get(key)?;
        if entry.inserted_at.elapsed() > entry.ttl {
            return None;
        }
        entry.data.downcast_ref::<T>()
    }

    pub fn insert<T: Send + Sync + 'static>(&mut self, key: CacheKey, data: T, ttl: Duration) {
        self.entries.insert(
            key,
            CacheEntry {
                data: Box::new(data),
                inserted_at: Instant::now(),
                ttl,
            },
        );
    }

    pub fn invalidate(&mut self, key: &CacheKey) {
        self.entries.remove(key);
    }

    pub fn invalidate_pr_related(&mut self, owner: &str, repo: &str, pr_number: u64) {
        for state in [FilterState::Open, FilterState::Closed, FilterState::All] {
            self.invalidate(&CacheKey::Pulls { owner: owner.to_string(), repo: repo.to_string(), state: state.as_str().to_string() });
        }
        self.invalidate(&CacheKey::PrComments { owner: owner.to_string(), repo: repo.to_string(), pr_number });
        self.invalidate(&CacheKey::PrReviews { owner: owner.to_string(), repo: repo.to_string(), pr_number });
        self.invalidate(&CacheKey::PrCodeComments { owner: owner.to_string(), repo: repo.to_string(), pr_number });
        self.invalidate(&CacheKey::PrFiles { owner: owner.to_string(), repo: repo.to_string(), pr_number });
    }

    pub fn invalidate_actions(&mut self, owner: &str, repo: &str) {
        self.invalidate(&CacheKey::Actions { owner: owner.to_string(), repo: repo.to_string() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::constants::TEST_CACHE_TTL;

    #[test]
    fn insert_and_get() {
        let mut cache = Cache::new();
        let key = CacheKey::PrComments {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        cache.insert(key.clone(), "hello".to_string(), TEST_CACHE_TTL);
        let val = cache.get::<String>(&key);
        assert_eq!(val, Some(&"hello".to_string()));
    }

    #[test]
    fn get_returns_none_for_missing_key() {
        let cache = Cache::new();
        let key = CacheKey::PrComments {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 999,
        };
        assert!(cache.get::<String>(&key).is_none());
    }

    #[test]
    fn invalidate_removes_entry() {
        let mut cache = Cache::new();
        let key = CacheKey::PrComments {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        cache.insert(key.clone(), "data".to_string(), TEST_CACHE_TTL);
        cache.invalidate(&key);
        assert!(cache.get::<String>(&key).is_none());
    }

    #[test]
    fn invalidate_nonexistent_key_is_noop() {
        let mut cache = Cache::new();
        let key = CacheKey::PrComments {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 42,
        };
        cache.invalidate(&key);
        assert!(cache.get::<String>(&key).is_none());
    }

    #[test]
    fn expired_entry_returns_none() {
        let mut cache = Cache::new();
        let key = CacheKey::PrComments {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        cache.insert(key.clone(), "stale".to_string(), Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(10));
        assert!(cache.get::<String>(&key).is_none());
    }

    #[test]
    fn pr_cache_keys_invalidate_independently() {
        let mut cache = Cache::new();
        let comments_key = CacheKey::PrComments {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        let reviews_key = CacheKey::PrReviews {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        let code_comments_key = CacheKey::PrCodeComments {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };

        cache.insert(comments_key.clone(), "comments".to_string(), TEST_CACHE_TTL);
        cache.insert(reviews_key.clone(), "reviews".to_string(), TEST_CACHE_TTL);
        cache.insert(code_comments_key.clone(), "code_comments".to_string(), TEST_CACHE_TTL);

        cache.invalidate(&comments_key);

        assert!(cache.get::<String>(&comments_key).is_none());
        assert_eq!(cache.get::<String>(&reviews_key), Some(&"reviews".to_string()));
        assert_eq!(cache.get::<String>(&code_comments_key), Some(&"code_comments".to_string()));
    }

    #[test]
    fn insert_overwrites_existing_key() {
        let mut cache = Cache::new();
        let key = CacheKey::PrComments {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        cache.insert(key.clone(), "first".to_string(), TEST_CACHE_TTL);
        cache.insert(key.clone(), "second".to_string(), TEST_CACHE_TTL);
        assert_eq!(cache.get::<String>(&key), Some(&"second".to_string()));
    }

    #[test]
    fn get_returns_none_for_wrong_type() {
        let mut cache = Cache::new();
        let key = CacheKey::PrComments {
            owner: "o".into(),
            repo: "r".into(),
            pr_number: 1,
        };
        cache.insert(key.clone(), "hello".to_string(), TEST_CACHE_TTL);
        assert!(cache.get::<Vec<u8>>(&key).is_none());
        // Entry still exists with correct type — cache didn't clear it
        assert!(cache.get::<String>(&key).is_some());
    }

    #[test]
    fn works_with_struct_type() {
        let mut cache = Cache::new();
        let key = CacheKey::Pulls {
            owner: "o".into(),
            repo: "r".into(),
            state: "open".into(),
        };
        let data = vec!["a".to_string(), "b".to_string()];
        cache.insert(key.clone(), data.clone(), TEST_CACHE_TTL);
        assert_eq!(cache.get::<Vec<String>>(&key), Some(&data));
    }

    #[test]
    fn invalidate_pr_related_removes_all_pr_caches() {
        let mut cache = Cache::new();
        let ttl = TEST_CACHE_TTL;

        cache.insert(CacheKey::Pulls { owner: "octocat".into(), repo: "hello-world".into(), state: "open".into() }, "open".to_string(), ttl);
        cache.insert(CacheKey::Pulls { owner: "octocat".into(), repo: "hello-world".into(), state: "closed".into() }, "closed".to_string(), ttl);
        cache.insert(CacheKey::Pulls { owner: "octocat".into(), repo: "hello-world".into(), state: "all".into() }, "all".to_string(), ttl);
        cache.insert(CacheKey::PrComments { owner: "octocat".into(), repo: "hello-world".into(), pr_number: 42 }, "comments".to_string(), ttl);
        cache.insert(CacheKey::PrReviews { owner: "octocat".into(), repo: "hello-world".into(), pr_number: 42 }, "reviews".to_string(), ttl);
        cache.insert(CacheKey::PrCodeComments { owner: "octocat".into(), repo: "hello-world".into(), pr_number: 42 }, "code_comments".to_string(), ttl);

        cache.invalidate_pr_related("octocat", "hello-world", 42);

        assert!(cache.get::<String>(&CacheKey::Pulls { owner: "octocat".into(), repo: "hello-world".into(), state: "open".into() }).is_none());
        assert!(cache.get::<String>(&CacheKey::Pulls { owner: "octocat".into(), repo: "hello-world".into(), state: "closed".into() }).is_none());
        assert!(cache.get::<String>(&CacheKey::Pulls { owner: "octocat".into(), repo: "hello-world".into(), state: "all".into() }).is_none());
        assert!(cache.get::<String>(&CacheKey::PrComments { owner: "octocat".into(), repo: "hello-world".into(), pr_number: 42 }).is_none());
        assert!(cache.get::<String>(&CacheKey::PrReviews { owner: "octocat".into(), repo: "hello-world".into(), pr_number: 42 }).is_none());
        assert!(cache.get::<String>(&CacheKey::PrCodeComments { owner: "octocat".into(), repo: "hello-world".into(), pr_number: 42 }).is_none());
    }

    #[test]
    fn invalidate_pr_related_does_not_touch_other_repos() {
        let mut cache = Cache::new();
        let ttl = TEST_CACHE_TTL;

        cache.insert(CacheKey::Pulls { owner: "other".into(), repo: "repo".into(), state: "open".into() }, "other_data".to_string(), ttl);

        cache.invalidate_pr_related("octocat", "hello-world", 1);

        assert_eq!(cache.get::<String>(&CacheKey::Pulls { owner: "other".into(), repo: "repo".into(), state: "open".into() }), Some(&"other_data".to_string()));
    }

    #[test]
    fn invalidate_actions_removes_actions_cache() {
        let mut cache = Cache::new();
        let ttl = TEST_CACHE_TTL;

        cache.insert(CacheKey::Actions { owner: "octocat".into(), repo: "hello-world".into() }, "actions_data".to_string(), ttl);

        cache.invalidate_actions("octocat", "hello-world");

        assert!(cache.get::<String>(&CacheKey::Actions { owner: "octocat".into(), repo: "hello-world".into() }).is_none());
    }
}
