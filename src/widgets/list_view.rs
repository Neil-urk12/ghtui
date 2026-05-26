use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Row, Table};

use super::{
    detail_pane::DetailPane,
    empty_state::{EmptyState, EmptyStateWidget},
    loading::LoadingWidget,
};
use crate::panels::panel::SelectionState;
use crate::theme::Theme;

pub enum LeftContent<'a> {
    Table {
        headers: Vec<&'a str>,
        rows: Vec<Row<'a>>,
        widths: Vec<Constraint>,
    },
    List {
        items: Vec<ListItem<'a>>,
    },
}

pub struct ListViewConfig {
    pub split_ratio: (u16, u16),
    pub margin: u16,
    pub focused: bool,
    pub block_title: String,
    pub wrap_full: bool,
    pub loading: bool,
    pub empty_state: EmptyState,
    pub detail_title: String,
    pub highlight_symbol: String,
    pub detail_empty_state: EmptyState,
}

#[allow(clippy::too_many_arguments)]
pub fn render_list_view(
    frame: &mut Frame,
    area: Rect,
    config: ListViewConfig,
    left: LeftContent<'_>,
    selection: &mut SelectionState,
    detail_pane: &DetailPane,
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
            EmptyStateWidget::render(frame, area, &config.empty_state, block);
            return;
        }

        let chunks = Layout::horizontal([
            Constraint::Percentage(config.split_ratio.0),
            Constraint::Percentage(config.split_ratio.1),
        ])
        .margin(config.margin)
        .split(area);

        render_left(
            frame,
            chunks[0],
            left,
            selection,
            theme,
            &config.highlight_symbol,
        );

        let detail_block = Block::bordered()
            .title(config.detail_title.as_str())
            .border_style(border_style);
        if detail_lines.is_empty() {
            EmptyStateWidget::render(frame, chunks[1], &config.detail_empty_state, detail_block);
        } else {
            detail_pane.render(
                frame,
                chunks[1],
                &config.detail_title,
                detail_lines,
                detail_block,
            );
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
            EmptyStateWidget::render(frame, list_area, &config.empty_state, block);
        } else {
            render_left_with_block(
                frame,
                list_area,
                left,
                block,
                selection,
                theme,
                &config.highlight_symbol,
            );
        }

        let detail_block = Block::default()
            .title(config.detail_title.as_str())
            .borders(Borders::ALL)
            .border_style(border_style);
        if detail_lines.is_empty() {
            EmptyStateWidget::render(frame, detail_area, &config.detail_empty_state, detail_block);
        } else {
            detail_pane.render(
                frame,
                detail_area,
                &config.detail_title,
                detail_lines,
                detail_block,
            );
        }
    }
}

fn render_left(
    frame: &mut Frame,
    area: Rect,
    left: LeftContent<'_>,
    selection: &mut SelectionState,
    theme: &Theme,
    highlight_symbol: &str,
) {
    match left {
        LeftContent::Table {
            headers,
            rows,
            widths,
        } => {
            let header = Row::new(headers).style(theme.muted);
            let table = Table::new(rows, widths)
                .header(header)
                .row_highlight_style(theme.highlight);
            if let SelectionState::Table(inner) = selection {
                StatefulWidget::render(table, area, frame.buffer_mut(), &mut inner.state);
            }
        }
        LeftContent::List { items } => {
            let list = List::new(items)
                .highlight_style(theme.highlight)
                .highlight_symbol(highlight_symbol);
            if let SelectionState::List(inner) = selection {
                StatefulWidget::render(list, area, frame.buffer_mut(), &mut inner.state);
            }
        }
    }
}

fn render_left_with_block(
    frame: &mut Frame,
    area: Rect,
    left: LeftContent<'_>,
    block: Block<'_>,
    selection: &mut SelectionState,
    theme: &Theme,
    highlight_symbol: &str,
) {
    match left {
        LeftContent::Table {
            headers,
            rows,
            widths,
        } => {
            let header = Row::new(headers).style(theme.muted);
            let table = Table::new(rows, widths)
                .block(block)
                .header(header)
                .row_highlight_style(theme.highlight);
            if let SelectionState::Table(inner) = selection {
                StatefulWidget::render(table, area, frame.buffer_mut(), &mut inner.state);
            }
        }
        LeftContent::List { items } => {
            let list = List::new(items)
                .block(block)
                .highlight_style(theme.highlight)
                .highlight_symbol(highlight_symbol);
            if let SelectionState::List(inner) = selection {
                StatefulWidget::render(list, area, frame.buffer_mut(), &mut inner.state);
            }
        }
    }
}
