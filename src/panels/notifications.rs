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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Notification;
    use crate::event::ApiPayload;
    use crate::cmd::Cmd;
    use crate::theme::Theme;
    use crate::panels::panel::ListPanelData;

    fn test_notification() -> Notification {
        Notification {
            id: "notif-1".into(),
            repo_name: "owner/repo".into(),
            subject_title: "Fix login bug".into(),
            subject_type: "Issue".into(),
            reason: "mention".into(),
            unread: true,
            updated_at: "2024-01-15T10:30:00Z".into(),
        }
    }

    #[test]
    fn left_content_builds_list_items() {
        let notif = test_notification();
        let data = ListPanelData {
            items: vec![notif],
            filtered_indices: vec![0],
            ..Default::default()
        };
        let panel = NotificationsPanelItem::default();
        let theme = Theme::default();

        let content = panel.left_content(&data, &theme);

        assert!(
            matches!(content, LeftContent::List { ref items } if items.len() == 1),
            "expected List with 1 item, got {:?}",
            std::mem::discriminant(&content)
        );
    }

    #[test]
    fn detail_lines_includes_fields() {
        let notif = test_notification();
        let panel = NotificationsPanelItem::default();
        let theme = Theme::default();

        let lines = panel.detail_lines(&notif, &theme);

        assert!(!lines.is_empty(), "detail_lines should return non-empty Vec");
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Fix login bug"), "should contain subject title");
        assert!(text.contains("owner/repo"), "should contain repo name");
        assert!(text.contains("mention"), "should contain reason");
        assert!(text.contains("unread"), "should contain read status");
    }

    #[test]
    fn handle_key_m_marks_notification_read() {
        let notif = test_notification();
        let mut panel = NotificationsPanelItem::default();
        let data: ListPanelData<Notification> = ListPanelData::default();

        let result = panel.handle_key('m', Some(&notif), &data);

        assert!(
            matches!(result, Some(Cmd::MarkNotificationRead { ref id }) if id == "notif-1"),
            "expected MarkNotificationRead with id 'notif-1', got {:?}",
            result
        );
    }

    #[test]
    fn handle_key_m_returns_none_when_no_selection() {
        let mut panel = NotificationsPanelItem::default();
        let data: ListPanelData<Notification> = ListPanelData::default();

        let result = panel.handle_key('m', None, &data);

        assert!(result.is_none(), "expected None when nothing selected, got {:?}", result);
    }

    #[test]
    fn handle_key_shift_m_marks_all_read() {
        let mut panel = NotificationsPanelItem::default();
        let data: ListPanelData<Notification> = ListPanelData::default();

        let result = panel.handle_key('M', None, &data);

        assert!(
            matches!(result, Some(Cmd::MarkAllNotificationsRead)),
            "expected MarkAllNotificationsRead, got {:?}",
            result
        );
    }

    #[test]
    fn handle_key_a_toggles_show_all_and_fetches() {
        let mut panel = NotificationsPanelItem::default();
        let data: ListPanelData<Notification> = ListPanelData::default();

        assert!(!panel.show_all, "show_all should default to false");

        let result = panel.handle_key('a', None, &data);

        assert!(panel.show_all, "show_all should be toggled to true");
        assert!(
            matches!(result, Some(Cmd::FetchNotifications { all: true })),
            "expected FetchNotifications with all=true, got {:?}",
            result
        );
    }

    #[test]
    fn extract_items_returns_notifications() {
        let notif = test_notification();
        let payload = ApiPayload::Notifications(vec![notif]);

        let result = NotificationsPanelItem::extract_items(payload);

        assert!(
            matches!(result, Some(ref v) if v.len() == 1),
            "expected Some with 1 notification, got {:?}",
            result
        );
    }

    #[test]
    fn extract_items_returns_none_for_repos() {
        let result = NotificationsPanelItem::extract_items(ApiPayload::Repos(vec![]));

        assert!(result.is_none(), "expected None for Repos payload, got {:?}", result);
    }

    #[test]
    fn extract_items_empty_notifications_returns_some_empty() {
        let result = NotificationsPanelItem::extract_items(ApiPayload::Notifications(vec![]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 0);
    }
}

    #[test]
    fn left_content_empty_items() {
        let data = ListPanelData::<Notification>::new();
        let item: NotificationsPanelItem = Default::default();
        let theme = Theme::default();
        let content = item.left_content(&data, &theme);
        match content {
            LeftContent::Table { rows, .. } => assert!(rows.is_empty()),
            LeftContent::List { items, .. } => assert!(items.is_empty()),
        }
    }
