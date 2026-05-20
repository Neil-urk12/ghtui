use std::any::Any;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum CacheKey {
    UserRepos,
    Pulls { owner: String, repo: String },
    Issues { owner: String, repo: String },
    Notifications,
    Actions { owner: String, repo: String },
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
}
