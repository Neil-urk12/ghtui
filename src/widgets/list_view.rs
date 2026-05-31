use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Row, Table};

use super::{
    detail_pane::{DetailPane, DetailPaneMode},
    empty_state::{EmptyState, EmptyStateWidget},
    loading::LoadingWidget,
};
use crate::widgets::selection::SelectionState;
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
    pub detail_mode: DetailPaneMode,
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
            .title(config.block_title.as_str())
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

        let inner = Rect {
            x: area.x + config.margin,
            y: area.y + config.margin,
            width: area.width.saturating_sub(config.margin * 2),
            height: area.height.saturating_sub(config.margin * 2),
        };

        let detail_block = Block::bordered()
            .title(config.detail_title.as_str())
            .border_style(border_style);

        render_with_detail_mode(
            frame,
            inner,
            &config,
            detail_pane,
            detail_lines,
            detail_block,
            |frame, left_area| {
                render_left(
                    frame,
                    left_area,
                    left,
                    None,
                    selection,
                    theme,
                    &config.highlight_symbol,
                );
            },
        );
    } else {
        // Table camp: block wraps left side only
        let block = Block::default()
            .title(config.block_title.as_str())
            .borders(Borders::ALL)
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

        let detail_block = Block::default()
            .title(config.detail_title.as_str())
            .borders(Borders::ALL)
            .border_style(border_style);

        render_with_detail_mode(
            frame,
            area,
            &config,
            detail_pane,
            detail_lines,
            detail_block,
            |frame, left_area| {
                render_left(
                    frame,
                    left_area,
                    left,
                    Some(block),
                    selection,
                    theme,
                    &config.highlight_symbol,
                );
            },
        );

    }
}

/// Handles the 3-way `DetailPaneMode` layout split and detail rendering.
///
/// The caller provides the pre-computed area (inner rect for wrap_full, raw area for table),
/// a pre-built detail block, and a closure that renders the left side into a given area.
#[allow(clippy::too_many_arguments)]
fn render_with_detail_mode(
    frame: &mut Frame,
    area: Rect,
    config: &ListViewConfig,
    detail_pane: &DetailPane,
    detail_lines: Vec<Line<'_>>,
    detail_block: Block<'_>,
    render_left_fn: impl FnOnce(&mut Frame, Rect),
) {
    match config.detail_mode {
        DetailPaneMode::Collapsed => {
            // Left takes full area; detail hidden
            render_left_fn(frame, area);
        }
        DetailPaneMode::Fullscreen => {
            // Detail covers entire area; left hidden
            if detail_lines.is_empty() {
                EmptyStateWidget::render(frame, area, &config.detail_empty_state, detail_block);
            } else {
                detail_pane.render(frame, area, &config.detail_title, detail_lines, detail_block);
            }
        }
        DetailPaneMode::Normal => {
            // Split horizontally, render both sides
            let [left_area, detail_area] = Layout::horizontal([
                Constraint::Percentage(config.split_ratio.0),
                Constraint::Percentage(config.split_ratio.1),
            ])
            .areas(area);

            render_left_fn(frame, left_area);

            if detail_lines.is_empty() {
                EmptyStateWidget::render(
                    frame,
                    detail_area,
                    &config.detail_empty_state,
                    detail_block,
                );
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
}

fn render_left(
    frame: &mut Frame,
    area: Rect,
    left: LeftContent<'_>,
    block: Option<Block<'_>>,
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
            let mut table = Table::new(rows, widths)
                .header(header)
                .row_highlight_style(theme.highlight);
            if let Some(b) = block {
                table = table.block(b);
            }
            if let SelectionState::Table(inner) = selection {
                StatefulWidget::render(table, area, frame.buffer_mut(), &mut inner.state);
            }
        }
        LeftContent::List { items } => {
            let mut list = List::new(items)
                .highlight_style(theme.highlight)
                .highlight_symbol(highlight_symbol);
            if let Some(b) = block {
                list = list.block(b);
            }
            if let SelectionState::List(inner) = selection {
                StatefulWidget::render(list, area, frame.buffer_mut(), &mut inner.state);
            }
        }
    }
}
