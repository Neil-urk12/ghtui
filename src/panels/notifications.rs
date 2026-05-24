use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::domain::Notification;
use crate::cmd::Cmd;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::util::format::relative_time;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::LeftContent;
use super::panel::{ListPanelData, PanelItem};

#[derive(Debug, Default)]
pub struct NotificationsPanelItem {
    pub show_all: bool,
}

impl PanelItem for NotificationsPanelItem {
    type Item = Notification;

    fn left_content<'a>(
        &'a self,
        data: &'a ListPanelData<Self::Item>,
        theme: &'a Theme,
    ) -> LeftContent<'a> {
        let items: Vec<ListItem> = data
            .filtered_indices
            .iter()
            .map(|&i| {
                let notif = &data.items[i];
                let type_icon = match notif.subject_type.as_str() {
                    "PullRequest" => Span::styled("⇄ ", Color::Magenta),
                    "Issue" => Span::styled("● ", Color::Green),
                    "Discussion" => Span::styled("◈ ", Color::Blue),
                    _ => Span::styled("○ ", Color::DarkGray),
                };

                let repo_style = if notif.unread {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    theme.muted
                };

                let title_style = if notif.unread {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    theme.muted
                };

                let line = Line::from(vec![
                    type_icon,
                    Span::styled(&notif.repo_name, repo_style),
                    Span::raw(" "),
                    Span::styled(&notif.subject_title, title_style),
                    Span::raw(format!("  {}", relative_time(&notif.updated_at))),
                ]);
                ListItem::new(line)
            })
            .collect();
        LeftContent::List { items }
    }

    fn detail_lines(&self, item: &Self::Item, theme: &Theme) -> Vec<Line<'static>> {
        let mut content: Vec<Line> = Vec::new();
        content.push(Line::from(Span::styled(
            item.subject_title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        content.push(Line::from(""));
        content.push(Line::from(vec![
            Span::styled("Repository: ", theme.muted),
            Span::raw(item.repo_name.clone()),
        ]));
        content.push(Line::from(vec![
            Span::styled("Type: ", theme.muted),
            Span::raw(item.subject_type.clone()),
        ]));
        content.push(Line::from(vec![
            Span::styled("Reason: ", theme.muted),
            Span::raw(item.reason.clone()),
        ]));
        content.push(Line::from(vec![
            Span::styled("Updated: ", theme.muted),
            Span::raw(relative_time(&item.updated_at)),
        ]));
        content.push(Line::from(vec![
            Span::styled("Status: ", theme.muted),
            if item.unread {
                Span::styled("unread", Style::default().add_modifier(Modifier::BOLD))
            } else {
                Span::styled("read", theme.muted)
            },
        ]));
        content
    }

    fn handle_key(
        &mut self,
        key: char,
        selected: Option<&Self::Item>,
        _data: &ListPanelData<Self::Item>,
    ) -> Option<Cmd> {
        match key {
            'a' => {
                self.show_all = !self.show_all;
                Some(Cmd::FetchNotifications { all: self.show_all })
            }
            'm' => {
                if let Some(notif) = selected {
                    let id = notif.id.clone();
                    Some(Cmd::MarkNotificationRead { id })
                } else {
                    None
                }
            }
            'M' => Some(Cmd::MarkAllNotificationsRead),
            _ => None,
        }
    }

    fn fetch_cmd(&self, _data: &ListPanelData<Self::Item>) -> Cmd {
        Cmd::FetchNotifications { all: self.show_all }
    }

    fn block_title(&self, _data: &ListPanelData<Self::Item>) -> String {
        if self.show_all {
            " Notifications [all] ".to_string()
        } else {
            " Notifications [unread] ".to_string()
        }
    }

    fn empty_state(&self, _data: &ListPanelData<Self::Item>) -> EmptyState {
        EmptyState::new("No notifications")
            .icon(" ")
            .hint("Press `a` to show all, `r` to refresh")
    }

    fn extract_items(payload: ApiPayload) -> Option<Vec<Self::Item>> {
        if let ApiPayload::Notifications(notifs) = payload {
            Some(notifs)
        } else {
            None
        }
    }

    fn detail_title(&self) -> String {
        " Detail ".to_string()
    }

    fn detail_empty_state(&self) -> EmptyState {
        EmptyState::new("Select a notification")
            .icon(" ")
            .hint("j/k to navigate")
    }
}
