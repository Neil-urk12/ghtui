#[derive(Debug, Default)]
pub struct Repo {
    pub name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stars: u32,
    pub forks: u32,
    pub updated_at: String,
    pub visibility: String,
    pub default_branch: String,
}

#[derive(Debug, Default, Clone)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub state: String,
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub body: Option<String>,
    pub comments: Option<Vec<PrComment>>,
    pub reviews: Option<Vec<PrReview>>,
    pub code_comments: Option<Vec<PrCodeComment>>,
}

#[derive(Debug, Default)]
pub struct Issue {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub state: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub body: Option<String>,
}
#[derive(Debug, Default, Clone)]
pub struct PrComment {
    pub id: u64,
    pub author: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Default, Clone)]
pub struct PrReview {
    pub id: u64,
    pub author: String,
    pub state: String,
    pub body: Option<String>,
    pub submitted_at: String,
}

#[derive(Debug, Default, Clone)]
pub struct PrCodeComment {
    pub id: u64,
    pub author: String,
    pub body: String,
    pub path: String,
    pub line: Option<u32>,
    pub created_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct Notification {
    pub id: String,
    pub repo_name: String,
    pub subject_title: String,
    pub subject_type: String,
    pub reason: String,
    pub unread: bool,
    pub updated_at: String,
}

#[derive(Debug, Default)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub branch: String,
    pub event: String,
    pub created_at: String,
    pub run_number: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterState {
    #[default]
    Open,
    Closed,
    All,
}

impl FilterState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::All => "all",
        }
    }
}

impl From<&str> for FilterState {
    fn from(s: &str) -> Self {
        match s {
            "open" => Self::Open,
            "closed" => Self::Closed,
            "all" => Self::All,
            other => {
                tracing::warn!("unknown filter state '{other}', defaulting to Open");
                Self::default()
            }
        }
    }
}

impl std::fmt::Display for FilterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_state_from_str() {
        assert_eq!(FilterState::from("open"), FilterState::Open);
        assert_eq!(FilterState::from("closed"), FilterState::Closed);
        assert_eq!(FilterState::from("all"), FilterState::All);
        assert_eq!(FilterState::from("invalid"), FilterState::Open); // default
    }

    #[test]
    fn filter_state_as_str() {
        assert_eq!(FilterState::Open.as_str(), "open");
        assert_eq!(FilterState::Closed.as_str(), "closed");
        assert_eq!(FilterState::All.as_str(), "all");
    }

    #[test]
    fn filter_state_default_is_open() {
        assert_eq!(FilterState::default(), FilterState::Open);
    }
}