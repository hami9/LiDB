use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    let popup_area = centered_rect(65, 60, area);
    f.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(vec![Span::styled(
            " LiDashBoard (LiDB) - TUI Quick Reference ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " Tabs Navigation:",
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("   1..7         ", Style::default().fg(theme.info)),
            Span::styled(
                "Directly switch to tab 1 through 7",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Tab / Right  ", Style::default().fg(theme.info)),
            Span::styled(
                "Cycle forward to next tab (or 'l')",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled("   BackTab / Left", Style::default().fg(theme.info)),
            Span::styled(
                "Cycle backward to previous tab (or 'h')",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " Item Selection & Inspection:",
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("   Up / Down    ", Style::default().fg(theme.info)),
            Span::styled(
                "Navigate rows in tables/lists (or 'k' / 'j')",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Enter        ", Style::default().fg(theme.info)),
            Span::styled(
                "Open detailed inspection modal for selected item",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " Process Tab Controls:",
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("   /            ", Style::default().fg(theme.info)),
            Span::styled(
                "Activate text search/filter filter mode",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled("   s            ", Style::default().fg(theme.info)),
            Span::styled(
                "Cycle sort order: CPU% → MEM% → PID → Command",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " Global Controls:",
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("   Space        ", Style::default().fg(theme.info)),
            Span::styled(
                "Pause / Resume real-time telemetry updates",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled("   g            ", Style::default().fg(theme.info)),
            Span::styled(
                "Toggle GPU between PRoot reality & Simulated fixture",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled("   t            ", Style::default().fg(theme.info)),
            Span::styled(
                "Cycle color theme (Dark, Light, HighContrast, Monochrome)",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled("   ? / F1       ", Style::default().fg(theme.info)),
            Span::styled("Open this help modal", Style::default().fg(theme.fg_muted)),
        ]),
        Line::from(vec![
            Span::styled("   Esc / q      ", Style::default().fg(theme.error)),
            Span::styled(
                "Close modal / Exit application",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
    ];

    let block = Block::default()
        .title(" Help & Keyboard Controls [Press Esc to Close] ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(true));

    f.render_widget(
        Paragraph::new(help_text)
            .block(block)
            .alignment(Alignment::Left),
        popup_area,
    );
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
