use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

use ratatui::prelude::*;
use ratatui::style::Color;
use ratatui::widgets::{Cell, Row};

use super::panel::{ListPanelData, PanelItem, SelectionState, StateFilterable};
use crate::cmd::Cmd;
use crate::domain::{FilterState, PullRequest};
use crate::theme::{Theme, status_style};
use crate::util::diff::parse_diff_patch;
use crate::util::format;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::LeftContent;


fn pr_to_diff(pr: &PullRequest, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if let Some(files) = &pr.files {
        for (i, file) in files.iter().enumerate() {
            let prefix = match (file.status.as_str(), &file.previous_filename) {
                ("renamed", Some(old_name)) => format!("{} → ", old_name),
                _ => String::new(),
            };
            let header = format!("{}{}  (+{} -{})  {}", prefix, file.filename, file.additions, file.deletions, file.status);
            lines.push(Line::from(Span::styled(
                header,
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            if let Some(patch) = &file.patch {
                lines.extend(parse_diff_patch(patch, theme));
            } else {
                lines.push(Line::from(Span::styled(
                    "(diff too large — view on GitHub)",
                    theme.muted,
                )));
            }

            if i + 1 < files.len() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "─".repeat(crate::constants::DIFF_SEPARATOR_WIDTH),
                    theme.muted,
                )));
                lines.push(Line::from(""));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "d:overview | o:browser | j/k:scroll",
            theme.muted,
        )));
    }

    lines
}

fn pr_to_row(pr: &PullRequest, theme: &Theme) -> Row<'static> {
    let status_str = derive_pr_status(pr);
    let status_st = status_style(status_str, theme);
    let updated = format::relative_time(&pr.updated_at);
    Row::new(vec![
        Cell::from(format!("#{}", pr.number)),
        Cell::from(pr.title.clone()),
        Cell::from(pr.author.clone()),
        Cell::from(Span::styled(status_str, status_st)),
        Cell::from(updated),
    ])
}

/// Derives the display status: "merged", "closed", or "open".
fn derive_pr_status(pr: &PullRequest) -> &'static str {
    if pr.merged_at.is_some() {
        "merged"
    } else if pr.state == "closed" {
        "closed"
    } else {
        "open"
    }
}

/// Returns true if the PR can be reopened (not merged).
fn can_reopen(pr: &PullRequest) -> bool {
    pr.state == "closed" && pr.merged_at.is_none()
}

/// Returns true if the PR can be merged (must be open).
fn can_merge(pr: &PullRequest) -> bool {
    pr.state == "open"
}

/// Returns true if the PR can be closed (must be open).
fn can_close(pr: &PullRequest) -> bool {
    pr.state == "open"
}

fn pr_to_detail(pr: &PullRequest, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            format!("#{} {}", pr.number, pr.title),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "by {} | opened {} | {}",
            pr.author,
            format::relative_time(&pr.created_at),
            format::relative_time(&pr.updated_at)
        )),
        Line::from(""),
    ];

    if !pr.labels.is_empty() {
        let labels: Vec<Span> = pr
            .labels
            .iter()
            .map(|l| Span::styled(format!("[{l}]"), Style::default().fg(Color::Yellow)))
            .collect();
        let mut label_line = vec![Span::raw("Labels: ")];
        label_line.extend(labels);
        lines.push(Line::from(label_line));
        lines.push(Line::from(""));
    }

    if let Some(body) = &pr.body {
        let rendered = crate::util::markdown::render_markdown(body);
        lines.extend(rendered);
    } else {
        lines.push(Line::from(Span::styled("(no description)", theme.muted)));
    }

    // Render reviews
    if let Some(reviews) = &pr.reviews {
        if !reviews.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("Reviews ({})", reviews.len()),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for review in reviews {
                let icon = match review.state.as_str() {
                    "APPROVED" => "✓",
                    "CHANGES_REQUESTED" | "DISMISSED" => "✗",
                    _ => "•",
                };
                lines.push(Line::from(format!(
                    "{} {}: {}",
                    icon, review.author, review.state
                )));
                if let Some(body) = &review.body {
                    if !body.is_empty() {
                        lines.push(Line::from(format!("  {}", body)));
                    }
                }
            }
            lines.push(Line::from(""));
        }
    }

    // Render comments
    if let Some(comments) = &pr.comments {
        if !comments.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("Comments ({})", comments.len()),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for comment in comments {
                lines.push(Line::from(format!("{}: {}", comment.author, comment.body)));
            }
            lines.push(Line::from(""));
        }
    }

    // Render code comments
    if let Some(code_comments) = &pr.code_comments {
        if !code_comments.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("Code Comments ({})", code_comments.len()),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for comment in code_comments {
                let location = if let Some(line) = comment.line {
                    format!("{}:{}", comment.path, line)
                } else {
                    comment.path.clone()
                };
                lines.push(Line::from(format!("{}: {}", location, comment.body)));
            }
            lines.push(Line::from(""));
        }
    }

    // Render partial error indicators
    if let Some(err) = &pr.review_errors {
        lines.push(Line::from(format!(
            "[WARN] Reviews: failed to load ({})",
            err
        )));
    }
    if let Some(err) = &pr.comment_errors {
        lines.push(Line::from(format!(
            "[WARN] Comments: failed to load ({})",
            err
        )));
    }
    if let Some(err) = &pr.code_comment_errors {
        lines.push(Line::from(format!(
            "[WARN] Code Comments: failed to load ({})",
            err
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "1:open 2:closed 3:all | o:browser | d:diff | f:fullscreen | c:collapse | a:approve | m:merge | x:close | X:reopen",
        theme.muted,
    )));

    lines
}

#[derive(Debug)]
pub struct PullsPanelItem {
    pub state_filter: FilterState,
    last_fetch_attempts: RefCell<HashMap<u64, Instant>>,
}

impl Default for PullsPanelItem {
    fn default() -> Self {
        Self {
            state_filter: FilterState::default(),
            last_fetch_attempts: RefCell::new(HashMap::new()),
        }
    }
}

impl PullsPanelItem {
    pub fn new(state_filter: FilterState) -> Self {
        Self {
            state_filter,
            last_fetch_attempts: RefCell::new(HashMap::new()),
        }
    }


    fn fetch_pulls_cmd(
        &self,
        data: &ListPanelData<PullRequest>,
    ) -> Option<Cmd> {
        let ctx = data.repo_context()?;
        Some(Cmd::FetchPulls {
            ctx,
            state: self.state_filter,
        })
    }
}

impl StateFilterable for PullsPanelItem {
    fn filter_state(&self) -> &FilterState {
        &self.state_filter
    }

    fn filter_state_mut(&mut self) -> &mut FilterState {
        &mut self.state_filter
    }
}

impl PanelItem for PullsPanelItem {
    type Item = PullRequest;

    fn left_content<'a>(
        &'a self,
        data: &'a ListPanelData<Self::Item>,
        theme: &'a Theme,
    ) -> LeftContent<'a> {
        let rows: Vec<Row> = data
            .filtered_indices
            .iter()
            .map(|&i| pr_to_row(&data.items[i], theme))
            .collect();
        LeftContent::Table {
            headers: vec!["#", "Title", "Author", "Status", "Updated"],
            rows,
            widths: vec![
                Constraint::Length(6),
                Constraint::Fill(3),
                Constraint::Fill(1),
                Constraint::Length(8),
                Constraint::Fill(1),
            ],
        }
    }

    fn detail_lines(&self, item: &Self::Item, theme: &Theme) -> Vec<Line<'static>> {
        pr_to_detail(item, theme)
    }

    fn diff_lines(&self, item: &Self::Item, theme: &Theme) -> Vec<Line<'static>> {
        match &item.files {
            Some(files) if !files.is_empty() => pr_to_diff(item, theme),
            _ => vec![],
        }
    }
    fn handle_key(
        &mut self,
        key: char,
        selected: Option<&Self::Item>,
        data: &ListPanelData<Self::Item>,
        detail_pane: &mut crate::widgets::detail_pane::DetailPane,
    ) -> Option<Cmd> {
        match key {
            'd' => {
                if selected.is_some() {
                    detail_pane.toggle_diff();
                }
                Some(Cmd::None)
            }
            'f' => {
                detail_pane.toggle_fullscreen();
                Some(Cmd::None)
            }
            'c' => {
                detail_pane.toggle_collapse();
                Some(Cmd::None)
            }
            '1' => {
                if self.set_filter(FilterState::Open) {
                    self.fetch_pulls_cmd(data)
                } else {
                    None
                }
            }
            '2' => {
                if self.set_filter(FilterState::Closed) {
                    self.fetch_pulls_cmd(data)
                } else {
                    None
                }
            }
            '3' => {
                if self.set_filter(FilterState::All) {
                    self.fetch_pulls_cmd(data)
                } else {
                    None
                }
            }
            'o' => {
                if let Some(pr) = selected {
                    if let Some((owner, repo)) = data.owner_repo() {
                        crate::util::open_in_browser(pr.number, owner, repo);
                    }
                }
                None
            }
            'a' => {
                if let Some(pr) = selected {
                    if let Some(ctx) = data.repo_context() {
                        return Some(Cmd::ApprovePr {
                            ctx,
                            pr_number: pr.number,
                        });
                    }
                }
                None
            }
            'm' => {
                if let Some(pr) = selected {
                    if can_merge(pr) {
                        if let Some(ctx) = data.repo_context() {
                            return Some(Cmd::MergePr {
                                ctx,
                                pr_number: pr.number,
                            });
                        }
                    }
                }
                None
            }

            'x' => {
                if let Some(pr) = selected {
                    if can_close(pr) {
                        if let Some(ctx) = data.repo_context() {
                            return Some(Cmd::ClosePr {
                                ctx,
                                pr_number: pr.number,
                            });
                        }
                    }
                }
                None
            }
            'X' => {
                if let Some(pr) = selected {
                    if can_reopen(pr) {
                        if let Some(ctx) = data.repo_context() {
                            return Some(Cmd::ReopenPr {
                                ctx,
                                pr_number: pr.number,
                            });
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn on_select(
        &self,
        selected: Option<&Self::Item>,
        data: &ListPanelData<Self::Item>,
    ) -> Option<Cmd> {

        if let Some(pr) = selected {
            let needs_fetch =
                pr.comments.is_none() || pr.reviews.is_none() || pr.code_comments.is_none() || pr.files.is_none();
            if needs_fetch {
                let pr_number = pr.number;
                let within_debounce = self
                    .last_fetch_attempts
                    .borrow()
                    .get(&pr_number)
                    .map(|last| last.elapsed() < crate::constants::FETCH_DEBOUNCE)
                    .unwrap_or(false);

                if within_debounce {
                    return None;
                }

                if let Some(ctx) = data.repo_context() {
                    self.last_fetch_attempts
                        .borrow_mut()
                        .insert(pr_number, Instant::now());
                    return Some(Cmd::FetchPrComments {
                        ctx,
                        pr_number,
                    });
                }
            }
        }
        None
    }

    fn fetch_cmd(&self, data: &ListPanelData<Self::Item>) -> Cmd {
        if let Some(ctx) = data.repo_context() {
            Cmd::FetchPulls {
                ctx,
                state: self.state_filter,
            }
        } else {
            Cmd::None
        }
    }

    fn block_title(&self, data: &ListPanelData<Self::Item>) -> String {
        if let Some((o, r)) = data.owner_repo() {
            format!(" PRs: {o}/{r} ({}) ", self.state_filter.as_str())
        } else {
            format!(" Pull Requests ({}) ", self.state_filter.as_str())
        }
    }

    fn empty_state(&self, data: &ListPanelData<Self::Item>) -> EmptyState {
        data.repo_aware_empty_state(
            "No pull requests found",
            " ",
            "Press 1 for open, 2 for closed, 3 for all · r to refresh",
        )
    }

    extract_items!(Pulls);

    fn split_ratio(&self) -> (u16, u16) {
        (50, 50)
    }

    fn detail_title(&self, showing_diff: bool) -> String {
        if showing_diff {
            " PR Diff ".to_string()
        } else {
            " PR Details ".to_string()
        }
    }

    fn detail_empty_state(&self) -> EmptyState {
        EmptyState::new("Select a pull request")
            .icon(" ")
            .hint("j/k to navigate")
    }

    fn diff_empty_state(&self) -> EmptyState {
        EmptyState::new("No diff loaded")
            .icon(" ")
            .hint("Press Enter to fetch PR files, then 'd' for diff")
    }

    fn apply_filter(
        &self,
        data: &mut ListPanelData<Self::Item>,
        query: &str,
        selection: &mut SelectionState,
    ) {
        data.filter_by(
            query,
            |item, q| {
                item.title.to_lowercase().contains(q)
                    || item.author.to_lowercase().contains(q)
                    || item.labels.iter().any(|l| l.to_lowercase().contains(q))
            },
            selection,
        );
    }

    fn initial_selection(&self) -> SelectionState {
        SelectionState::table()
    }

    fn item_display(&self, item: &Self::Item) -> Option<String> {
        Some(format!("#{} {}", item.number, item.title))
    }
}

#[cfg(test)]
mod tests;
