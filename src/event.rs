#[derive(Debug)]
pub enum Event {
    Input(crossterm::event::Event),
    Tick,
    ApiResult { payload: ApiPayload },
    ApiError(String),
}

#[derive(Debug)]
pub enum ApiPayload {
    Repos(Vec<crate::app::Repo>),
    Pulls(Vec<crate::app::PullRequest>),
    Issues(Vec<crate::app::Issue>),
    Notifications(Vec<crate::app::Notification>),
    Actions(Vec<crate::app::WorkflowRun>),
    Success(String),
}
