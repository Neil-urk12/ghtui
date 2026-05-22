use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Row, Table, TableState};

use super::{detail_pane::DetailPane, empty_state::EmptyStateWidget, loading::LoadingWidget};
use crate::theme::Theme;

pub enum LeftContent<'a> {
    Table {
        headers: Vec<&'a str>,
        rows: Vec<Row<'a>>,
        widths: Vec<Constraint>,
        state: &'a mut TableState,
    },
    List {
        items: Vec<ListItem<'a>>,
        state: &'a mut ListState,
    },
}

pub struct ListViewConfig {
    pub split_ratio: (u16, u16),
    pub margin: u16,
    pub focused: bool,
    pub block_title: String,
    pub wrap_full: bool,
    pub loading: bool,
    pub empty_message: String,
    pub detail_title: String,
    pub highlight_symbol: String,
    pub detail_empty_message: String,
}

pub fn render_list_view(
    frame: &mut Frame,
    area: Rect,
    config: ListViewConfig,
    left: LeftContent<'_>,
    detail_pane: &mut DetailPane,
    detail_lines: Vec<Line<'_>>,
    theme: &Theme,
    tick: u64,
) {
    let border_style = if config.focused {
        theme.active_border
    } else {
        theme.inactive_border
    };

    if config.wrap_full {
        // List camp: block wraps full area, split inside with margin
        let block = Block::bordered()
            .title(config.block_title)
            .border_style(border_style);

        if config.loading {
            LoadingWidget::new().render(frame, area, tick, block);
            return;
        }

        let is_empty = match &left {
            LeftContent::Table { rows, .. } => rows.is_empty(),
            LeftContent::List { items, .. } => items.is_empty(),
        };
        if is_empty {
            EmptyStateWidget::render(frame, area, &config.empty_message, block);
            return;
        }

        let chunks = Layout::horizontal([
            Constraint::Percentage(config.split_ratio.0),
            Constraint::Percentage(config.split_ratio.1),
        ])
        .margin(config.margin)
        .split(area);

        render_left(frame, chunks[0], left, theme, &config.highlight_symbol);

        let detail_block = Block::bordered()
            .title(config.detail_title.as_str())
            .border_style(border_style);
        if detail_lines.is_empty() {
            EmptyStateWidget::render(frame, chunks[1], &config.detail_empty_message, detail_block);
        } else {
            detail_pane.render(frame, chunks[1], &config.detail_title, detail_lines, detail_block);
        }
    } else {
        // Table camp: block wraps left side only
        let block = Block::default()
            .title(config.block_title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let [list_area, detail_area] = Layout::horizontal([
            Constraint::Percentage(config.split_ratio.0),
            Constraint::Percentage(config.split_ratio.1),
        ])
        .areas(area);

        if config.loading {
            LoadingWidget::new().render(frame, list_area, tick, block);
            return;
        }

        let is_empty = match &left {
            LeftContent::Table { rows, .. } => rows.is_empty(),
            LeftContent::List { items, .. } => items.is_empty(),
        };
        if is_empty {
            EmptyStateWidget::render(frame, list_area, &config.empty_message, block);
        } else {
            render_left_with_block(frame, list_area, left, block, theme, &config.highlight_symbol);
        }

        let detail_block = Block::default()
            .title(config.detail_title.as_str())
            .borders(Borders::ALL)
            .border_style(border_style);
        if detail_lines.is_empty() {
            EmptyStateWidget::render(frame, detail_area, &config.detail_empty_message, detail_block);
        } else {
            detail_pane.render(frame, detail_area, &config.detail_title, detail_lines, detail_block);
        }
    }
}

fn render_left(frame: &mut Frame, area: Rect, left: LeftContent<'_>, theme: &Theme, highlight_symbol: &str) {
    match left {
        LeftContent::Table {
            headers,
            rows,
            widths,
            state,
        } => {
            let header = Row::new(headers).style(theme.muted);
            let table = Table::new(rows, widths)
                .header(header)
                .row_highlight_style(theme.highlight);
            StatefulWidget::render(table, area, frame.buffer_mut(), state);
        }
        LeftContent::List { items, state } => {
            let list = List::new(items)
                .highlight_style(theme.highlight)
                .highlight_symbol(highlight_symbol);
            StatefulWidget::render(list, area, frame.buffer_mut(), state);
        }
    }
}

fn render_left_with_block(
    frame: &mut Frame,
    area: Rect,
    left: LeftContent<'_>,
    block: Block<'_>,
    theme: &Theme,
    highlight_symbol: &str,
) {
    match left {
        LeftContent::Table {
            headers,
            rows,
            widths,
            state,
        } => {
            let header = Row::new(headers).style(theme.muted);
            let table = Table::new(rows, widths)
                .header(header)
                .block(block)
                .row_highlight_style(theme.highlight);
            StatefulWidget::render(table, area, frame.buffer_mut(), state);
        }
        LeftContent::List { items, state } => {
            let list = List::new(items)
                .block(block)
                .highlight_style(theme.highlight)
                .highlight_symbol(highlight_symbol);
            StatefulWidget::render(list, area, frame.buffer_mut(), state);
        }
    }
}
