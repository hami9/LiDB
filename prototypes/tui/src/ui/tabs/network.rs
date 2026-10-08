use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Sparkline, Table},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Interface table
            Constraint::Length(7), // Traffic sparklines
            Constraint::Min(4),    // Socket states & netlink status
        ])
        .split(area);

    // Section 1: Interfaces Table
    let header_cells = [
        "INTERFACE",
        "STATE",
        "IP ADDRESS",
        "MAC ADDRESS",
        "MTU",
        "RX RATE",
        "TX RATE",
        "DROPS/ERR",
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
        .network
        .interfaces
        .iter()
        .enumerate()
        .map(|(i, iface)| {
            let is_selected = i == app.network_selected_idx;
            let state_span = if iface.is_up {
                Span::styled(
                    "UP",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("DOWN", Style::default().fg(theme.error))
            };

            let rx_fmt = format!("{:.1} KB/s", iface.rx_bytes_sec as f64 / 1024.0);
            let tx_fmt = format!("{:.1} KB/s", iface.tx_bytes_sec as f64 / 1024.0);
            let drop_fmt = format!("{}/{}", iface.rx_dropped, iface.rx_errors);

            let row_cells = vec![
                Span::styled(
                    &iface.name,
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
                state_span,
                Span::styled(&iface.ip, Style::default().fg(theme.info)),
                Span::styled(&iface.mac, Style::default().fg(theme.fg_muted)),
                Span::styled(iface.mtu.to_string(), Style::default().fg(theme.fg_muted)),
                Span::styled(rx_fmt, Style::default().fg(theme.fg)),
                Span::styled(tx_fmt, Style::default().fg(theme.fg)),
                Span::styled(drop_fmt, Style::default().fg(theme.fg_muted)),
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
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(18),
            Constraint::Length(20),
            Constraint::Length(8),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Min(10),
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .title(" Linux Network Interfaces (/proc/net/dev & Netlink) ")
            .title_style(theme.title_style())
            .borders(Borders::ALL)
            .border_style(theme.block_border_style(false)),
    );
    f.render_widget(table, chunks[0]);

    // Section 2: Traffic Sparklines
    let spark_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let rx_spark = Sparkline::default()
        .block(
            Block::default()
                .title(" Inbound (RX) Throughput Trend ")
                .title_style(theme.title_style())
                .borders(Borders::ALL)
                .border_style(theme.block_border_style(false)),
        )
        .data(&app.network.rx_history)
        .style(Style::default().fg(theme.info));
    f.render_widget(rx_spark, spark_cols[0]);

    let tx_spark = Sparkline::default()
        .block(
            Block::default()
                .title(" Outbound (TX) Throughput Trend ")
                .title_style(theme.title_style())
                .borders(Borders::ALL)
                .border_style(theme.block_border_style(false)),
        )
        .data(&app.network.tx_history)
        .style(Style::default().fg(theme.warning));
    f.render_widget(tx_spark, spark_cols[1]);

    // Section 3: Sockets & Protocol Summary
    let socket_text = vec![
        Line::from(vec![
            Span::styled(" Linux Sockets Overview: ", Style::default().fg(theme.fg_muted)),
            Span::styled(format!("TCP Established: {}", app.network.sockets.tcp_established), Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            Span::styled("  │  ", Style::default().fg(theme.border)),
            Span::styled(format!("TCP Listen: {}", app.network.sockets.tcp_listen), Style::default().fg(theme.info).add_modifier(Modifier::BOLD)),
            Span::styled("  │  ", Style::default().fg(theme.border)),
            Span::styled(format!("TCP TimeWait: {}", app.network.sockets.tcp_time_wait), Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled("  │  ", Style::default().fg(theme.border)),
            Span::styled(format!("UDP Sockets: {}", app.network.sockets.udp_total), Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
            Span::styled("  │  ", Style::default().fg(theme.border)),
            Span::styled(app.network.status.badge_label(), theme.fixture_badge_style()),
        ]),
        Line::from(vec![
            Span::styled(" Rule R02 Compliance:   ", Style::default().fg(theme.fg_muted)),
            Span::styled("Distinguish ingress/egress/forwarding hooks; no mutation of host nftables or routes.", Style::default().fg(theme.fg_muted)),
        ]),
    ];
    let socket_block = Block::default()
        .title(" Protocol Sockets & Routing State ")
        .title_style(theme.title_style())
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(Paragraph::new(socket_text).block(socket_block), chunks[2]);
}
