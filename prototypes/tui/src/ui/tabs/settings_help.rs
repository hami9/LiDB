use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Interactive settings & runtime controls
            Constraint::Length(8), // Keyboard shortcuts cheat sheet
            Constraint::Min(6),    // LiDB Project and architecture documentation
        ])
        .split(area);

    // Section 1: Runtime Settings & Configuration
    let settings_lines = vec![
        Line::from(vec![
            Span::styled(" Color Theme:      ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                app.theme.mode.name(),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (Press 't' to cycle: Dark → Light → High-Contrast → Monochrome)",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Telemetry Engine: ", Style::default().fg(theme.fg_muted)),
            if app.settings.paused {
                Span::styled(
                    "PAUSED",
                    Style::default()
                        .bg(theme.warning)
                        .fg(theme.bg)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    "STREAMING (500ms tick)",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                )
            },
            Span::styled(
                " (Press 'Space' to toggle pause/resume)",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled(" GPU View Mode:    ", Style::default().fg(theme.fg_muted)),
            match app.gpu_ai.view_mode {
                crate::model::gpu_ai::GpuViewMode::HostReality => Span::styled(
                    "Host Reality (PRoot unsupported)",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                crate::model::gpu_ai::GpuViewMode::SimulatedFixture => Span::styled(
                    "Simulated Superpod Fixture (GB10)",
                    Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                ),
            },
            Span::styled(
                " (Press 'g' to toggle)",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Terminal Bounds:  ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                format!("Cols: {}, Rows: {}", area.width, area.height),
                Style::default().fg(theme.fg),
            ),
            Span::styled(
                " (Resizing is dynamically handled)",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
    ];
    let settings_block = Block::default()
        .title(" Runtime Configuration & Display Settings ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(
        Paragraph::new(settings_lines).block(settings_block),
        chunks[0],
    );

    // Section 2: Complete Keyboard Shortcuts Guide
    let shortcuts_lines = vec![
        Line::from(vec![
            Span::styled(
                " Tab Switching:    ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "1..7",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (Direct jump)  │  ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                "Tab / Right / l",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (Next tab)  │  ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                "BackTab / Left / h",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (Previous tab)", Style::default().fg(theme.fg_muted)),
        ]),
        Line::from(vec![
            Span::styled(
                " Navigation:       ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Down / j",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (Select next item)  │  ",
                Style::default().fg(theme.fg_muted),
            ),
            Span::styled(
                "Up / k",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (Select previous item)  │  ",
                Style::default().fg(theme.fg_muted),
            ),
            Span::styled(
                "Enter",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (Inspect item details)",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " Process Controls: ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "/",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (Filter processes by name/user/pid)  │  ",
                Style::default().fg(theme.fg_muted),
            ),
            Span::styled(
                "s",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (Cycle sort: CPU% → MEM% → PID → Command)",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " System Controls:  ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Space",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (Pause/Resume)  │  ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                "g",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (Toggle GPU mock)  │  ",
                Style::default().fg(theme.fg_muted),
            ),
            Span::styled(
                "t",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (Cycle theme)  │  ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                "?",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (Help modal)  │  ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                "q / Ctrl+C",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (Exit)", Style::default().fg(theme.fg_muted)),
        ]),
    ];
    let shortcuts_block = Block::default()
        .title(" Keyboard Navigation Guide ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(
        Paragraph::new(shortcuts_lines).block(shortcuts_block),
        chunks[1],
    );

    // Section 3: LiDB Architecture & Documentation
    let docs_lines = vec![
        Line::from(vec![
            Span::styled(" Repository:        ", Style::default().fg(theme.fg_muted)),
            Span::styled("https://github.com/hami9/LiDB", Style::default().fg(theme.accent)),
            Span::styled("  │  Assigned Issue: #8 (Antigravity TUI)", Style::default().fg(theme.fg_muted)),
        ]),
        Line::from(vec![
            Span::styled(" Core Contract:     ", Style::default().fg(theme.fg_muted)),
            Span::styled("PR #9 (ChatGPT branch agent/chatgpt/p0-core)", Style::default().fg(theme.info)),
            Span::styled("  │  Worktree: .worktrees/antigravity/p0-tui", Style::default().fg(theme.fg_muted)),
        ]),
        Line::from(vec![
            Span::styled(" Design Policy:     ", Style::default().fg(theme.fg_muted)),
            Span::styled("Rule R01: Truthful telemetry  │  Rule R05: Non-root baseline  │  Rule R10: Graceful fallbacks", Style::default().fg(theme.fg)),
        ]),
    ];
    let docs_block = Block::default()
        .title(" Architecture, Coordination & Documentation ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(Paragraph::new(docs_lines).block(docs_block), chunks[2]);
}
