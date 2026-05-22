use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::domain::Notification;
use crate::cmd::Cmd;
use crate::event::ApiPayload;
use crate::theme::Theme;
use crate::util::format::relative_time;
use crate::widgets::detail_pane::DetailPane;
use crate::widgets::empty_state::EmptyState;
use crate::widgets::list_view::{render_list_view, LeftContent, ListViewConfig};
use super::panel::{Panel, SelectionState, handle_key_default, ListPanelData};

#[derive(Debug, Default)]
pub struct NotificationsPanel {
    pub data: ListPanelData<Notification>,
    pub show_all: bool,
}

impl NotificationsPanel {
    #[allow(clippy::too_many_arguments)]
    pub fn view(
        frame: &mut Frame,
        area: Rect,
        state: &NotificationsPanel,
        selection: &mut SelectionState,
        detail_pane: &DetailPane,
        focused: bool,
        theme: &Theme,
        tick: u64,
    ) {
        // Build items
        let items: Vec<ListItem> = state
            .data
            .items
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
        let detail_lines = if let Some(notif) = state.data.selected(selection) {
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
            loading: state.data.loading,
            empty_state: EmptyState::new("No notifications")
                .icon(" ")
                .hint("Press `a` to show all, `r` to refresh"),
            detail_title: " Detail ".to_string(),
            highlight_symbol: "▸ ".to_string(),
            detail_empty_state: EmptyState::new("Select a notification")
                .icon(" ")
                .hint("j/k to navigate"),
        };

        let left = LeftContent::List { items };

        render_list_view(
            frame,
            area,
            config,
            left,
            selection,
            detail_pane,
            detail_lines,
            theme,
            tick,
        );
    }
}

impl Panel for NotificationsPanel {
    fn fetch_cmd(&self) -> Cmd {
        Cmd::FetchNotifications { all: self.show_all }
    }

    fn load_data(&mut self, payload: ApiPayload, selection: &mut SelectionState) {
        if let ApiPayload::Notifications(notifs) = payload {
            self.data.load_items(notifs, selection);
        }
    }

    fn set_loading(&mut self, loading: bool) {
        self.data.loading = loading;
    }

    fn handle_key(&mut self, key: char, selection: &mut SelectionState, detail_pane: &mut DetailPane) -> Option<Cmd> {
        match key {
            'a' => {
                self.show_all = !self.show_all;
                selection.select(if self.data.items.is_empty() { None } else { Some(0) });
                selection.set_len(self.data.items.len());
                None
            }
            'm' => {
                if let Some(notif) = self.data.selected(selection) {
                    let id = notif.id.clone();
                    Some(Cmd::MarkNotificationRead { id })
                } else {
                    None
                }
            }
            'M' => Some(Cmd::MarkAllNotificationsRead),
            _ => handle_key_default(self, key, selection, detail_pane),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, selection: &mut SelectionState, detail_pane: &DetailPane, focused: bool, theme: &Theme, tick: u64) {
        Self::view(frame, area, self, selection, detail_pane, focused, theme, tick);
    }
}
