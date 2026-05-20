#[derive(Debug)]
pub enum Event {
    Input(crossterm::event::Event),
    Tick,
    ApiResult { payload: ApiPayload },
    ApiError(String),
}

#[derive(Debug)]
pub enum ApiPayload {
    Repos(Vec<crate::domain::Repo>),
    Pulls(Vec<crate::domain::PullRequest>),
    Issues(Vec<crate::domain::Issue>),
    Notifications(Vec<crate::domain::Notification>),
    Actions(Vec<crate::domain::WorkflowRun>),
    Success(String),
}
