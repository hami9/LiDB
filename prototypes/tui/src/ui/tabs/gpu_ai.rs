use crate::{app::App, model::gpu_ai::GpuViewMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    match app.gpu_ai.view_mode {
        GpuViewMode::HostReality => render_host_reality(f, area, app),
        GpuViewMode::SimulatedFixture => render_simulated_fixture(f, area, app),
    }
}

fn render_host_reality(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Hardware status notice
            Constraint::Length(9), // Architecture explanation
            Constraint::Min(4),    // Action prompt to toggle fixture
        ])
        .split(area);

    // Box 1: Unprobed status banner
    let status_lines = vec![
        Line::from(vec![
            Span::styled(" Subsystem: ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                "NVIDIA / CUDA / AI Accelerator Telemetry",
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                " [PLANNED] ",
                Style::default()
                    .bg(theme.selected_bg)
                    .fg(theme.fg_muted)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Detection Result: ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                "GPU support is planned after the general Linux core. Nothing is probed.",
                Style::default().fg(theme.info),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Execution Host:   ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                "Not in the current release scope. The 'g' fixture is layout data only.",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
    ];

    let status_block = Block::default()
        .title(" Host Hardware State (Rule R10 & R19 Compliant) ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(Paragraph::new(status_lines).block(status_block), chunks[0]);

    // Box 2: Engineering & Governance rules
    let rules_lines = vec![
        Line::from(vec![
            Span::styled(" Rule R01 (Truthfulness): ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            Span::styled("LiDB refuses to fabricate live GPU metrics; unprobed capabilities are explicitly labeled.", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(" Rule R03 (DGX Topology): ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            Span::styled("Spark CPU↔GPU NVLink-C2C is intra-node. DGX Spark inter-node uses ConnectX-7 Ethernet/RoCE.", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(" Rule R19 (Validation):   ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            Span::styled("Hardware-specific claims must be confirmed on real hardware; mock tests are not hardware validation.", Style::default().fg(theme.fg)),
        ]),
    ];

    let rules_block = Block::default()
        .title(" Architectural Guarantees ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(Paragraph::new(rules_lines).block(rules_block), chunks[1]);

    // Box 3: Toggle prompt
    let prompt_lines = vec![Line::from(vec![
        Span::styled(
            " Prototype Layout Testing: ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Press ", Style::default().fg(theme.fg)),
        Span::styled(
            " 'g' ",
            Style::default()
                .bg(theme.accent)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " to toggle the Simulated DGX Spark (GB10 Grace Blackwell) fixture dashboard.",
            Style::default().fg(theme.fg),
        ),
    ])];
    let prompt_block = Block::default()
        .title(" Simulator Control ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(true));
    f.render_widget(Paragraph::new(prompt_lines).block(prompt_block), chunks[2]);
}

fn render_simulated_fixture(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Simulated GB10 devices
            Constraint::Length(7), // AI Serving workloads table
            Constraint::Min(4),    // Fabric note & toggle info
        ])
        .split(area);

    // Section 1: GPU Devices Cards (2 simulated devices)
    let dev_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    for (i, dev) in app.gpu_ai.devices.iter().enumerate() {
        let mem_used_gb = dev.memory_used_bytes as f64 / 1e9;
        let mem_total_gb = dev.memory_total_bytes as f64 / 1e9;
        let mem_pct = ((mem_used_gb / mem_total_gb) * 100.0) as u16;

        let dev_block = Block::default()
            .title(format!(
                " Host: {} | Device #{}: {} ",
                dev.host_node, dev.index, dev.name
            ))
            .title_style(theme.title_style())
            .borders(Borders::ALL)
            .border_style(theme.block_border_style(false));

        let card_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Min(1),
            ])
            .split(dev_block.inner(dev_cols[i]));

        f.render_widget(dev_block, dev_cols[i]);

        let line1 = Line::from(vec![
            Span::styled("SM Util: ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                format!("{:.1}%", dev.sm_utilization_pct),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ Temp: ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                format!("{}°C", dev.temperature_c),
                Style::default().fg(theme.warning),
            ),
            Span::styled(" │ Power: ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                format!(
                    "{}W / {}W (SoC TDP)",
                    dev.power_watts, dev.power_limit_watts
                ),
                Style::default().fg(theme.fg),
            ),
        ]);
        f.render_widget(Paragraph::new(line1), card_chunks[0]);

        let line2 = Line::from(vec![
            Span::styled("Intra-Node: ", Style::default().fg(theme.fg_muted)),
            Span::styled(&dev.nvlink_status, Style::default().fg(theme.info)),
            Span::styled(" │ Inter-Node: ", Style::default().fg(theme.fg_muted)),
            Span::styled(&dev.inter_node_fabric, Style::default().fg(theme.success)),
        ]);
        f.render_widget(Paragraph::new(line2), card_chunks[1]);

        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(theme.success).bg(theme.selected_bg))
            .percent(mem_pct)
            .label(format!(
                "LPDDR5x UMA: {mem_used_gb:.1}GB / {mem_total_gb:.1}GB ({mem_pct}% unified pressure)"
            ));
        f.render_widget(gauge, card_chunks[2]);
    }

    // Section 2: AI Serving Workloads Table
    let header_cells = [
        "MODEL / SERVICE",
        "FRAMEWORK",
        "TOKENS / SEC",
        "TTFT (ms)",
        "TPOT (ms)",
        "KV CACHE %",
        "BATCH",
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
        .gpu_ai
        .workloads
        .iter()
        .map(|wl| {
            let cells = vec![
                Span::styled(
                    &wl.name,
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&wl.framework, Style::default().fg(theme.info)),
                Span::styled(
                    format!("{:.1}", wl.tokens_per_sec),
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{:.1}", wl.ttft_ms), Style::default().fg(theme.fg)),
                Span::styled(format!("{:.1}", wl.tpot_ms), Style::default().fg(theme.fg)),
                Span::styled(
                    format!("{:.1}%", wl.kv_cache_usage_pct),
                    Style::default().fg(theme.warning),
                ),
                Span::styled(
                    wl.batch_size.to_string(),
                    Style::default().fg(theme.fg_muted),
                ),
            ];
            Row::new(cells)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(24),
            Constraint::Length(16),
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Min(8),
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .title(" AI Serving KPIs (vLLM & TensorRT-LLM Telemetry) ")
            .title_style(theme.title_style())
            .borders(Borders::ALL)
            .border_style(theme.block_border_style(false)),
    );
    f.render_widget(table, chunks[1]);

    // Section 3: Fabric note & toggle info
    let note_text = vec![
        Line::from(vec![
            Span::styled(" Fabric Note: ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                &app.gpu_ai.fabric_topology_note,
                Style::default().fg(theme.info),
            ),
            Span::styled("  │  ", Style::default().fg(theme.border)),
            Span::styled(app.gpu_ai.status.badge_label(), theme.fixture_badge_style()),
        ]),
        Line::from(vec![
            Span::styled(" Toggle:      ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                "Press 'g' to return to Host Reality (unprobed host view).",
                Style::default().fg(theme.fg_muted),
            ),
        ]),
    ];
    let note_block = Block::default()
        .title(" Interconnect Architecture & Simulation Control ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(Paragraph::new(note_text).block(note_block), chunks[2]);
}
