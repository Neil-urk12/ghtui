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
    pub number: u64,
    pub title: String,
    pub author: String,
    pub state: String,
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub body: Option<String>,
    pub merged_at: Option<String>,
    pub comments: Option<Vec<PrComment>>,
    pub reviews: Option<Vec<PrReview>>,
    pub code_comments: Option<Vec<PrCodeComment>>,
    pub comment_errors: Option<String>,
    pub review_errors: Option<String>,
    pub code_comment_errors: Option<String>,
    pub files: Option<Vec<PrFile>>,
    pub file_errors: Option<String>,
}

#[derive(Debug, Default)]
pub struct Issue {
    pub number: u64,
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
    pub author: String,
    pub body: String,
}

#[derive(Debug, Default, Clone)]
pub struct PrReview {
    pub author: String,
    pub state: String,
    pub body: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct PrCodeComment {
    pub body: String,
    pub path: String,
    pub line: Option<u32>,
}

#[derive(Debug, Default, Clone)]
pub struct PrFile {
    pub filename: String,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
    pub patch: Option<String>,
    pub previous_filename: Option<String>,
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
                tracing::warn!(state = %other, "unknown filter state, defaulting to Open");
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

    #[test]
    fn pull_request_number_accepts_large_values() {
        let pr = PullRequest {
            number: 5_000_000_000,
            ..Default::default()
        };
        assert_eq!(pr.number, 5_000_000_000);
    }

    #[test]
    fn issue_number_accepts_large_values() {
        let issue = Issue {
            number: 5_000_000_000,
            ..Default::default()
        };
        assert_eq!(issue.number, 5_000_000_000);
    }

    #[test]
    fn pull_request_has_merged_at_field() {
        let pr = PullRequest {
            number: 1,
            title: "test".into(),
            author: "user".into(),
            state: "closed".into(),
            labels: vec![],
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
            body: None,
            comments: None,
            reviews: None,
            code_comments: None,
            comment_errors: None,
            review_errors: None,
            files: None,
            file_errors: None,
            code_comment_errors: None,
            merged_at: Some("2024-01-02T00:00:00Z".into()),
        };
        assert!(pr.merged_at.is_some());
    }
}
