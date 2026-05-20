use std::future::Future;
use std::pin::Pin;

use crate::app::Message;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

pub enum Cmd {
    None,
    FetchRepos,
    FetchPulls { owner: String, repo: String },
    FetchIssues { owner: String, repo: String },
    FetchNotifications,
    FetchActions { owner: String, repo: String },
    Custom(BoxFuture<Message>),
}

impl std::fmt::Debug for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::None => write!(f, "None"),
            Cmd::FetchRepos => write!(f, "FetchRepos"),
            Cmd::FetchPulls { owner, repo } => write!(f, "FetchPulls({}, {})", owner, repo),
            Cmd::FetchIssues { owner, repo } => write!(f, "FetchIssues({}, {})", owner, repo),
            Cmd::FetchNotifications => write!(f, "FetchNotifications"),
            Cmd::FetchActions { owner, repo } => write!(f, "FetchActions({}, {})", owner, repo),
            Cmd::Custom(_) => write!(f, "Custom(<future>)"),
        }
    }
}
