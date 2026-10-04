use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, AppState};
use crate::panels::login::render_login;
use crate::widgets::popup;
use crate::widgets::status_bar::render_status_bar;
use crate::widgets::tab_bar::render_tab_bar;

pub fn view(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Login screen — render instead of main UI
    if matches!(app.state, AppState::Login) {
        if let Some(ref login) = app.login {
            render_login(frame, login);
        } else {
            let loading = Paragraph::new("Initializing login...").alignment(Alignment::Center);
            frame.render_widget(loading, area);
        }
        return;
    }

    // Main layout: tab bar, content, search/status
    let (content_height, status_height) = if app.search.active {
        (Constraint::Fill(1), Constraint::Length(2))
    } else {
        (Constraint::Fill(1), Constraint::Length(1))
    };

    let layout =
        Layout::vertical([Constraint::Length(1), content_height, status_height]).split(area);

    render_tab_bar(frame, layout[0], app);

    // Content area - delegate to active panel
    let content_area = layout[1];
    if let Some(entry) = app.panels.get_mut(&app.active_panel) {
        entry.render(frame, content_area, &app.theme, app.tick);
    }

    // Status area + optional search bar
    let status_area = layout[2];
    if app.search.active {
        let status_layout =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(status_area);

        // Search bar
        let search_text = format!("/{}", app.search.query);
        let search = Paragraph::new(search_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default());
        frame.render_widget(search, status_layout[0]);

        render_status_bar(frame, status_layout[1], app);
    } else {
        render_status_bar(frame, status_area, app);
    }

    // Error toast overlay
    if let Some(ref err) = app.error_toast {
        let severity = popup::classify_severity(err);
        popup::render_error_toast(frame, area, err, severity, app.toast_tick);
    }

    // Success toast overlay
    if let Some(ref msg) = app.success_toast {
        popup::render_success_toast(frame, area, msg, app.toast_tick);
    }

    // Help overlay
    if app.show_help {
        popup::render_help(frame, area, &app.active_panel.to_string());
    }

    // Confirmation modal overlay
    if let Some(ref confirm) = app.confirmation {
        popup::render_confirmation(frame, area, &confirm.action_label, &confirm.detail);
    }
}
