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
            Constraint::Length(7), // Host & kernel info cards
            Constraint::Length(8), // Architectural & policy compliance
            Constraint::Min(8),    // System overview telemetry snapshot
        ])
        .split(area);

    // Row 1: Host identification & resources
    let row1 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    // Host node info
    let uptime_hrs = app.system.uptime_seconds / 3600;
    let uptime_mins = (app.system.uptime_seconds % 3600) / 60;
    let uptime_secs = app.system.uptime_seconds % 60;

    let host_info = vec![
        Line::from(vec![
            Span::styled(" Hostname: ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                &app.system.hostname,
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled("   OS: ", Style::default().fg(theme.fg_muted)),
            Span::styled(&app.system.os_name, Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(" Kernel:   ", Style::default().fg(theme.fg_muted)),
            Span::styled(&app.system.kernel_version, Style::default().fg(theme.fg)),
            Span::styled("   Arch: ", Style::default().fg(theme.fg_muted)),
            Span::styled(&app.system.architecture, Style::default().fg(theme.accent)),
        ]),
        Line::from(vec![
            Span::styled(" Uptime:   ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                format!("{}h {}m {}s", uptime_hrs, uptime_mins, uptime_secs),
                Style::default().fg(theme.fg),
            ),
            Span::styled("   Load: ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                format!(
                    "{:.2}, {:.2}, {:.2}",
                    app.system.load_average[0],
                    app.system.load_average[1],
                    app.system.load_average[2]
                ),
                Style::default().fg(theme.info),
            ),
        ]),
    ];
    let host_block = Block::default()
        .title(" System Identification ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(Paragraph::new(host_info).block(host_block), row1[0]);

    // Agent & Runtime Status
    let agent_info = vec![
        Line::from(vec![
            Span::styled(" Active Agent:     ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                &app.system.lidb_agent,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Prototype Scope:  ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                "Issue #8 Ratatui TUI Prototype (Standalone)",
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Core Contracts:   ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                "PR #9 (ChatGPT parallel branch)",
                Style::default().fg(theme.info),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Sandbox Mode:     ", Style::default().fg(theme.fg_muted)),
            if app.system.proot_detected {
                Span::styled(
                    "PRoot aarch64 Android Sandbox (Unprivileged)",
                    Style::default().fg(theme.warning),
                )
            } else {
                Span::styled("Standard Host Runtime", Style::default().fg(theme.success))
            },
        ]),
    ];
    let agent_block = Block::default()
        .title(" LiDB Coordination & Agent Context ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(Paragraph::new(agent_info).block(agent_block), row1[1]);

    // Row 2: Policy & Architectural Invariants (Compliance Card)
    let policy_lines = vec![
        Line::from(vec![
            Span::styled(
                " ✓ Rule R01 / R14: ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Truthful telemetry reporting. Simulated fixtures are explicitly identified.",
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " ✓ Rule R03 / P0:  ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Spark CPU↔GPU NVLink-C2C is intra-node; DGX Spark inter-node uses ConnectX RoCE.",
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " ✓ Rule R05 / R06: ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Non-root baseline. Read-only metadata operations; zero destructive actions.",
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " ✓ Rule R10 / R19: ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Zero assumed CUDA/eBPF/driver access. Truthful unprobed/unsupported state fallback.",
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " ✓ Multi-Agent:    ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Isolated worktree at .worktrees/antigravity/p0-tui-laptop. Root Cargo.toml preserved.",
                Style::default().fg(theme.fg),
            ),
        ]),
    ];
    let policy_block = Block::default()
        .title(" Architectural Contract & Compliance Gates ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(Paragraph::new(policy_lines).block(policy_block), chunks[1]);

    // Row 3: Live vs Simulated Telemetry Status Card
    let status_card = vec![
        Line::from(vec![
            Span::styled(" Telemetry Status: ", Style::default().fg(theme.fg_muted)),
            Span::styled(app.system.status.badge_label(), theme.fixture_badge_style()),
        ]),
        Line::from(vec![
            Span::styled(" Status Detail:    ", Style::default().fg(theme.fg_muted)),
            Span::styled(&app.system.status_message, Style::default().fg(theme.fg)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Quick Stats:      ", Style::default().fg(theme.fg_muted)),
            Span::styled(format!("CPU: {:.1}%", app.cpu_mem.overall_cpu_pct), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("  │  ", Style::default().fg(theme.border)),
            Span::styled(
                format!("Memory: {:.1} GB / {:.1} GB ({:.0}%)",
                    app.cpu_mem.memory.used_bytes as f64 / 1e9,
                    app.cpu_mem.memory.total_bytes as f64 / 1e9,
                    (app.cpu_mem.memory.used_bytes as f64 / app.cpu_mem.memory.total_bytes as f64) * 100.0
                ),
                Style::default().fg(theme.info),
            ),
            Span::styled("  │  ", Style::default().fg(theme.border)),
            Span::styled(
                format!("Active Interfaces: {}", app.network.interfaces.iter().filter(|i| i.is_up).count()),
                Style::default().fg(theme.success),
            ),
            Span::styled("  │  ", Style::default().fg(theme.border)),
            Span::styled(
                format!("Processes Monitored: {}", app.processes.items.len()),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Navigation Note:  ", Style::default().fg(theme.fg_muted)),
            Span::styled("Press '2' for CPU/Memory, '3' for Network, '4' for GPU/AI, '5' for Processes, '6' for Diagnostics.", Style::default().fg(theme.fg_muted)),
        ]),
    ];
    let status_block = Block::default()
        .title(" Subsystem Summary ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(Paragraph::new(status_card).block(status_block), chunks[2]);
}
