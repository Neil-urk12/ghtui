use crate::app::{Issue, Notification, PanelKind, PullRequest, Repo, WorkflowRun};

#[derive(Debug)]
pub enum Event {
    Input(crossterm::event::Event),
    Tick,
    ApiResult {
        panel: PanelKind,
        result: ApiPayload,
    },
}

#[derive(Debug)]
pub enum ApiPayload {
    Repos(Vec<Repo>),
    Pulls(Vec<PullRequest>),
    Issues(Vec<Issue>),
    Notifications(Vec<Notification>),
    Actions(Vec<WorkflowRun>),
}
