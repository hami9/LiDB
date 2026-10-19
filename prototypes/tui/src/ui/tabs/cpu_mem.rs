use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Overall CPU gauge & Sparkline
            Constraint::Length(7), // Per-core bars
            Constraint::Length(6), // Memory & Swap gauges
            Constraint::Min(4),    // PSI metrics
        ])
        .split(area);

    // Section 1: Overall CPU gauge & sparkline
    let cpu_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let cpu_pct = app.cpu_mem.overall_cpu_pct.clamp(0.0, 100.0) as u16;
    let cpu_gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Overall CPU Utilization ")
                .title_style(theme.title_style())
                .borders(Borders::ALL)
                .border_style(theme.block_border_style(false)),
        )
        .gauge_style(
            Style::default()
                .fg(if cpu_pct > 80 {
                    theme.error
                } else if cpu_pct > 50 {
                    theme.warning
                } else {
                    theme.accent
                })
                .bg(theme.selected_bg),
        )
        .percent(cpu_pct)
        .label(format!("{:.1}% [8 Cores]", app.cpu_mem.overall_cpu_pct));
    f.render_widget(cpu_gauge, cpu_chunks[0]);

    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(" CPU Activity Trend (Recent Ticks) ")
                .title_style(theme.title_style())
                .borders(Borders::ALL)
                .border_style(theme.block_border_style(false)),
        )
        .data(&app.cpu_mem.cpu_history)
        .style(Style::default().fg(theme.accent));
    f.render_widget(sparkline, cpu_chunks[1]);

    // Section 2: Per-Core Distribution (8 cores)
    let core_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[1]);

    for (i, core) in app.cpu_mem.cores.iter().take(4).enumerate() {
        let p_core = core.usage_pct.clamp(0.0, 100.0) as u16;
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(format!(" Core #{} ", core.core_id))
                    .borders(Borders::ALL)
                    .border_style(theme.block_border_style(false)),
            )
            .gauge_style(Style::default().fg(theme.info).bg(theme.selected_bg))
            .percent(p_core)
            .label(format!(
                "{:.1}% ({}MHz)",
                core.usage_pct, core.frequency_mhz
            ));
        f.render_widget(gauge, core_cols[i]);
    }

    // Section 3: Memory & Swap
    let mem_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    let mem_total_gb = app.cpu_mem.memory.total_bytes as f64 / 1e9;
    let mem_used_gb = app.cpu_mem.memory.used_bytes as f64 / 1e9;
    let mem_pct = ((mem_used_gb / mem_total_gb) * 100.0).clamp(0.0, 100.0) as u16;

    let mem_gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Physical Memory (RAM) ")
                .title_style(theme.title_style())
                .borders(Borders::ALL)
                .border_style(theme.block_border_style(false)),
        )
        .gauge_style(Style::default().fg(theme.success).bg(theme.selected_bg))
        .percent(mem_pct)
        .label(format!(
            "{:.1} GB / {:.1} GB ({}%)",
            mem_used_gb, mem_total_gb, mem_pct
        ));
    f.render_widget(mem_gauge, mem_cols[0]);

    let swap_total_gb = app.cpu_mem.memory.swap_total_bytes as f64 / 1e9;
    let swap_used_gb = app.cpu_mem.memory.swap_used_bytes as f64 / 1e9;
    let swap_pct = if swap_total_gb > 0.0 {
        ((swap_used_gb / swap_total_gb) * 100.0).clamp(0.0, 100.0) as u16
    } else {
        0
    };

    let swap_gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Swap Space ")
                .title_style(theme.title_style())
                .borders(Borders::ALL)
                .border_style(theme.block_border_style(false)),
        )
        .gauge_style(Style::default().fg(theme.warning).bg(theme.selected_bg))
        .percent(swap_pct)
        .label(format!(
            "{:.1} GB / {:.1} GB ({}%)",
            swap_used_gb, swap_total_gb, swap_pct
        ));
    f.render_widget(swap_gauge, mem_cols[1]);

    // Section 4: Pressure Stall Information (PSI)
    let psi_text = vec![Line::from(vec![
        Span::styled(
            " Linux PSI Stall Metrics (avg10): ",
            Style::default().fg(theme.fg_muted),
        ),
        Span::styled(
            format!("CPU Some: {:.2}%", app.cpu_mem.psi.cpu_some_avg10),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(theme.border)),
        Span::styled(
            format!("Memory Some: {:.2}%", app.cpu_mem.psi.mem_some_avg10),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(theme.border)),
        Span::styled(
            format!("Memory Full: {:.2}%", app.cpu_mem.psi.mem_full_avg10),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(theme.border)),
        Span::styled(
            format!("I/O Some: {:.2}%", app.cpu_mem.psi.io_some_avg10),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(theme.border)),
        Span::styled(
            app.cpu_mem.status.badge_label(),
            theme.fixture_badge_style(),
        ),
    ])];
    let psi_block = Block::default()
        .title(" Pressure Stall Information (PSI - Simulated Fixture) ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(Paragraph::new(psi_text).block(psi_block), chunks[3]);
}
