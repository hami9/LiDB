use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search and sort controls bar
            Constraint::Min(8),    // Processes table
        ])
        .split(area);

    // Filter & Sort Bar
    let filter_text = if app.is_filtering_processes {
        vec![
            Span::styled(
                " [SEARCH ACTIVE] ",
                Style::default()
                    .bg(theme.accent)
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Filter: "),
            Span::styled(
                &app.processes.filter,
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(theme.accent)),
            Span::styled(
                " (Press Enter/Esc to finish)",
                Style::default().fg(theme.fg_muted),
            ),
        ]
    } else {
        vec![
            Span::styled(" Filter: ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                if app.processes.filter.is_empty() {
                    "<all> (press '/' to filter)"
                } else {
                    &app.processes.filter
                },
                Style::default().fg(theme.fg),
            ),
            Span::styled("  │  Sorted by: ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                app.processes.sort_field.label(),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (press 's' to cycle sort)",
                Style::default().fg(theme.fg_muted),
            ),
            Span::styled("  │  Showing: ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                format!(
                    "{}/{}",
                    app.filtered_processes().len(),
                    app.processes.items.len()
                ),
                Style::default().fg(theme.info),
            ),
        ]
    };

    let filter_block =
        Block::default()
            .borders(Borders::ALL)
            .border_style(if app.is_filtering_processes {
                theme.block_border_style(true)
            } else {
                theme.block_border_style(false)
            });
    f.render_widget(
        Paragraph::new(Line::from(filter_text)).block(filter_block),
        chunks[0],
    );

    // Processes Table
    let header_cells = [
        "PID", "USER", "STATE", "CPU %", "MEM %", "RSS", "THREADS", "I/O R/W", "COMMAND",
    ]
    .iter()
    .map(|h| {
        Span::styled(
            *h,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header_row = Row::new(header_cells).style(Style::default().bg(theme.header_bg));

    let filtered = app.filtered_processes();
    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, proc)| {
            let is_selected = i == app.process_selected_idx;

            let state_span = if proc.state == "R" {
                Span::styled(
                    "R",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(&proc.state, Style::default().fg(theme.fg_muted))
            };

            let rss_mb = format!("{:.1} MB", proc.rss_bytes as f64 / 1e6);
            let io_str = format!("{}/{} KB/s", proc.io_read_kb_s, proc.io_write_kb_s);

            let row_cells = vec![
                Span::styled(proc.pid.to_string(), Style::default().fg(theme.info)),
                Span::styled(&proc.user, Style::default().fg(theme.fg_muted)),
                state_span,
                Span::styled(
                    format!("{:.1}%", proc.cpu_pct),
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:.1}%", proc.mem_pct),
                    Style::default().fg(theme.fg),
                ),
                Span::styled(rss_mb, Style::default().fg(theme.fg_muted)),
                Span::styled(
                    proc.threads.to_string(),
                    Style::default().fg(theme.fg_muted),
                ),
                Span::styled(io_str, Style::default().fg(theme.fg_muted)),
                Span::styled(&proc.command, Style::default().fg(theme.fg)),
            ];

            let mut row = Row::new(row_cells);
            if is_selected {
                row = row.style(theme.selected_row_style());
            }
            row
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(18),
            Constraint::Min(24),
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .title(" Process Telemetry (/proc/[pid]/stat & /proc/[pid]/status) ")
            .title_style(theme.title_style())
            .borders(Borders::ALL)
            .border_style(theme.block_border_style(false)),
    );
    let mut table_state = ratatui::widgets::TableState::default();
    if !app.filtered_processes().is_empty() {
        table_state.select(Some(app.process_selected_idx));
    }
    f.render_stateful_widget(table, chunks[1], &mut table_state);
}
