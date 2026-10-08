use crate::{
    model::{diagnostics::DiagnosticCheck, processes::ProcessItem},
    theme::Theme,
    ui::modals::help_modal::centered_rect,
};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_process_detail(f: &mut Frame, area: Rect, proc: &ProcessItem, theme: &Theme) {
    let popup_area = centered_rect(65, 55, area);
    f.render_widget(Clear, popup_area);

    let detail_lines = vec![
        Line::from(vec![Span::styled(
            format!(" Process Inspection: PID {} ({}) ", proc.pid, proc.command),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Process ID (PID): ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                proc.pid.to_string(),
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled("   User: ", Style::default().fg(theme.fg_muted)),
            Span::styled(&proc.user, Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(" State:            ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                &proc.state,
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Threads: ", Style::default().fg(theme.fg_muted)),
            Span::styled(proc.threads.to_string(), Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(" CPU Usage:        ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                format!("{:.1}%", proc.cpu_pct),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Memory (RSS): ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                format!(
                    "{:.1} MB ({:.1}%)",
                    proc.rss_bytes as f64 / 1e6,
                    proc.mem_pct
                ),
                Style::default().fg(theme.info),
            ),
        ]),
        Line::from(vec![
            Span::styled(" I/O Activity:     ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                format!(
                    "Read: {} KB/s  │  Write: {} KB/s",
                    proc.io_read_kb_s, proc.io_write_kb_s
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Command Line:     ", Style::default().fg(theme.fg_muted)),
            Span::styled(&proc.command, Style::default().fg(theme.fg)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " Isolation Context: /proc/[pid]/ns/ and cgroup v2 controller attribution.",
            Style::default().fg(theme.fg_muted),
        )]),
    ];

    let block = Block::default()
        .title(" Process Details [Press Esc / Enter to Close] ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(true));

    f.render_widget(Paragraph::new(detail_lines).block(block), popup_area);
}

pub fn render_diagnostic_detail(f: &mut Frame, area: Rect, check: &DiagnosticCheck, theme: &Theme) {
    let popup_area = centered_rect(65, 55, area);
    f.render_widget(Clear, popup_area);

    let detail_lines = vec![
        Line::from(vec![Span::styled(
            format!(" Diagnostic Check: {} [{}] ", check.id, check.name),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Category:    ", Style::default().fg(theme.fg_muted)),
            Span::styled(&check.category, Style::default().fg(theme.info)),
        ]),
        Line::from(vec![
            Span::styled(" Evaluation:  ", Style::default().fg(theme.fg_muted)),
            Span::styled(check.status.badge(), theme.fixture_badge_style()),
        ]),
        Line::from(vec![
            Span::styled(" Finding:     ", Style::default().fg(theme.fg_muted)),
            Span::styled(&check.finding, Style::default().fg(theme.fg)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Remediation: ", Style::default().fg(theme.fg_muted)),
            Span::styled(&check.remediation, Style::default().fg(theme.info)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " Compliance:  Per LiDB Rules R05, R06, R10 (Non-root, safe read-only baseline).",
            Style::default().fg(theme.fg_muted),
        )]),
    ];

    let block = Block::default()
        .title(" Diagnostic Assessment [Press Esc / Enter to Close] ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(true));

    f.render_widget(Paragraph::new(detail_lines).block(block), popup_area);
}
