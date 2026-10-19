use crate::{app::App, model::diagnostics::DiagnosticStatus};
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
            Constraint::Length(10), // Checks table
            Constraint::Min(6),     // Selected check details & remediation
        ])
        .split(area);

    // Section 1: Checks Table
    let header_cells = [
        "ID",
        "CATEGORY",
        "STATUS",
        "DIAGNOSTIC CHECK",
        "OBSERVATION",
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

    let rows: Vec<Row> = app
        .diagnostics
        .checks
        .iter()
        .enumerate()
        .map(|(i, check)| {
            let is_selected = i == app.diagnostics_selected_idx;

            let status_span = match check.status {
                DiagnosticStatus::Pass => Span::styled(
                    " PASS ",
                    Style::default()
                        .bg(theme.success)
                        .fg(theme.bg)
                        .add_modifier(Modifier::BOLD),
                ),
                DiagnosticStatus::SimulatedPass => Span::styled(
                    " SIM-PASS ",
                    Style::default()
                        .bg(theme.info)
                        .fg(theme.bg)
                        .add_modifier(Modifier::BOLD),
                ),
                DiagnosticStatus::Warn => Span::styled(
                    " WARN ",
                    Style::default()
                        .bg(theme.warning)
                        .fg(theme.bg)
                        .add_modifier(Modifier::BOLD),
                ),
                DiagnosticStatus::Fail => Span::styled(
                    " FAIL ",
                    Style::default()
                        .bg(theme.error)
                        .fg(theme.bg)
                        .add_modifier(Modifier::BOLD),
                ),
                DiagnosticStatus::Unavailable => Span::styled(
                    " N/A  ",
                    Style::default()
                        .bg(theme.selected_bg)
                        .fg(theme.fg_muted)
                        .add_modifier(Modifier::BOLD),
                ),
                DiagnosticStatus::NotProbed => Span::styled(
                    " UNPROBED ",
                    Style::default()
                        .bg(theme.selected_bg)
                        .fg(theme.fg_muted)
                        .add_modifier(Modifier::BOLD),
                ),
            };

            let row_cells = vec![
                Span::styled(&check.id, Style::default().fg(theme.info)),
                Span::styled(&check.category, Style::default().fg(theme.fg_muted)),
                status_span,
                Span::styled(
                    &check.name,
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&check.finding, Style::default().fg(theme.fg)),
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
            Constraint::Length(10),
            Constraint::Length(18),
            Constraint::Length(12),
            Constraint::Length(26),
            Constraint::Min(30),
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .title(" LiDB Health & Environment Diagnostics ")
            .title_style(theme.title_style())
            .borders(Borders::ALL)
            .border_style(theme.block_border_style(false)),
    );
    let mut table_state = ratatui::widgets::TableState::default();
    if !app.diagnostics.checks.is_empty() {
        table_state.select(Some(app.diagnostics_selected_idx));
    }
    f.render_stateful_widget(table, chunks[0], &mut table_state);

    // Section 2: Selected Check Detail Card
    let selected = app
        .diagnostics
        .checks
        .get(app.diagnostics_selected_idx)
        .or_else(|| app.diagnostics.checks.first());

    let detail_lines = if let Some(check) = selected {
        vec![
            Line::from(vec![
                Span::styled(" Diagnostic ID: ", Style::default().fg(theme.fg_muted)),
                Span::styled(
                    &check.id,
                    Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                ),
                Span::styled("   Name: ", Style::default().fg(theme.fg_muted)),
                Span::styled(
                    &check.name,
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled("   Category: ", Style::default().fg(theme.fg_muted)),
                Span::styled(&check.category, Style::default().fg(theme.accent)),
            ]),
            Line::from(vec![
                Span::styled(" Status:        ", Style::default().fg(theme.fg_muted)),
                match check.status {
                    DiagnosticStatus::Pass => Span::styled(
                        "PASSED (All contracts satisfied)",
                        Style::default()
                            .fg(theme.success)
                            .add_modifier(Modifier::BOLD),
                    ),
                    DiagnosticStatus::SimulatedPass => Span::styled(
                        "SIMULATED PASS (Simulated fixture demo - not a verified runtime audit)",
                        Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                    ),
                    DiagnosticStatus::Warn => Span::styled(
                        "WARNING (Degraded or non-optimal configuration)",
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                    DiagnosticStatus::Fail => Span::styled(
                        "FAILED (Critical requirement missing)",
                        Style::default()
                            .fg(theme.error)
                            .add_modifier(Modifier::BOLD),
                    ),
                    DiagnosticStatus::Unavailable => Span::styled(
                        "NOT APPLICABLE / UNSUPPORTED ON THIS PLATFORM",
                        Style::default()
                            .fg(theme.fg_muted)
                            .add_modifier(Modifier::BOLD),
                    ),
                    DiagnosticStatus::NotProbed => Span::styled(
                        "NOT PROBED (Capability discovery not executed in standalone prototype)",
                        Style::default()
                            .fg(theme.fg_muted)
                            .add_modifier(Modifier::BOLD),
                    ),
                },
            ]),
            Line::from(vec![
                Span::styled(" Finding:       ", Style::default().fg(theme.fg_muted)),
                Span::styled(&check.finding, Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(" Remediation:   ", Style::default().fg(theme.fg_muted)),
                Span::styled(&check.remediation, Style::default().fg(theme.info)),
            ]),
            Line::from(vec![
                Span::styled(" Interaction:   ", Style::default().fg(theme.fg_muted)),
                Span::styled(
                    "Press 'Enter' to open detailed modal view. Use '↑' / '↓' to select check.",
                    Style::default().fg(theme.fg_muted),
                ),
            ]),
        ]
    } else {
        vec![Line::from("No diagnostic check selected.")]
    };

    let detail_block = Block::default()
        .title(" Selected Check Remediation & Inspection ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(true));
    f.render_widget(Paragraph::new(detail_lines).block(detail_block), chunks[1]);
}
