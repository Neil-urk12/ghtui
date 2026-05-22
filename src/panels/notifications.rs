use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::domain::Notification;
use crate::cmd::Cmd;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::util::format::relative_time;
use crate::widgets::detail_pane::DetailPane;
use crate::widgets::list_view::{render_list_view, LeftContent, ListViewConfig};
use super::panel::{Panel, handle_key_default};

#[derive(Debug)]
pub struct NotificationsPanel {
    pub notifications: Vec<Notification>,
    pub list_state: ListState,
    pub loading: bool,
    pub detail_pane: DetailPane,
    pub show_all: bool,
}

impl Default for NotificationsPanel {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            notifications: Vec::new(),
            list_state,
            loading: false,
            detail_pane: DetailPane::new(),
            show_all: false,
        }
    }
}

impl NotificationsPanel {
    pub fn selected_item(&self) -> Option<&Notification> {
        self.list_state
            .selected()
            .and_then(|i| self.notifications.get(i))
    }

    pub fn set_notifications(&mut self, notifs: Vec<Notification>) {
        self.notifications = notifs;
        self.loading = false;
        self.list_state.select(if self.notifications.is_empty() { None } else { Some(0) });
        self.detail_pane.reset_scroll();
    }

    pub fn view(
        frame: &mut Frame,
        area: Rect,
        state: &NotificationsPanel,
        focused: bool,
        theme: &Theme,
        tick: u64,
    ) {
        // Build items
        let items: Vec<ListItem> = state
            .notifications
            .iter()
            .map(|notif| {
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

        // Build detail lines
        let detail_lines = if let Some(notif) = state.selected_item() {
            let mut content: Vec<Line> = Vec::new();
            content.push(Line::from(Span::styled(
                &notif.subject_title,
                Style::default().add_modifier(Modifier::BOLD),
            )));
            content.push(Line::from(""));
            content.push(Line::from(vec![
                Span::styled("Repository: ", theme.muted),
                Span::raw(&notif.repo_name),
            ]));
            content.push(Line::from(vec![
                Span::styled("Type: ", theme.muted),
                Span::raw(&notif.subject_type),
            ]));
            content.push(Line::from(vec![
                Span::styled("Reason: ", theme.muted),
                Span::raw(&notif.reason),
            ]));
            content.push(Line::from(vec![
                Span::styled("Updated: ", theme.muted),
                Span::raw(relative_time(&notif.updated_at)),
            ]));
            content.push(Line::from(vec![
                Span::styled("Status: ", theme.muted),
                if notif.unread {
                    Span::styled("unread", Style::default().add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("read", theme.muted)
                },
            ]));
            content
        } else {
            vec![]
        };

        let config = ListViewConfig {
            split_ratio: (50, 50),
            margin: 1,
            focused,
            block_title: if state.show_all {
                " Notifications [all] "
            } else {
                " Notifications [unread] "
            }
            .to_string(),
            wrap_full: true,
            loading: state.loading,
            empty_message: "No notifications".to_string(),
            detail_title: " Detail ".to_string(),
            highlight_symbol: "▸ ".to_string(),
            detail_empty_message: "Select a notification to view details".to_string(),
        };

        let left = LeftContent::List {
            items,
            state: &mut state.list_state.clone(),
        };

        render_list_view(
            frame,
            area,
            config,
            left,
            &mut state.detail_pane.clone(),
            detail_lines,
            theme,
            tick,
        );
    }
}

impl Panel for NotificationsPanel {
    fn detail_pane(&mut self) -> &mut DetailPane {
        &mut self.detail_pane
    }

    fn list_len(&self) -> usize {
        self.notifications.len()
    }

    fn selected_index(&self) -> Option<usize> {
        self.list_state.selected()
    }

    fn set_selected(&mut self, index: Option<usize>) {
        self.list_state.select(index);
    }

    fn fetch_cmd(&self) -> Cmd {
        Cmd::FetchNotifications { all: self.show_all }
    }

    fn load_data(&mut self, payload: ApiPayload) {
        if let ApiPayload::Notifications(notifs) = payload {
            self.set_notifications(notifs);
        }
    }

    fn handle_key(&mut self, key: char) -> Option<Cmd> {
        match key {
            'x' => {
                if let Some(notif) = self.selected_item() {
                    let id = notif.id.clone();
                    if !id.is_empty() {
                        return Some(Cmd::MarkNotificationRead { id });
                    }
                }
                None
            }
            'X' => Some(Cmd::MarkAllNotificationsRead),
            '1' => {
                self.show_all = false;
                Some(Cmd::FetchNotifications { all: false })
            }
            '2' => {
                self.show_all = true;
                Some(Cmd::FetchNotifications { all: true })
            }
            _ => handle_key_default(self, key),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool, theme: &Theme, tick: u64) {
        Self::view(frame, area, self, focused, theme, tick);
    }
}
