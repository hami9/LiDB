use crate::{
    config::Config,
    output::{mode_name, value_text},
    worker::{Update, Worker},
};
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use lidb_core::{
    EbpfReport, MetricObservation, MetricState, MetricValue, NetworkReport, Snapshot,
    SourceAvailability,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table, Tabs, Wrap},
    Frame, Terminal,
};
use std::{
    collections::VecDeque,
    fmt::Write as _,
    io, panic,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

/// Active dashboard view tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// Full list of all collected host metrics.
    #[default]
    Metrics = 0,
    /// Dedicated CPU utilization and memory overview dashboard.
    CpuMemory = 1,
    /// Dedicated Network overview dashboard (interfaces, routes, sockets, namespace).
    Network = 2,
    /// Dedicated eBPF lifecycle events dashboard.
    Ebpf = 3,
}

impl Tab {
    /// All available tabs in presentation order.
    pub const ALL: [Tab; 4] = [Tab::Metrics, Tab::CpuMemory, Tab::Network, Tab::Ebpf];

    /// Cycles forward to the next tab.
    pub fn next(&self) -> Self {
        match self {
            Self::Metrics => Self::CpuMemory,
            Self::CpuMemory => Self::Network,
            Self::Network => Self::Ebpf,
            Self::Ebpf => Self::Metrics,
        }
    }

    /// Cycles backward to the previous tab.
    pub fn prev(&self) -> Self {
        match self {
            Self::Metrics => Self::Ebpf,
            Self::CpuMemory => Self::Metrics,
            Self::Network => Self::CpuMemory,
            Self::Ebpf => Self::Network,
        }
    }
}

/// Maximum number of historical updates retained in memory for TUI time-travel inspection.
pub const MAX_VIEW_HISTORY: usize = 120;

/// In-memory interactive state for the terminal UI.
#[derive(Default)]
pub struct View {
    pub update: Option<Update>,
    pub history: VecDeque<Update>,
    pub history_cursor: Option<usize>,
    pub paused: bool,
    pub help: bool,
    pub detail: bool,
    pub scroll: usize,
    pub text_scroll: u16,
    pub(crate) text_page_size: u16,
    pub fixture: bool,
    pub tab: Tab,
}

impl View {
    /// Pushes a new update into bounded history and updates current view if live.
    pub fn push_update(&mut self, update: Update) {
        if self.history.len() >= MAX_VIEW_HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(update.clone());
        if !self.paused {
            self.update = Some(update);
            self.history_cursor = None;
        }
    }

    /// Steps backward one historical update, pausing live stream if running.
    pub fn step_history_back(&mut self) {
        if self.history.is_empty() {
            return;
        }
        self.paused = true;
        let max_offset = self.history.len().saturating_sub(1);
        let next_offset = match self.history_cursor {
            None => 1.min(max_offset),
            Some(curr) => curr.saturating_add(1).min(max_offset),
        };
        self.history_cursor = Some(next_offset);
        let idx = self
            .history
            .len()
            .saturating_sub(1)
            .saturating_sub(next_offset);
        if let Some(sample) = self.history.get(idx) {
            self.update = Some(sample.clone());
        }
    }

    /// Steps forward one historical update towards latest.
    pub fn step_history_forward(&mut self) {
        if self.history.is_empty() {
            return;
        }
        if let Some(curr) = self.history_cursor {
            let next_offset = curr.saturating_sub(1);
            self.history_cursor = Some(next_offset);
            let idx = self
                .history
                .len()
                .saturating_sub(1)
                .saturating_sub(next_offset);
            if let Some(sample) = self.history.get(idx) {
                self.update = Some(sample.clone());
            }
        }
    }

    /// Adjusts table scroll position by a signed delta bounded to available items on the active tab.
    fn scroll_by(&mut self, count: isize) {
        let max = self.update.as_ref().map_or(0, |update| match self.tab {
            Tab::Metrics | Tab::CpuMemory => update.snapshot.metrics().len().saturating_sub(1),
            Tab::Network => update
                .network
                .as_ref()
                .map_or(0, |n| n.sockets().len().saturating_sub(1)),
            Tab::Ebpf => update
                .ebpf
                .as_ref()
                .map_or(0, |e| e.events().len().saturating_sub(1)),
        });
        self.scroll = self.scroll.saturating_add_signed(count).min(max);
        self.text_scroll = 0;
    }

    /// Scrolls text views (help and details) by one page up or down.
    fn scroll_text_page(&mut self, down: bool) {
        let page = self.text_page_size.max(1);
        self.text_scroll = if down {
            self.text_scroll.saturating_add(page)
        } else {
            self.text_scroll.saturating_sub(page)
        };
    }
}

/// Draws scrollable wrapped text inside a bordered block.
fn draw_text(frame: &mut Frame<'_>, text: &str, title: &str, scroll: &mut u16, area: Rect) {
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    let lines = paragraph.line_count(area.width.saturating_sub(2));
    let max_scroll = lines.saturating_sub(area.height.saturating_sub(2) as usize);
    *scroll = usize::from(*scroll).min(max_scroll) as u16;
    frame.render_widget(
        paragraph
            .scroll((*scroll, 0))
            .block(Block::default().title(title).borders(Borders::ALL)),
        area,
    );
}

/// Renders the complete terminal dashboard into the given frame.
pub fn draw(frame: &mut Frame<'_>, view: &mut View) {
    let area = frame.area();
    let mode = if view.fixture {
        "fixture"
    } else {
        view.update
            .as_ref()
            .map_or("live", |update| mode_name(update.snapshot.mode()))
    };
    if area.width < 35 || area.height < 10 {
        let message = format!("LiDB {mode}\nTerminal too small\nResize to 35x10; q quits");
        frame.render_widget(Paragraph::new(message), area);
        return;
    }
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    view.text_page_size = layout[1].height.saturating_sub(2).max(1);
    let (state, freshness) = if let Some(offset) = view.history_cursor {
        let total = view.history.len();
        let state_str = format!("PAUSED [-{offset}/{total}]");
        let freshness_str = view.update.as_ref().map_or_else(
            || "history empty".into(),
            |update| {
                format!(
                    "age {:.1}s | read {:.1}ms | dropped {} | skipped {}",
                    update.collected_at.elapsed().as_secs_f64(),
                    update.snapshot.collection_duration_ns() as f64 / 1_000_000.0,
                    update.snapshot.dropped_metrics(),
                    update.skipped_updates
                )
            },
        );
        (state_str, freshness_str)
    } else {
        let state_str = if view.paused { "PAUSED" } else { "running" }.to_string();
        let freshness_str = view.update.as_ref().map_or_else(
            || "awaiting first sample".into(),
            |update| {
                format!(
                    "age {:.1}s | read {:.1}ms | dropped {} | skipped {}",
                    update.collected_at.elapsed().as_secs_f64(),
                    update.snapshot.collection_duration_ns() as f64 / 1_000_000.0,
                    update.snapshot.dropped_metrics(),
                    update.skipped_updates
                )
            },
        );
        (state_str, freshness_str)
    };
    let header_block = Block::default()
        .title(format!(" LiDB {mode} | read-only host metrics "))
        .borders(Borders::ALL);
    let inner_header = header_block.inner(layout[0]);
    frame.render_widget(header_block, layout[0]);

    if inner_header.width >= 78 {
        let header_layout =
            Layout::horizontal([Constraint::Min(24), Constraint::Length(51)]).split(inner_header);
        frame.render_widget(
            Paragraph::new(format!("{state} | {freshness}")),
            header_layout[0],
        );
        let tabs = Tabs::new(vec!["1: Metrics", "2: CPU/Mem", "3: Network", "4: eBPF"])
            .select(view.tab as usize)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED));
        frame.render_widget(tabs, header_layout[1]);
    } else {
        frame.render_widget(
            Paragraph::new(format!("{state} | {freshness}")),
            inner_header,
        );
    }
    let shortcuts = if view.help || view.detail {
        "q quit | Up/Dn / PgUp/PgDn scroll text | n/p item"
    } else {
        "q quit | Tab / 1-4 tab | Space pause | [ / ] history | ? help | Enter detail | Up/Down scroll"
    };
    frame.render_widget(Paragraph::new(shortcuts), layout[2]);
    if view.help {
        const HELP_TEXT: &str = "q, Esc, Ctrl-C: quit\nTab, 1, 2, 3, 4, Left/Right: switch between 1:Metrics, 2:CPU/Mem, 3:Network, and 4:eBPF tabs\nSpace: pause displayed snapshot (sampling continues)\n[, ]: step backward/forward in history (auto-pauses; Space resumes live)\n?, h: toggle help\nEnter: toggle full details for the active view (Metrics, CPU/Mem, Network, eBPF)\nUp/Down, j/k: scroll metrics (scroll text in help/details)\nn, p: next/previous item (works in table and detail views)\nPageUp/PageDown: scroll ten metrics or one page of text\nHome/End: first/last metric or start/end of text\n\nAge is time since collection; paused values grow older.\nUnavailable states retain their reason; no missing value becomes zero.\nSkipped counts snapshots replaced before the UI consumed them.\nDropped counts metrics omitted by the collector.\nRates require two valid counter samples.\nAll timestamps use this collector's monotonic clock.\nFixture readings are test input, not live host measurements.";
        let help_body = if area.width >= 96 {
            format!("{}\n\n{}", crate::config::ASCII_LOGO, HELP_TEXT)
        } else {
            HELP_TEXT.to_string()
        };
        draw_text(
            frame,
            &help_body,
            " Help ",
            &mut view.text_scroll,
            layout[1],
        );
        return;
    }
    let Some(update) = &view.update else {
        frame.render_widget(
            Paragraph::new("Reading local procfs; rates await the next sample."),
            layout[1],
        );
        return;
    };
    let metrics = update.snapshot.metrics();
    let start = view.scroll.min(metrics.len().saturating_sub(1));
    if view.detail {
        let (text, title) = match view.tab {
            Tab::Metrics => {
                let text = metrics.get(start).map_or_else(
                    || "No metrics collected".into(),
                    |metric| {
                        format!(
                            "{}\n{}\nunit: {}\nsource: {}\nobservation: {}ns (collector monotonic)",
                            metric.name(),
                            value_text(metric),
                            metric.unit().as_str(),
                            metric.source(),
                            metric.monotonic_ns()
                        )
                    },
                );
                (text, " Metric detail | Enter returns ")
            }
            Tab::Network => (
                format_network_detail(update.network.as_ref(), &update.snapshot, view.scroll),
                " Socket detail | Enter returns ",
            ),
            Tab::Ebpf => {
                let events = update.ebpf.as_ref().map(|e| e.events()).unwrap_or(&[]);
                let ebpf_start = if events.is_empty() {
                    0
                } else {
                    view.scroll.min(events.len().saturating_sub(1))
                };
                let text = events.get(ebpf_start).map_or_else(
                    || "No eBPF events observed".into(),
                    |event| {
                        let transition = match (event.old_state(), event.new_state()) {
                            (Some(old), Some(new)) => {
                                format!("{} -> {}", old.as_str(), new.as_str())
                            }
                            (None, Some(new)) => format!("-> {}", new.as_str()),
                            (Some(old), None) => format!("{} ->", old.as_str()),
                            (None, None) => "-".to_string(),
                        };
                        let rtt = event.round_trip_us().map_or("-".to_string(), |us| {
                            format!("{:.1}ms ({us} µs)", us as f64 / 1000.0)
                        });
                        format!(
                            "Event: {}\nProcess: {} [PID {}]\nAddress Family: {}\nSource Endpoint: {}:{}\nDestination Endpoint: {}:{}\nState Transition: {}\nRound-Trip Latency: {}\nTracepoint: sock:inet_sock_set_state\nTimestamp: {}ns (collector monotonic)",
                            event.event_type().as_str(),
                            event.comm(),
                            event.pid(),
                            event.family().as_str(),
                            event.local_address(),
                            event.local_port(),
                            event.remote_address(),
                            event.remote_port(),
                            transition,
                            rtt,
                            event.timestamp_ns()
                        )
                    },
                );
                (text, " eBPF Event detail | Enter returns ")
            }
            Tab::CpuMemory => (
                format_cpu_memory_detail(&update.snapshot),
                " CPU & Memory detail | Enter returns ",
            ),
        };
        draw_text(frame, &text, title, &mut view.text_scroll, layout[1]);
        return;
    }
    if view.tab == Tab::CpuMemory {
        draw_cpu_mem(frame, &update.snapshot, layout[1]);
        return;
    }
    if view.tab == Tab::Network {
        draw_network(
            frame,
            update.network.as_ref(),
            Some(&update.snapshot),
            view.scroll,
            layout[1],
        );
        return;
    }
    if view.tab == Tab::Ebpf {
        draw_ebpf(frame, update.ebpf.as_ref(), view.scroll, layout[1]);
        return;
    }
    if area.width >= 90 {
        let rows = metrics.iter().skip(start).map(|metric| {
            Row::new(vec![
                metric.name().to_owned(),
                value_text(metric),
                metric.unit().as_str().to_owned(),
                metric.source().to_owned(),
            ])
        });
        let table = Table::new(
            rows,
            [
                Constraint::Percentage(30),
                Constraint::Percentage(30),
                Constraint::Percentage(18),
                Constraint::Percentage(22),
            ],
        )
        .header(
            Row::new(["Metric", "Value / state", "Unit", "Source"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    " Metrics {}..{} of {} | {}ns ",
                    start.saturating_add(1),
                    (start + layout[1].height.saturating_sub(3) as usize).min(metrics.len()),
                    metrics.len(),
                    update.snapshot.monotonic_ns()
                ))
                .borders(Borders::ALL),
        );
        frame.render_widget(table, layout[1]);
    } else {
        let mut lines = Vec::new();
        for metric in metrics.iter().skip(start).take(layout[1].height as usize) {
            lines.push(
                Line::from(metric.name().to_owned())
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            );
            lines.push(Line::from(format!(
                "{} {}",
                value_text(metric),
                metric.unit().as_str()
            )));
            lines.push(Line::from(format!("source: {}", metric.source())));
        }
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }).block(
                Block::default()
                    .title(format!(" Metrics | {}ns ", update.snapshot.monotonic_ns()))
                    .borders(Borders::ALL),
            ),
            layout[1],
        );
    }
}

/// Finds a metric observation within a snapshot by its exact unique name.
fn find_metric<'a>(
    snapshot: &'a Snapshot,
    name: &str,
) -> Option<&'a MetricObservation<MetricValue>> {
    snapshot.metrics().iter().find(|m| m.name() == name)
}

/// Formats a metric observation for display, or returns "not probed" if absent.
fn metric_display(m: Option<&MetricObservation<MetricValue>>) -> String {
    m.map_or_else(|| "not probed".to_owned(), value_text)
}

/// Formats a byte rate (bytes per second) into a human-readable throughput string.
fn format_rate_value(value: f64) -> String {
    if !value.is_finite() || value < 0.0 {
        return "-".to_owned();
    }
    if value < 1024.0 {
        format!("{value:.0} B/s")
    } else if value < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", value / 1024.0)
    } else if value < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB/s", value / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB/s", value / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Extracts and formats a metric value as a transfer rate if available.
fn format_metric_rate(metric: &MetricObservation<MetricValue>) -> String {
    match metric.state().available_value() {
        Some(MetricValue::Number(num)) => format_rate_value(*num),
        Some(MetricValue::Integer(int)) => format_rate_value(*int as f64),
        None => "-".to_owned(),
    }
}

/// Formats uptime in seconds into a human-friendly duration string (e.g. "2d 4h 12m 30s (187950s)").
fn format_uptime_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "-".to_owned();
    }
    let total_secs = seconds as u64;
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if days > 0 {
        format!("{days}d {hours}h {minutes}m {secs}s ({seconds:.0}s)")
    } else if hours > 0 {
        format!("{hours}h {minutes}m {secs}s ({seconds:.0}s)")
    } else if minutes > 0 {
        format!("{minutes}m {secs}s ({seconds:.0}s)")
    } else {
        format!("{seconds:.1}s")
    }
}

/// Formats a comprehensive deep-dive system resource summary for the CPU and Memory tab.
fn format_cpu_memory_detail(snapshot: &Snapshot) -> String {
    let cpu_busy = metric_display(find_metric(snapshot, "cpu.busy.percent"));
    let user = metric_display(find_metric(snapshot, "cpu.user.ticks"));
    let system = metric_display(find_metric(snapshot, "cpu.system.ticks"));
    let idle = metric_display(find_metric(snapshot, "cpu.idle.ticks"));
    let iowait = metric_display(find_metric(snapshot, "cpu.iowait.ticks"));
    let steal = metric_display(find_metric(snapshot, "cpu.steal.ticks"));
    let nice = metric_display(find_metric(snapshot, "cpu.nice.ticks"));
    let irq = metric_display(find_metric(snapshot, "cpu.irq.ticks"));
    let softirq = metric_display(find_metric(snapshot, "cpu.softirq.ticks"));

    let mem_total = find_metric(snapshot, "memory.total.bytes");
    let mem_used = find_metric(snapshot, "memory.used.bytes");
    let mem_avail = find_metric(snapshot, "memory.available.bytes");

    let mem_summary = match (
        mem_total.map(MetricObservation::state),
        mem_used.map(MetricObservation::state),
    ) {
        (Some(MetricState::Available(total_val)), Some(MetricState::Available(used_val))) => {
            let total = total_val.as_f64();
            let used = used_val.as_f64();
            let total_gib = total / 1_073_741_824.0;
            let used_gib = used / 1_073_741_824.0;
            let pct = if total > 0.0 {
                ((used / total) * 100.0).clamp(0.0, 100.0) as u16
            } else {
                0
            };
            let avail_str = match mem_avail.map(MetricObservation::state) {
                Some(MetricState::Available(avail_val)) => {
                    format!("{:.2} GiB", avail_val.as_f64() / 1_073_741_824.0)
                }
                _ => "n/a".to_owned(),
            };
            format!(
                "Total: {total_gib:.2} GiB ({total:.0} B)\nUsed: {used_gib:.2} GiB ({used:.0} B) [{pct}%]\nAvailable: {avail_str}"
            )
        }
        _ => {
            let total_str = mem_total.map_or_else(|| "not probed".to_owned(), value_text);
            let used_str = mem_used.map_or_else(|| "not probed".to_owned(), value_text);
            format!("Total: {total_str}\nUsed: {used_str}")
        }
    };

    let swap_total = find_metric(snapshot, "swap.total.bytes");
    let swap_used = find_metric(snapshot, "swap.used.bytes");

    let swap_summary = match (
        swap_total.map(MetricObservation::state),
        swap_used.map(MetricObservation::state),
    ) {
        (Some(MetricState::Available(total_val)), Some(MetricState::Available(used_val))) => {
            let total = total_val.as_f64();
            let used = used_val.as_f64();
            if total > 0.0 {
                let total_gib = total / 1_073_741_824.0;
                let used_gib = used / 1_073_741_824.0;
                let pct = ((used / total) * 100.0).clamp(0.0, 100.0) as u16;
                format!(
                    "Total: {total_gib:.2} GiB ({total:.0} B)\nUsed: {used_gib:.2} GiB ({used:.0} B) [{pct}%]"
                )
            } else {
                "No swap configured (0 B)".to_owned()
            }
        }
        _ => {
            let total_str = swap_total.map_or_else(|| "not probed".to_owned(), value_text);
            let used_str = swap_used.map_or_else(|| "not probed".to_owned(), value_text);
            format!("Total: {total_str}\nUsed: {used_str}")
        }
    };

    let psi_cpu = metric_display(find_metric(snapshot, "pressure.cpu.some.avg10.percent"));
    let psi_mem = metric_display(find_metric(snapshot, "pressure.memory.some.avg10.percent"));
    let psi_io = metric_display(find_metric(snapshot, "pressure.io.some.avg10.percent"));

    let load1 = metric_display(find_metric(snapshot, "load.1m"));
    let load5 = metric_display(find_metric(snapshot, "load.5m"));
    let load15 = metric_display(find_metric(snapshot, "load.15m"));

    let uptime =
        match find_metric(snapshot, "uptime.seconds").and_then(|m| m.state().available_value()) {
            Some(MetricValue::Number(s)) => format_uptime_duration(*s),
            Some(MetricValue::Integer(s)) => format_uptime_duration(*s as f64),
            _ => metric_display(find_metric(snapshot, "uptime.seconds")),
        };

    let net_devs = metric_display(find_metric(snapshot, "network.state"));
    let disk_devs = metric_display(find_metric(snapshot, "disk.state"));

    format!(
        "--- CPU Subsystem (/proc/stat) ---\n\
         CPU Busy: {cpu_busy}\n\
         Ticks: user {user} | sys {system} | idle {idle}\n\
         Wait: iowait {iowait} | steal {steal} | nice {nice}\n\
         Interrupts: irq {irq} | softirq {softirq}\n\
         PSI CPU Stall (avg10): {psi_cpu}\n\n\
         --- Memory Subsystem (/proc/meminfo) ---\n\
         {mem_summary}\n\
         PSI Memory Stall (avg10): {psi_mem}\n\n\
         --- Swap Space (/proc/meminfo) ---\n\
         {swap_summary}\n\n\
         --- System Activity & Pressure (/proc/loadavg & /proc/uptime) ---\n\
         Load Average: 1m: {load1} | 5m: {load5} | 15m: {load15}\n\
         System Uptime: {uptime}\n\
         PSI I/O Stall (avg10): {psi_io}\n\
         Active Network Devices: {net_devs}\n\
         Active Disk Devices: {disk_devs}\n\
         Collector Monotonic: {}ns",
        snapshot.monotonic_ns()
    )
}

/// Formats a comprehensive deep-dive network subsystem summary for the Network tab.
fn format_network_detail(
    report: Option<&lidb_core::network::NetworkReport>,
    snapshot: &Snapshot,
    selected_socket_idx: usize,
) -> String {
    let Some(report) = report else {
        return "Reading local procfs network tables; awaiting next sample.".into();
    };

    let sockets = report.sockets();
    let ifaces = report.interfaces();
    let routes = report.routes();

    let mut detail = String::new();

    let ns_desc = match report.namespace() {
        Some(ns) => match ns.inode() {
            Some(ino) => format!("{} (inode: {ino})", ns.name()),
            None => ns.name().to_string(),
        },
        None => "default / not identified".to_owned(),
    };

    // Section 1: Selected Socket Inspection
    let sock_start = if sockets.is_empty() {
        0
    } else {
        selected_socket_idx.min(sockets.len().saturating_sub(1))
    };

    if let Some(sock) = sockets.get(sock_start) {
        let local = crate::output::format_socket_endpoint(sock.local_address(), sock.local_port());
        let remote = match (sock.remote_address(), sock.remote_port()) {
            (Some(addr), Some(port)) => crate::output::format_socket_endpoint(addr, port),
            _ => "*".to_owned(),
        };
        let ino = sock.inode().map_or("-".into(), |i| i.to_string());
        let _ = write!(
            detail,
            "--- Selected Socket [{}/{}] ---\nProtocol: {}\nLocal Endpoint: {}\nRemote Endpoint: {}\nState: {}\nInode: {}\nNetwork Namespace: {}",
            sock_start + 1,
            sockets.len(),
            sock.protocol().as_str(),
            local,
            remote,
            sock.state().as_str(),
            ino,
            ns_desc
        );
    } else if ifaces.is_empty() && routes.is_empty() {
        detail.push_str("No network sockets or interfaces collected");
    } else {
        detail.push_str("--- Selected Socket ---\nNo network sockets collected");
    }

    // Section 2: Network Interfaces Throughput & Errors
    if !ifaces.is_empty() {
        detail.push_str("\n\n--- Network Interfaces Throughput & Errors ---");
        for iface in ifaces {
            let name = iface.name();
            let rx_rate = find_metric(snapshot, &format!("network.{name}.rx.bytes_per_second"))
                .map_or_else(|| "-".to_owned(), format_metric_rate);
            let tx_rate = find_metric(snapshot, &format!("network.{name}.tx.bytes_per_second"))
                .map_or_else(|| "-".to_owned(), format_metric_rate);
            let rx_bytes = find_metric(snapshot, &format!("network.{name}.rx.bytes"))
                .map_or_else(|| "-".to_owned(), value_text);
            let tx_bytes = find_metric(snapshot, &format!("network.{name}.tx.bytes"))
                .map_or_else(|| "-".to_owned(), value_text);
            let rx_err = find_metric(snapshot, &format!("network.{name}.rx.errors"))
                .map_or_else(|| "0".to_owned(), value_text);
            let tx_err = find_metric(snapshot, &format!("network.{name}.tx.errors"))
                .map_or_else(|| "0".to_owned(), value_text);
            let rx_drop = find_metric(snapshot, &format!("network.{name}.rx.drops"))
                .map_or_else(|| "0".to_owned(), value_text);
            let tx_drop = find_metric(snapshot, &format!("network.{name}.tx.drops"))
                .map_or_else(|| "0".to_owned(), value_text);
            let mtu = iface.mtu().map_or("-".into(), |m| m.to_string());
            let mac = iface.mac_address().map_or("-".into(), |m| m.to_string());
            let speed = iface
                .speed_mbps()
                .map_or("-".into(), |s| format!("{s} Mbps"));
            let flags = if iface.flags().is_empty() {
                "-".to_string()
            } else {
                format!("<{}>", iface.flags().join(","))
            };
            let _ = write!(
                detail,
                "\n\nInterface: {}: state {}\nState: {}\nMTU: {}\nMAC: {}\nSpeed: {}\nFlags: {}\nRX Rate: {} (total: {} B, err: {}, drop: {})\nTX Rate: {} (total: {} B, err: {}, drop: {})",
                name,
                iface.oper_state().as_str(),
                iface.oper_state().as_str(),
                mtu,
                mac,
                speed,
                flags,
                rx_rate,
                rx_bytes,
                rx_err,
                rx_drop,
                tx_rate,
                tx_bytes,
                tx_err,
                tx_drop,
            );
        }
    }

    // Section 3: Active Routing Table
    if !routes.is_empty() {
        let _ = write!(
            detail,
            "\n\n--- Active Routing Table ({} routes) ---",
            routes.len()
        );
        for (i, route) in routes.iter().enumerate() {
            let dst = format!("{}/{}", route.destination(), route.prefix_len());
            let gw = route
                .gateway()
                .map_or("direct / on-link".into(), |g| g.to_string());
            let iface = route.interface();
            let metric = route.metric().map_or("-".into(), |m| m.to_string());
            let family = route.family().as_str();
            let _ = write!(
                detail,
                "\n[{}] {} via {} dev {} metric {} ({})",
                i + 1,
                dst,
                gw,
                iface,
                metric,
                family
            );
        }
    } else {
        detail.push_str("\n\n--- Active Routing Table ---\nNo routes observed");
    }

    // Section 4: Diagnostics Status & Namespace Attribution
    let _ = write!(
        detail,
        "\n\n--- Diagnostics Status & Namespace Attribution ---\nNetwork Namespace: {}\nInterfaces: {} (dropped: {})\nRoutes: {} (dropped: {})\nSockets: {} (dropped: {})",
        ns_desc,
        report.interfaces_status().as_str(),
        report.dropped_interfaces(),
        report.routes_status().as_str(),
        report.dropped_routes(),
        report.sockets_status().as_str(),
        report.dropped_sockets(),
    );

    detail
}

/// Renders the dedicated CPU and Memory overview dashboard tab.
fn draw_cpu_mem(frame: &mut Frame<'_>, snapshot: &Snapshot, area: Rect) {
    let cpu_busy = find_metric(snapshot, "cpu.busy.percent");
    let (cpu_pct, cpu_label) = match cpu_busy.map(MetricObservation::state) {
        Some(MetricState::Available(val)) => {
            let p = val.as_f64();
            let pct = p.clamp(0.0, 100.0) as u16;
            (pct, format!("{p:.1}% [CPU Busy]"))
        }
        Some(MetricState::TemporarilyUnavailable(reason)) => (0, format!("CPU Busy: {reason}")),
        Some(state) => (0, format!("CPU Busy: {}", state.name())),
        None => (0, "CPU Busy: not probed".to_owned()),
    };

    let cpu_gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Overall CPU Utilization (/proc/stat) ")
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().add_modifier(Modifier::REVERSED))
        .percent(cpu_pct)
        .label(cpu_label);

    let mem_total = find_metric(snapshot, "memory.total.bytes");
    let mem_used = find_metric(snapshot, "memory.used.bytes");
    let mem_avail = find_metric(snapshot, "memory.available.bytes");

    let (mem_pct, mem_label) = match (
        mem_total.map(MetricObservation::state),
        mem_used.map(MetricObservation::state),
    ) {
        (Some(MetricState::Available(total_val)), Some(MetricState::Available(used_val))) => {
            let total = total_val.as_f64();
            let used = used_val.as_f64();
            let total_gib = total / 1_073_741_824.0;
            let used_gib = used / 1_073_741_824.0;
            let avail_str = match mem_avail.map(MetricObservation::state) {
                Some(MetricState::Available(avail_val)) => {
                    format!("{:.2} GiB avail", avail_val.as_f64() / 1_073_741_824.0)
                }
                _ => "avail n/a".to_owned(),
            };
            let pct = if total > 0.0 {
                ((used / total) * 100.0).clamp(0.0, 100.0) as u16
            } else {
                0
            };
            (
                pct,
                format!("{used_gib:.2} GiB / {total_gib:.2} GiB ({pct}%, {avail_str})"),
            )
        }
        _ => {
            let total_str = mem_total.map_or_else(|| "not probed".to_owned(), value_text);
            let used_str = mem_used.map_or_else(|| "not probed".to_owned(), value_text);
            (0, format!("RAM: used {used_str} / total {total_str}"))
        }
    };

    let mem_gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Physical Memory (RAM) ")
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().add_modifier(Modifier::REVERSED))
        .percent(mem_pct)
        .label(mem_label);

    let swap_total = find_metric(snapshot, "swap.total.bytes");
    let swap_used = find_metric(snapshot, "swap.used.bytes");

    let (swap_pct, swap_label) = match (
        swap_total.map(MetricObservation::state),
        swap_used.map(MetricObservation::state),
    ) {
        (Some(MetricState::Available(total_val)), Some(MetricState::Available(used_val))) => {
            let total = total_val.as_f64();
            let used = used_val.as_f64();
            if total > 0.0 {
                let total_gib = total / 1_073_741_824.0;
                let used_gib = used / 1_073_741_824.0;
                let pct = ((used / total) * 100.0).clamp(0.0, 100.0) as u16;
                (
                    pct,
                    format!("{used_gib:.2} GiB / {total_gib:.2} GiB ({pct}%)"),
                )
            } else {
                (0, "No swap configured (0 B)".to_owned())
            }
        }
        _ => {
            let total_str = swap_total.map_or_else(|| "not probed".to_owned(), value_text);
            let used_str = swap_used.map_or_else(|| "not probed".to_owned(), value_text);
            (0, format!("Swap: used {used_str} / total {total_str}"))
        }
    };

    let swap_gauge = Gauge::default()
        .block(Block::default().title(" Swap Space ").borders(Borders::ALL))
        .gauge_style(Style::default().add_modifier(Modifier::REVERSED))
        .percent(swap_pct)
        .label(swap_label);

    if area.height <= 7 {
        let rows = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);
        frame.render_widget(cpu_gauge, rows[0]);
        frame.render_widget(mem_gauge, rows[1]);
        return;
    }

    if area.height < 14 {
        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);
        frame.render_widget(cpu_gauge, rows[0]);
        if area.width >= 70 {
            let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[1]);
            frame.render_widget(mem_gauge, cols[0]);
            frame.render_widget(swap_gauge, cols[1]);
        } else {
            frame.render_widget(mem_gauge, rows[1]);
        }
        let psi_cpu = metric_display(find_metric(snapshot, "pressure.cpu.some.avg10.percent"));
        let psi_mem = metric_display(find_metric(snapshot, "pressure.memory.some.avg10.percent"));
        let psi_io = metric_display(find_metric(snapshot, "pressure.io.some.avg10.percent"));
        let psi_text = if area.width >= 60 {
            format!("PSI avg10: CPU {psi_cpu} | Mem {psi_mem} | I/O {psi_io}")
        } else {
            format!("PSI CPU: {psi_cpu}\nMem: {psi_mem} | I/O: {psi_io}")
        };
        frame.render_widget(
            Paragraph::new(psi_text).wrap(Wrap { trim: true }).block(
                Block::default()
                    .title(" Pressure Stalls ")
                    .borders(Borders::ALL),
            ),
            rows[2],
        );
        return;
    }

    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(4),
        Constraint::Min(4),
    ])
    .split(area);

    frame.render_widget(cpu_gauge, rows[0]);

    if area.width >= 70 {
        let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);
        frame.render_widget(mem_gauge, cols[0]);
        frame.render_widget(swap_gauge, cols[1]);
    } else {
        frame.render_widget(mem_gauge, rows[1]);
    }

    let user = metric_display(find_metric(snapshot, "cpu.user.ticks"));
    let system = metric_display(find_metric(snapshot, "cpu.system.ticks"));
    let idle = metric_display(find_metric(snapshot, "cpu.idle.ticks"));
    let iowait = metric_display(find_metric(snapshot, "cpu.iowait.ticks"));
    let steal = metric_display(find_metric(snapshot, "cpu.steal.ticks"));
    let nice = metric_display(find_metric(snapshot, "cpu.nice.ticks"));

    let ticks_lines = if area.width >= 70 {
        vec![
            Line::from(format!("User: {user}   System: {system}   Idle: {idle}")),
            Line::from(format!(
                "I/O Wait: {iowait}   Steal: {steal}   Nice: {nice}"
            )),
        ]
    } else if area.width >= 48 {
        vec![
            Line::from(format!("usr: {user}  sys: {system}  idl: {idle}")),
            Line::from(format!("iow: {iowait}  stl: {steal}  nic: {nice}")),
        ]
    } else {
        vec![
            Line::from(format!("u:{user} s:{system} i:{idle}")),
            Line::from(format!("w:{iowait} st:{steal} ni:{nice}")),
        ]
    };

    let ticks_paragraph = Paragraph::new(ticks_lines).wrap(Wrap { trim: true }).block(
        Block::default()
            .title(" CPU Ticks (/proc/stat) ")
            .borders(Borders::ALL),
    );
    frame.render_widget(ticks_paragraph, rows[2]);

    let psi_cpu = metric_display(find_metric(snapshot, "pressure.cpu.some.avg10.percent"));
    let psi_mem = metric_display(find_metric(snapshot, "pressure.memory.some.avg10.percent"));
    let psi_io = metric_display(find_metric(snapshot, "pressure.io.some.avg10.percent"));

    let load1 = metric_display(find_metric(snapshot, "load.1m"));
    let load5 = metric_display(find_metric(snapshot, "load.5m"));
    let load15 = metric_display(find_metric(snapshot, "load.15m"));
    let uptime =
        match find_metric(snapshot, "uptime.seconds").and_then(|m| m.state().available_value()) {
            Some(MetricValue::Number(s)) => format_uptime_duration(*s),
            Some(MetricValue::Integer(s)) => format_uptime_duration(*s as f64),
            _ => metric_display(find_metric(snapshot, "uptime.seconds")),
        };

    let psi_load_lines = if area.width >= 70 {
        vec![
            Line::from(format!(
                "PSI Stall avg10: CPU {psi_cpu}  │  Mem {psi_mem}  │  I/O {psi_io}"
            )),
            Line::from(format!(
                "Load Average: 1m: {load1}  5m: {load5}  15m: {load15}  │  Uptime: {uptime}"
            )),
        ]
    } else if area.width >= 48 {
        vec![
            Line::from(format!(
                "PSI avg10: CPU {psi_cpu} | Mem {psi_mem} | IO {psi_io}"
            )),
            Line::from(format!("Load: {load1} {load5} {load15} │ Up: {uptime}")),
        ]
    } else {
        vec![
            Line::from(format!("PSI: C:{psi_cpu} M:{psi_mem} IO:{psi_io}")),
            Line::from(format!("Load: {load1} {load5} {load15}")),
            Line::from(format!("Uptime: {uptime}")),
        ]
    };

    let psi_load_paragraph = Paragraph::new(psi_load_lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" PSI & Load (/proc/pressure & /proc/loadavg) ")
                .borders(Borders::ALL),
        );
    frame.render_widget(psi_load_paragraph, rows[3]);
}

/// Renders the dedicated Network overview dashboard tab.
fn draw_network(
    frame: &mut Frame<'_>,
    network: Option<&lidb_core::network::NetworkReport>,
    snapshot: Option<&Snapshot>,
    scroll: usize,
    area: Rect,
) {
    let Some(report) = network else {
        frame.render_widget(
            Paragraph::new("Reading local procfs network tables; awaiting next sample.")
                .block(Block::default().title(" Network ").borders(Borders::ALL)),
            area,
        );
        return;
    };

    let ns_desc = match report.namespace() {
        Some(ns) => match ns.inode() {
            Some(ino) => format!("Namespace: {} (inode: {ino})", ns.name()),
            None => format!("Namespace: {}", ns.name()),
        },
        None => "Namespace: default / not identified".to_owned(),
    };

    let summary_text = format!(
        "{ns_desc} │ Ifaces: {} (dropped: {}) │ Routes: {} (dropped: {}) │ Sockets: {} (dropped: {})",
        report.interfaces_status().as_str(),
        report.dropped_interfaces(),
        report.routes_status().as_str(),
        report.dropped_routes(),
        report.sockets_status().as_str(),
        report.dropped_sockets(),
    );

    if area.height <= 10 {
        let text = format!(
            "Ifaces: {} ({}) │ Routes: {} ({})\nSockets: {} ({})\n{ns_desc}",
            report.interfaces().len(),
            report.interfaces_status().as_str(),
            report.routes().len(),
            report.routes_status().as_str(),
            report.sockets().len(),
            report.sockets_status().as_str(),
        );
        frame.render_widget(
            Paragraph::new(text).wrap(Wrap { trim: true }).block(
                Block::default()
                    .title(" Network Overview ")
                    .borders(Borders::ALL),
            ),
            area,
        );
        return;
    }

    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Percentage(45),
        Constraint::Percentage(55),
    ])
    .split(area);

    let summary_widget = Paragraph::new(summary_text).block(
        Block::default()
            .title(" Network Diagnostics Status ")
            .borders(Borders::ALL),
    );
    frame.render_widget(summary_widget, rows[0]);

    let iface_inner_height = rows[1].height.saturating_sub(2) as usize;
    let iface_width = if rows[1].width >= 80 {
        rows[1].width / 2
    } else {
        rows[1].width
    };

    let (headers, widths, iface_rows): (Vec<&str>, Vec<Constraint>, Vec<Row>) = if iface_width >= 60
    {
        (
            vec!["Name", "State", "MTU", "RX Rate", "TX Rate"],
            vec![
                Constraint::Percentage(22),
                Constraint::Percentage(16),
                Constraint::Percentage(14),
                Constraint::Percentage(24),
                Constraint::Percentage(24),
            ],
            report
                .interfaces()
                .iter()
                .take(iface_inner_height.max(1))
                .map(|iface| {
                    let name = iface.name();
                    let rx_rate = snapshot
                        .and_then(|s| {
                            find_metric(s, &format!("network.{name}.rx.bytes_per_second"))
                        })
                        .map_or_else(|| "-".to_owned(), format_metric_rate);
                    let tx_rate = snapshot
                        .and_then(|s| {
                            find_metric(s, &format!("network.{name}.tx.bytes_per_second"))
                        })
                        .map_or_else(|| "-".to_owned(), format_metric_rate);
                    let mtu = iface.mtu().map_or("-".into(), |m| m.to_string());
                    Row::new(vec![
                        name.to_string(),
                        iface.oper_state().as_str().to_string(),
                        mtu,
                        rx_rate,
                        tx_rate,
                    ])
                })
                .collect(),
        )
    } else {
        (
            vec!["Name", "State", "RX Rate", "TX Rate"],
            vec![
                Constraint::Percentage(28),
                Constraint::Percentage(20),
                Constraint::Percentage(26),
                Constraint::Percentage(26),
            ],
            report
                .interfaces()
                .iter()
                .take(iface_inner_height.max(1))
                .map(|iface| {
                    let name = iface.name();
                    let rx_rate = snapshot
                        .and_then(|s| {
                            find_metric(s, &format!("network.{name}.rx.bytes_per_second"))
                        })
                        .map_or_else(|| "-".to_owned(), format_metric_rate);
                    let tx_rate = snapshot
                        .and_then(|s| {
                            find_metric(s, &format!("network.{name}.tx.bytes_per_second"))
                        })
                        .map_or_else(|| "-".to_owned(), format_metric_rate);
                    Row::new(vec![
                        name.to_string(),
                        iface.oper_state().as_str().to_string(),
                        rx_rate,
                        tx_rate,
                    ])
                })
                .collect(),
        )
    };

    let iface_title = if report.interfaces().len() > iface_rows.len() {
        format!(
            " Interfaces (showing {} of {}) ",
            iface_rows.len(),
            report.interfaces().len()
        )
    } else {
        format!(" Interfaces ({}) ", report.interfaces().len())
    };
    let iface_block = Block::default().title(iface_title).borders(Borders::ALL);

    let iface_table = Table::new(iface_rows, widths)
        .header(Row::new(headers).style(Style::default().add_modifier(Modifier::BOLD)))
        .block(iface_block);

    let route_rows: Vec<Row> = report
        .routes()
        .iter()
        .take(iface_inner_height.max(1))
        .map(|route| {
            let dst = format!("{}/{}", route.destination(), route.prefix_len());
            let gw = route.gateway().map_or("-".into(), |g| g.to_string());
            let iface = route.interface().to_string();
            let metric = route.metric().map_or("-".into(), |m| m.to_string());
            Row::new(vec![dst, gw, iface, metric])
        })
        .collect();

    let route_title = if report.routes().len() > route_rows.len() {
        format!(
            " Routes (showing {} of {}) ",
            route_rows.len(),
            report.routes().len()
        )
    } else {
        format!(" Routes ({}) ", report.routes().len())
    };
    let route_block = Block::default().title(route_title).borders(Borders::ALL);

    let route_table = Table::new(
        route_rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(18),
            Constraint::Percentage(12),
        ],
    )
    .header(
        Row::new(["Destination", "Gateway", "Iface", "Metric"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(route_block);

    if rows[1].width >= 80 {
        let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);
        frame.render_widget(iface_table, cols[0]);
        frame.render_widget(route_table, cols[1]);
    } else {
        let split_rows = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);
        frame.render_widget(iface_table, split_rows[0]);
        frame.render_widget(route_table, split_rows[1]);
    }

    let socket_count = report.sockets().len();
    let sock_inner_height = rows[2].height.saturating_sub(2) as usize;
    let start = if socket_count == 0 {
        0
    } else {
        scroll.min(socket_count.saturating_sub(1))
    };
    let end = (start + sock_inner_height).min(socket_count);
    let title = if socket_count == 0 {
        " Sockets (0) ".to_string()
    } else {
        format!(" Sockets {}..{} of {} ", start + 1, end, socket_count)
    };

    let socket_block = Block::default().title(title).borders(Borders::ALL);

    let sock_rows: Vec<Row> = report
        .sockets()
        .iter()
        .skip(start)
        .take(sock_inner_height.max(1))
        .map(|sock| {
            let local =
                crate::output::format_socket_endpoint(sock.local_address(), sock.local_port());
            let remote = match (sock.remote_address(), sock.remote_port()) {
                (Some(addr), Some(port)) => crate::output::format_socket_endpoint(addr, port),
                _ => "*".to_owned(),
            };
            let ino = sock.inode().map_or("-".into(), |i| i.to_string());
            Row::new(vec![
                sock.protocol().as_str().to_string(),
                local,
                remote,
                sock.state().as_str().to_string(),
                ino,
            ])
        })
        .collect();

    let sock_table = Table::new(
        sock_rows,
        [
            Constraint::Length(6),
            Constraint::Percentage(32),
            Constraint::Percentage(32),
            Constraint::Percentage(18),
            Constraint::Percentage(12),
        ],
    )
    .header(
        Row::new(["Proto", "Local", "Remote", "State", "Inode"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(socket_block);

    frame.render_widget(sock_table, rows[2]);
}

fn draw_ebpf(frame: &mut Frame<'_>, ebpf: Option<&EbpfReport>, scroll: usize, area: Rect) {
    let Some(report) = ebpf else {
        frame.render_widget(
            Paragraph::new(
                "eBPF lifecycle tracing is disabled by default.\n\n\
                 Run lidash with --ebpf to enable in-kernel TCP lifecycle tracing:\n\
                   cargo run -p lidash -- --ebpf\n\
                   lidash tui --ebpf\n\n\
                 Requirements:\n\
                 - Linux kernel 4.16+ with BTF (/sys/kernel/btf/vmlinux)\n\
                 - CAP_BPF or CAP_SYS_ADMIN privileges (or unprivileged safe fallback)",
            )
            .block(
                Block::default()
                    .title(" eBPF Socket Lifecycle Tracing ")
                    .borders(Borders::ALL),
            ),
            area,
        );
        return;
    };

    match report.status() {
        SourceAvailability::Disabled(reason) => {
            let msg = format!("eBPF collection is disabled: {reason}\nPass --ebpf to enable.");
            frame.render_widget(
                Paragraph::new(msg).block(
                    Block::default()
                        .title(" eBPF Socket Lifecycle Tracing ")
                        .borders(Borders::ALL),
                ),
                area,
            );
            return;
        }
        SourceAvailability::PermissionDenied(reason) => {
            let msg = format!(
                "Permission denied: {reason}\n\n\
                 eBPF tracepoints require elevated capabilities:\n\
                 - CAP_BPF or CAP_SYS_ADMIN (root)\n\
                 Running unprivileged safely falls back without crashing."
            );
            frame.render_widget(
                Paragraph::new(msg).block(
                    Block::default()
                        .title(" eBPF Permission Denied ")
                        .borders(Borders::ALL),
                ),
                area,
            );
            return;
        }
        SourceAvailability::Unsupported(reason) => {
            let msg = format!(
                "eBPF unsupported on this platform: {reason}\n\n\
                 Supported platform requirements:\n\
                 - Linux kernel 4.16+\n\
                 - Kernel BTF (/sys/kernel/btf/vmlinux)"
            );
            frame.render_widget(
                Paragraph::new(msg).block(
                    Block::default()
                        .title(" eBPF Unsupported ")
                        .borders(Borders::ALL),
                ),
                area,
            );
            return;
        }
        _ => {}
    }

    let summary_text = format!(
        "Tracepoint: sock:inet_sock_set_state │ Status: {} │ Events: {} (dropped: {}) │ Ringbuf: 256KB",
        report.status().as_str(),
        report.events().len(),
        report.dropped_events(),
    );

    if area.height <= 10 {
        let text = format!(
            "{summary_text}\nEvents: {}\nMode: {}",
            report.events().len(),
            mode_name(report.mode())
        );
        frame.render_widget(
            Paragraph::new(text).wrap(Wrap { trim: true }).block(
                Block::default()
                    .title(" eBPF Lifecycle Overview ")
                    .borders(Borders::ALL),
            ),
            area,
        );
        return;
    }

    let rows = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);

    let summary_widget = Paragraph::new(summary_text).block(
        Block::default()
            .title(" eBPF Socket Lifecycle Status ")
            .borders(Borders::ALL),
    );
    frame.render_widget(summary_widget, rows[0]);

    let events = report.events();
    if events.is_empty() {
        frame.render_widget(
            Paragraph::new("No TCP lifecycle events observed yet. Monitoring tracepoint sock:inet_sock_set_state...").block(
                Block::default()
                    .title(" eBPF Lifecycle Events (0) ")
                    .borders(Borders::ALL),
            ),
            rows[1],
        );
        return;
    }

    let inner_height = rows[1].height.saturating_sub(2) as usize;
    let start = scroll.min(events.len().saturating_sub(1));
    let table_rows: Vec<Row> = events
        .iter()
        .skip(start)
        .take(inner_height.max(1))
        .map(|event| {
            let event_type = event.event_type().as_str().to_string();
            let comm_pid = format!("{} [{}]", event.comm(), event.pid());
            let family = event.family().as_str().to_string();
            let src = format!("{}:{}", event.local_address(), event.local_port());
            let dst = format!("{}:{}", event.remote_address(), event.remote_port());
            let states = match (event.old_state(), event.new_state()) {
                (Some(old), Some(new)) => format!("{} -> {}", old.as_str(), new.as_str()),
                (None, Some(new)) => format!("-> {}", new.as_str()),
                (Some(old), None) => format!("{} ->", old.as_str()),
                (None, None) => "-".to_string(),
            };
            let rtt = event
                .round_trip_us()
                .map_or("-".to_string(), |us| format!("{:.1}ms", us as f64 / 1000.0));
            Row::new(vec![event_type, comm_pid, family, src, dst, states, rtt])
        })
        .collect();

    let table = Table::new(
        table_rows,
        [
            Constraint::Length(12),
            Constraint::Length(16),
            Constraint::Length(6),
            Constraint::Percentage(22),
            Constraint::Percentage(22),
            Constraint::Length(24),
            Constraint::Length(8),
        ],
    )
    .header(
        Row::new([
            "Event",
            "Process",
            "Family",
            "Source",
            "Destination",
            "Transition",
            "RTT",
        ])
        .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .title(format!(
                " Events {}..{} of {} ",
                start.saturating_add(1),
                (start + inner_height).min(events.len()),
                events.len()
            ))
            .borders(Borders::ALL),
    );
    frame.render_widget(table, rows[1]);
}

struct TerminalGuard;
impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        // Install the guard before fallible alternate-screen initialization.
        let guard = Self;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(guard)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

/// Restores terminal raw mode and screen settings.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
}

/// Fetches the latest collection update from the background worker if not paused.
pub(crate) fn update_from_worker(view: &mut View, worker: &Worker) -> io::Result<()> {
    // Leave the bounded mailbox untouched while paused so resume shows its latest
    // snapshot immediately, even when the next sample is a minute away.
    if !view.paused {
        if let Some(update) = worker.take_latest()? {
            view.push_update(update);
        }
    }
    Ok(())
}

struct SignalGuard {
    stop: Arc<AtomicBool>,
    ids: Vec<signal_hook::SigId>,
}

impl SignalGuard {
    fn register() -> io::Result<Self> {
        let mut guard = Self {
            stop: Arc::new(AtomicBool::new(false)),
            ids: Vec::new(),
        };
        for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
            guard.ids.push(signal_hook::flag::register(
                signal,
                Arc::clone(&guard.stop),
            )?);
        }
        Ok(guard)
    }
}

impl Drop for SignalGuard {
    fn drop(&mut self) {
        for id in self.ids.drain(..) {
            signal_hook::low_level::unregister(id);
        }
    }
}

/// Runs the interactive terminal dashboard event loop until user exit or termination signal.
pub fn run(config: Config) -> io::Result<()> {
    let mut collector = crate::collector(&config);
    let ebpf_enabled = config.ebpf;
    run_with_sampler(config, move || {
        let snapshot = collector.sample();
        let network = collector.network();
        let ebpf = if ebpf_enabled {
            Some(collector.ebpf(true))
        } else {
            None
        };
        (snapshot, Some(network), ebpf)
    })
}

pub(crate) fn run_with_sampler(
    config: Config,
    sample: impl FnMut() -> (
            Snapshot,
            Option<NetworkReport>,
            Option<lidb_core::ebpf::EbpfReport>,
        ) + Send
        + 'static,
) -> io::Result<()> {
    // Restore before the default panic hook prints; the guard also covers unwinding/errors.
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous(info);
    }));
    let mut worker = Worker::start(sample, config.interval)?;
    let signals = SignalGuard::register()?;
    let guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut view = View {
        fixture: config.fixture,
        ..View::default()
    };
    loop {
        if signals.stop.load(Ordering::Relaxed) {
            break;
        }
        worker.check_collection()?;
        update_from_worker(&mut view, &worker)?;
        if worker.is_finished() {
            // The worker can reject a late batch after the check above.
            worker.check_collection()?;
            drop(guard);
            worker.shutdown()?;
            return Err(io::Error::other("collector stopped unexpectedly"));
        }
        terminal.draw(|frame| draw(frame, &mut view))?;
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char(' ') => {
                        view.paused = !view.paused;
                        if !view.paused {
                            view.history_cursor = None;
                            if let Some(latest) = view.history.back() {
                                view.update = Some(latest.clone());
                            }
                        }
                    }
                    KeyCode::Char('[') | KeyCode::Char(',') => {
                        view.step_history_back();
                    }
                    KeyCode::Char(']') | KeyCode::Char('.') => {
                        view.step_history_forward();
                    }
                    KeyCode::Char('?') | KeyCode::Char('h') => {
                        view.help = !view.help;
                        view.text_scroll = 0;
                    }
                    KeyCode::Enter => {
                        view.detail = !view.detail;
                        view.text_scroll = 0;
                    }
                    KeyCode::Tab => {
                        view.tab = view.tab.next();
                        view.scroll = 0;
                        view.detail = false;
                    }
                    KeyCode::BackTab => {
                        view.tab = view.tab.prev();
                        view.scroll = 0;
                        view.detail = false;
                    }
                    KeyCode::Char('1') => {
                        view.tab = Tab::Metrics;
                        view.scroll = 0;
                        view.detail = false;
                    }
                    KeyCode::Char('2') => {
                        view.tab = Tab::CpuMemory;
                        view.scroll = 0;
                        view.detail = false;
                    }
                    KeyCode::Char('3') => {
                        view.tab = Tab::Network;
                        view.scroll = 0;
                        view.detail = false;
                    }
                    KeyCode::Char('4') => {
                        view.tab = Tab::Ebpf;
                        view.scroll = 0;
                        view.detail = false;
                    }
                    KeyCode::Left if !view.help && !view.detail => {
                        view.tab = view.tab.prev();
                        view.scroll = 0;
                    }
                    KeyCode::Right if !view.help && !view.detail => {
                        view.tab = view.tab.next();
                        view.scroll = 0;
                    }
                    KeyCode::Down | KeyCode::Char('j') if view.help || view.detail => {
                        view.text_scroll = view.text_scroll.saturating_add(1);
                    }
                    KeyCode::Up | KeyCode::Char('k') if view.help || view.detail => {
                        view.text_scroll = view.text_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => view.scroll_by(1),
                    KeyCode::Up | KeyCode::Char('k') => view.scroll_by(-1),
                    KeyCode::Char('n') => view.scroll_by(1),
                    KeyCode::Char('p') => view.scroll_by(-1),
                    KeyCode::PageDown if view.help || view.detail => view.scroll_text_page(true),
                    KeyCode::PageUp if view.help || view.detail => view.scroll_text_page(false),
                    KeyCode::PageDown => view.scroll_by(10),
                    KeyCode::PageUp => view.scroll_by(-10),
                    KeyCode::Home if view.help || view.detail => {
                        view.text_scroll = 0;
                    }
                    KeyCode::Home => {
                        view.scroll = 0;
                        view.text_scroll = 0;
                    }
                    KeyCode::End if view.help || view.detail => {
                        view.text_scroll = u16::MAX;
                    }
                    KeyCode::End => view.scroll_by(isize::MAX),
                    _ => {}
                }
            }
        }
    }
    drop(guard);
    worker.shutdown()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lidb_core::{MetricObservation, MetricState, MetricValue, Snapshot, SnapshotMode, Unit};
    use ratatui::backend::TestBackend;
    use std::time::Instant;

    fn fixture() -> View {
        let mut snapshot = Snapshot::new(0, 100, SnapshotMode::Fixture);
        snapshot
            .push(
                MetricObservation::new(
                    "memory.total.bytes",
                    "procfs/meminfo",
                    Unit::Bytes,
                    0,
                    MetricState::Available(MetricValue::Integer(0)),
                )
                .unwrap(),
            )
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "pressure.cpu",
                    "procfs/pressure/cpu",
                    Unit::Percent,
                    0,
                    MetricState::Unsupported("not exposed — unavailable".into()),
                )
                .unwrap(),
            )
            .unwrap();
        View {
            update: Some(Update {
                snapshot,
                network: None,
                ebpf: None,
                collected_at: Instant::now(),
                skipped_updates: 3,
            }),
            fixture: true,
            ..View::default()
        }
    }

    fn render(width: u16, height: u16, view: &mut View) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, view)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn wide_and_narrow_views_retain_source_state_and_fixture_label() {
        let mut view = fixture();
        for width in [120, 70, 40] {
            let output = render(width, 30, &mut view);
            assert!(output.contains("fixture"));
            assert!(output.contains("memory.total.bytes"));
            assert!(output.contains("procfs/meminfo"));
            assert!(output.contains("unsupported"));
            assert!(output.contains("bytes"));
            assert!(output.contains("age"));
        }
    }

    #[test]
    fn tiny_resize_pause_help_and_extreme_scroll_are_safe() {
        let mut view = fixture();
        view.paused = true;
        assert!(render(120, 30, &mut view).contains("PAUSED"));
        view.help = true;
        assert!(render(70, 30, &mut view).contains("sampling continues"));
        assert!(render(120, 30, &mut view).contains("8888888b"));
        assert!(!render(70, 30, &mut view).contains("8888888b"));
        for (width, height) in [(0, 0), (1, 1), (20, 4), (35, 7), (120, 30)] {
            let _ = render(width, height, &mut view);
        }
        view.scroll_by(isize::MAX);
        assert_eq!(view.scroll, 1);
        view.scroll_by(isize::MIN);
        assert_eq!(view.scroll, 0);
        assert!(render(25, 4, &mut view).contains("fixture"));
        assert!(render(35, 7, &mut view).contains("Terminal too small"));
        view.help = false;
        let minimum = render(40, 10, &mut view);
        assert!(minimum.contains("procfs/meminfo"));
        assert!(minimum.contains("bytes"));
        view.detail = true;
        view.scroll = 1;
        let detail = render(90, 20, &mut view);
        assert!(detail.contains("unsupported: not exposed — unavailable"));
        assert!(detail.contains("procfs/pressure/cpu"));
    }

    #[test]
    fn minimum_size_details_and_help_scroll_to_all_evidence() {
        let mut view = fixture();
        view.detail = true;
        view.scroll = 1;
        view.text_scroll = u16::MAX;
        let bottom = render(35, 10, &mut view);
        assert!(bottom.contains("source: procfs/pressure/cpu"));
        assert!(bottom.contains("observation:"));
        assert!(view.text_scroll > 0 && view.text_scroll < u16::MAX);
        view.scroll_by(-1);
        assert_eq!(view.text_scroll, 0);
        view.help = true;
        view.text_scroll = u16::MAX;
        assert!(render(35, 10, &mut view).contains("measurements."));
        assert!(view.text_scroll < u16::MAX);
        view.text_scroll = 0;
        assert!(render(35, 10, &mut view).contains("Ctrl-C: quit"));
    }

    #[test]
    fn narrow_detail_pages_do_not_skip_middle_failure_lines() {
        let mut snapshot = Snapshot::new(0, 0, SnapshotMode::Fixture);
        snapshot
            .push(
                MetricObservation::new(
                    "network.docker0.rx.bytes_per_second",
                    "proc.net.dev",
                    Unit::BytesPerSecond,
                    0,
                    MetricState::TemporarilyUnavailable(
                        "rate requires a second successful observation".into(),
                    ),
                )
                .unwrap(),
            )
            .unwrap();
        let mut view = View {
            update: Some(Update {
                snapshot,
                network: None,
                ebpf: None,
                collected_at: Instant::now(),
                skipped_updates: 0,
            }),
            detail: true,
            fixture: true,
            ..View::default()
        };
        let mut pages = render(35, 10, &mut view);
        assert_eq!(view.text_page_size, 4);
        loop {
            let previous = view.text_scroll;
            view.scroll_text_page(true);
            pages.push_str(&render(35, 10, &mut view));
            if view.text_scroll == previous {
                break;
            }
        }
        assert!(pages.contains("successful"));
        assert!(pages.contains("bytes_per_second"));
        assert!(pages.contains("source: proc.net.dev"));
        assert!(pages.contains("observation:"));
        view.scroll_text_page(false);
        assert!(view.text_scroll <= view.text_page_size);
    }

    #[test]
    fn cpu_memory_tab_renders_gauges_and_subsystems() {
        let mut view = fixture();
        view.tab = Tab::CpuMemory;
        for width in [120, 70] {
            let output = render(width, 30, &mut view);
            assert!(output.contains("Overall CPU Utilization"));
            assert!(output.contains("Physical Memory (RAM)"));
            assert!(output.contains("Swap Space"));
        }
        let narrow = render(40, 30, &mut view);
        assert!(narrow.contains("Overall CPU Utilization"));
        assert!(narrow.contains("Physical Memory (RAM)"));
        assert!(narrow.contains("Uptime"));
        assert!(narrow.contains("Load"));

        let minimum = render(35, 10, &mut view);
        assert!(minimum.contains("CPU"));
        assert!(minimum.contains("RAM"));
    }

    #[test]
    fn cpu_memory_tab_renders_fallback_labels_when_metrics_unavailable() {
        let mut snapshot = Snapshot::new(0, 100, SnapshotMode::Fixture);
        snapshot
            .push(
                MetricObservation::new(
                    "memory.total.bytes",
                    "procfs/meminfo",
                    Unit::Bytes,
                    0,
                    MetricState::Available(MetricValue::Integer(16_000_000_000)),
                )
                .unwrap(),
            )
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "memory.used.bytes",
                    "procfs/meminfo",
                    Unit::Bytes,
                    0,
                    MetricState::TemporarilyUnavailable("baseline required".into()),
                )
                .unwrap(),
            )
            .unwrap();
        let mut view = View {
            update: Some(Update {
                snapshot,
                network: None,
                ebpf: None,
                collected_at: Instant::now(),
                skipped_updates: 0,
            }),
            tab: Tab::CpuMemory,
            ..View::default()
        };
        let output = render(100, 30, &mut view);
        assert!(output.contains("RAM: used temporarily_unavailable"));
    }

    #[test]
    fn tab_navigation_cycles_correctly() {
        let tab = Tab::Metrics;
        assert_eq!(tab.next(), Tab::CpuMemory);
        assert_eq!(tab.next().next(), Tab::Network);
        assert_eq!(tab.next().next().next(), Tab::Ebpf);
        assert_eq!(tab.next().next().next().next(), Tab::Metrics);
        assert_eq!(tab.prev(), Tab::Ebpf);
        assert_eq!(tab.prev().prev(), Tab::Network);
        assert_eq!(tab.prev().prev().prev(), Tab::CpuMemory);
        assert_eq!(tab.prev().prev().prev().prev(), Tab::Metrics);
    }

    #[test]
    fn ebpf_tab_renders_disabled_and_unavailable_states_cleanly() {
        let mut view = fixture();
        view.tab = Tab::Ebpf;

        // When ebpf is None (not enabled via --ebpf)
        let out_disabled = render(100, 30, &mut view);
        assert!(out_disabled.contains("eBPF lifecycle tracing is disabled by default"));
        assert!(out_disabled.contains("--ebpf"));

        // When ebpf report is present but permission denied
        let mut report = EbpfReport::new(SnapshotMode::Fixture, 100);
        report.set_status(SourceAvailability::PermissionDenied(
            "Missing CAP_BPF".into(),
        ));
        view.update.as_mut().unwrap().ebpf = Some(report);
        let out_denied = render(100, 30, &mut view);
        assert!(out_denied.contains("Permission denied: Missing CAP_BPF"));
        assert!(out_denied.contains("CAP_BPF or CAP_SYS_ADMIN"));

        // When ebpf report is present but unsupported
        let mut report_unsupported = EbpfReport::new(SnapshotMode::Fixture, 100);
        report_unsupported.set_status(SourceAvailability::Unsupported("Kernel BTF missing".into()));
        view.update.as_mut().unwrap().ebpf = Some(report_unsupported);
        let out_unsupported = render(100, 30, &mut view);
        assert!(out_unsupported.contains("eBPF unsupported on this platform"));
        assert!(out_unsupported.contains("Kernel BTF missing"));
    }

    #[test]
    fn ebpf_tab_renders_events_and_transitions() {
        use lidb_core::ebpf::{EbpfEventType, EbpfLifecycleEvent};
        use lidb_core::network::SocketState;
        use std::net::{IpAddr, Ipv4Addr};

        let mut report = EbpfReport::new(SnapshotMode::Fixture, 500);
        report.set_status(SourceAvailability::Available);

        let event = EbpfLifecycleEvent::new(
            EbpfEventType::TcpConnect,
            123456789,
            4242,
            "curl",
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
            45678,
            IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
            80,
        )
        .unwrap()
        .with_states(Some(SocketState::SynSent), Some(SocketState::Established))
        .with_round_trip_us(Some(12400));
        report.push_event(event);

        let mut view = fixture();
        view.tab = Tab::Ebpf;
        view.update.as_mut().unwrap().ebpf = Some(report);

        for width in [120, 85, 50] {
            let out = render(width, 30, &mut view);
            assert!(out.contains("eBPF Socket Lifecycle Status"));
            assert!(out.contains("Tracepoint: sock:inet_sock_set_state"));
            if width >= 85 {
                assert!(out.contains("curl [4242]"));
                assert!(out.contains("tcp_connect"));
                assert!(out.contains("syn_sent -> established"));
                assert!(out.contains("12.4ms"));
            }
            if width >= 120 {
                assert!(out.contains("192.168.1.10:45678"));
                assert!(out.contains("93.184.216.34:80"));
            }
        }

        // Small height test
        let small = render(80, 10, &mut view);
        assert!(small.contains("eBPF Lifecycle Overview"));
        assert!(small.contains("Events: 1"));
    }

    #[test]
    fn network_tab_renders_interfaces_routes_and_sockets() {
        use lidb_core::network::{
            InterfaceOperState, InterfaceRecord, NetworkNamespace, NetworkReport, RouteRecord,
            SocketProtocol, SocketRecord, SocketState, SourceAvailability,
        };
        use std::net::{IpAddr, Ipv4Addr};

        let mut net_report = NetworkReport::new(100, 20, SnapshotMode::Fixture);
        net_report.set_interfaces_status(SourceAvailability::Available);
        net_report.set_routes_status(SourceAvailability::Available);
        net_report.set_sockets_status(SourceAvailability::Available);
        net_report.set_namespace(Some(
            NetworkNamespace::new("host", Some(4026531992)).unwrap(),
        ));

        let iface = InterfaceRecord::new("eth0", InterfaceOperState::Up)
            .unwrap()
            .with_ifindex(2)
            .with_mtu(1500)
            .with_mac_address("00:15:5d:01:02:03")
            .unwrap()
            .with_flags(vec!["UP".into(), "BROADCAST".into(), "RUNNING".into()])
            .unwrap();
        net_report.push_interface(iface).unwrap();

        let route = RouteRecord::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0, "eth0")
            .unwrap()
            .with_gateway(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
            .unwrap();
        net_report.push_route(route).unwrap();

        let sock = SocketRecord::new(
            SocketProtocol::Tcp,
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            80,
            SocketState::Listen,
        )
        .with_inode(12345);
        net_report.push_socket(sock).unwrap();

        let mut snapshot = Snapshot::new(0, 0, SnapshotMode::Fixture);
        snapshot
            .push(
                MetricObservation::new(
                    "network.eth0.rx.bytes_per_second",
                    "proc.net.dev",
                    lidb_core::Unit::BytesPerSecond,
                    0,
                    MetricState::Available(MetricValue::Number(24_576.0)),
                )
                .unwrap(),
            )
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "network.eth0.tx.bytes_per_second",
                    "proc.net.dev",
                    lidb_core::Unit::BytesPerSecond,
                    0,
                    MetricState::Available(MetricValue::Number(1_048_576.0)),
                )
                .unwrap(),
            )
            .unwrap();

        let mut view = View {
            update: Some(Update {
                snapshot,
                network: Some(net_report),
                ebpf: None,
                collected_at: Instant::now(),
                skipped_updates: 0,
            }),
            tab: Tab::Network,
            fixture: true,
            ..View::default()
        };

        for width in [120, 85, 50] {
            let output = render(width, 30, &mut view);
            assert!(output.contains("Network Diagnostics Status"));
            assert!(output.contains("host"));
            assert!(output.contains("4026531992"));
            assert!(output.contains("eth0"));
            assert!(output.contains("12345"));
            assert!(output.contains("Sockets 1..1 of 1"));
            if width >= 85 {
                assert!(output.contains("24.0 KB/s"));
                assert!(output.contains("1.0 MB/s"));
            }
        }

        // Test small screen
        let small = render(40, 10, &mut view);
        assert!(small.contains("Network Overview"));
        assert!(small.contains("Ifaces: 1"));
        assert!(small.contains("Routes: 1"));
        assert!(small.contains("Sockets: 1"));
    }

    #[test]
    fn network_tab_scrolls_sockets_table() {
        use lidb_core::network::{
            NetworkReport, SocketProtocol, SocketRecord, SocketState, SourceAvailability,
        };
        use std::net::{IpAddr, Ipv4Addr};

        let mut net_report = NetworkReport::new(100, 20, SnapshotMode::Fixture);
        net_report.set_interfaces_status(SourceAvailability::Available);
        net_report.set_routes_status(SourceAvailability::Available);
        net_report.set_sockets_status(SourceAvailability::Available);

        for i in 0..25 {
            let sock = SocketRecord::new(
                SocketProtocol::Tcp,
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                1000 + i as u16,
                SocketState::Listen,
            )
            .with_inode(50000 + i as u64);
            net_report.push_socket(sock).unwrap();
        }

        let mut view = View {
            update: Some(Update {
                snapshot: Snapshot::new(0, 0, SnapshotMode::Fixture),
                network: Some(net_report),
                ebpf: None,
                collected_at: Instant::now(),
                skipped_updates: 0,
            }),
            tab: Tab::Network,
            fixture: true,
            scroll: 0,
            ..View::default()
        };

        let top = render(100, 30, &mut view);
        assert!(top.contains("Sockets 1.."));
        assert!(top.contains("of 25"));
        assert!(top.contains("50000"));

        // Scroll down to 10
        view.scroll = 10;
        let scrolled = render(100, 30, &mut view);
        assert!(scrolled.contains("Sockets 11.."));
        assert!(scrolled.contains("of 25"));
        assert!(scrolled.contains("50010"));
        assert!(!scrolled.contains("50000"));
    }

    #[test]
    fn network_and_ebpf_tabs_render_detail_modals() {
        use lidb_core::ebpf::{EbpfEventType, EbpfLifecycleEvent};
        use lidb_core::network::{
            NetworkNamespace, NetworkReport, SocketProtocol, SocketRecord, SocketState,
            SourceAvailability,
        };
        use std::net::{IpAddr, Ipv4Addr};

        // 1. Network detail
        let mut net_report = NetworkReport::new(100, 20, SnapshotMode::Fixture);
        net_report.set_sockets_status(SourceAvailability::Available);
        net_report.set_namespace(Some(
            NetworkNamespace::new("container1", Some(4026532500)).unwrap(),
        ));
        let iface = lidb_core::network::InterfaceRecord::new(
            "eth0",
            lidb_core::network::InterfaceOperState::Up,
        )
        .unwrap()
        .with_mtu(1500);
        net_report.push_interface(iface).unwrap();

        let sock = SocketRecord::new(
            SocketProtocol::Tcp,
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)),
            8080,
            SocketState::Established,
        )
        .with_remote(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 45678)
        .unwrap()
        .with_inode(98765);
        net_report.push_socket(sock).unwrap();

        let mut view = fixture();
        view.tab = Tab::Network;
        view.detail = true;
        view.update.as_mut().unwrap().network = Some(net_report);

        let net_detail = render(100, 25, &mut view);
        assert!(net_detail.contains("Socket detail | Enter returns"));
        assert!(net_detail.contains("Protocol: tcp"));
        assert!(net_detail.contains("Local Endpoint: 10.0.0.5:8080"));
        assert!(net_detail.contains("Remote Endpoint: 10.0.0.1:45678"));
        assert!(net_detail.contains("State: established"));
        assert!(net_detail.contains("Inode: 98765"));
        assert!(net_detail.contains("Network Namespace: container1 (inode: 4026532500)"));
        assert!(net_detail.contains("--- Network Interfaces Throughput & Errors ---"));
        assert!(net_detail.contains("eth0: state up"));

        // 2. eBPF detail
        let mut ebpf_report = EbpfReport::new(SnapshotMode::Fixture, 500);
        ebpf_report.set_status(SourceAvailability::Available);
        let event = EbpfLifecycleEvent::new(
            EbpfEventType::TcpConnect,
            999888777,
            5555,
            "nginx",
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
            80,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            52345,
        )
        .unwrap()
        .with_states(Some(SocketState::Listen), Some(SocketState::Established))
        .with_round_trip_us(Some(8500));
        ebpf_report.push_event(event);

        view.tab = Tab::Ebpf;
        view.update.as_mut().unwrap().ebpf = Some(ebpf_report);

        let ebpf_detail = render(100, 25, &mut view);
        assert!(ebpf_detail.contains("eBPF Event detail | Enter returns"));
        assert!(ebpf_detail.contains("Event: tcp_connect"));
        assert!(ebpf_detail.contains("Process: nginx [PID 5555]"));
        assert!(ebpf_detail.contains("Address Family: ipv4"));
        assert!(ebpf_detail.contains("Source Endpoint: 192.168.1.50:80"));
        assert!(ebpf_detail.contains("Destination Endpoint: 192.168.1.100:52345"));
        assert!(ebpf_detail.contains("State Transition: listen -> established"));
        assert!(ebpf_detail.contains("Round-Trip Latency: 8.5ms (8500 µs)"));
        assert!(ebpf_detail.contains("Tracepoint: sock:inet_sock_set_state"));
        assert!(ebpf_detail.contains("Timestamp: 999888777ns"));

        // 3. CpuMemory deep-dive detail
        view.tab = Tab::CpuMemory;
        let cpu_detail = render(100, 25, &mut view);
        assert!(cpu_detail.contains("CPU & Memory detail | Enter returns"));
        assert!(cpu_detail.contains("--- CPU Subsystem (/proc/stat) ---"));
        assert!(cpu_detail.contains("--- Memory Subsystem (/proc/meminfo) ---"));
        assert!(cpu_detail.contains("--- Swap Space (/proc/meminfo) ---"));
        assert!(cpu_detail
            .contains("--- System Activity & Pressure (/proc/loadavg & /proc/uptime) ---"));
    }

    #[test]
    fn format_uptime_duration_formats_correctly() {
        assert_eq!(format_uptime_duration(45.5), "45.5s");
        assert_eq!(format_uptime_duration(125.0), "2m 5s (125s)");
        assert_eq!(format_uptime_duration(3665.0), "1h 1m 5s (3665s)");
        assert_eq!(format_uptime_duration(187950.0), "2d 4h 12m 30s (187950s)");
        assert_eq!(format_uptime_duration(-5.0), "-");
        assert_eq!(format_uptime_duration(f64::NAN), "-");
    }

    #[test]
    fn cpu_memory_tab_renders_comprehensive_detail_modal() {
        let mut snapshot = Snapshot::new(123456789, 50, SnapshotMode::Fixture);
        snapshot
            .push(
                MetricObservation::new(
                    "cpu.busy.percent",
                    "proc.stat",
                    Unit::Percent,
                    0,
                    MetricState::Available(MetricValue::Number(25.5)),
                )
                .unwrap(),
            )
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "cpu.user.ticks",
                    "proc.stat",
                    Unit::Count,
                    0,
                    MetricState::Available(MetricValue::Integer(100000)),
                )
                .unwrap(),
            )
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "memory.total.bytes",
                    "proc.meminfo",
                    Unit::Bytes,
                    0,
                    MetricState::Available(MetricValue::Integer(17_179_869_184)),
                )
                .unwrap(),
            )
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "memory.used.bytes",
                    "proc.meminfo",
                    Unit::Bytes,
                    0,
                    MetricState::Available(MetricValue::Integer(8_589_934_592)),
                )
                .unwrap(),
            )
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "uptime.seconds",
                    "proc.uptime",
                    Unit::Seconds,
                    0,
                    MetricState::Available(MetricValue::Number(187950.0)),
                )
                .unwrap(),
            )
            .unwrap();

        let mut view = View {
            update: Some(Update {
                snapshot,
                network: None,
                ebpf: None,
                collected_at: Instant::now(),
                skipped_updates: 0,
            }),
            tab: Tab::CpuMemory,
            detail: true,
            ..View::default()
        };

        let detail = render(100, 30, &mut view);
        assert!(detail.contains("CPU & Memory detail | Enter returns"));
        assert!(detail.contains("CPU Busy: 25.50"));
        assert!(detail.contains("Ticks: user 100000"));
        assert!(detail.contains("Total: 16.00 GiB (17179869184 B)"));
        assert!(detail.contains("Used: 8.00 GiB (8589934592 B) [50%]"));
        assert!(detail.contains("System Uptime: 2d 4h 12m 30s (187950s)"));
        assert!(detail.contains("Collector Monotonic: 123456789ns"));
    }

    #[test]
    fn network_tab_renders_interface_detail_modal_when_sockets_empty() {
        use lidb_core::network::{
            InterfaceOperState, InterfaceRecord, NetworkNamespace, NetworkReport,
            SourceAvailability,
        };

        let mut net_report = NetworkReport::new(100, 20, SnapshotMode::Fixture);
        net_report.set_interfaces_status(SourceAvailability::Available);
        net_report.set_sockets_status(SourceAvailability::Available);
        net_report.set_namespace(Some(
            NetworkNamespace::new("container2", Some(4026532800)).unwrap(),
        ));

        let iface = InterfaceRecord::new("eth0", InterfaceOperState::Up)
            .unwrap()
            .with_ifindex(2)
            .with_mtu(9000)
            .with_mac_address("aa:bb:cc:dd:ee:ff")
            .unwrap()
            .with_speed_mbps(10000)
            .with_flags(vec!["UP".into(), "RUNNING".into()])
            .unwrap();
        net_report.push_interface(iface).unwrap();

        let mut snapshot = Snapshot::new(0, 0, SnapshotMode::Fixture);
        snapshot
            .push(
                MetricObservation::new(
                    "network.eth0.rx.bytes_per_second",
                    "proc.net.dev",
                    lidb_core::Unit::BytesPerSecond,
                    0,
                    MetricState::Available(MetricValue::Number(52_428_800.0)),
                )
                .unwrap(),
            )
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "network.eth0.tx.bytes_per_second",
                    "proc.net.dev",
                    lidb_core::Unit::BytesPerSecond,
                    0,
                    MetricState::Available(MetricValue::Number(104_857_600.0)),
                )
                .unwrap(),
            )
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "network.eth0.rx.bytes",
                    "proc.net.dev",
                    lidb_core::Unit::Bytes,
                    0,
                    MetricState::Available(MetricValue::Integer(100_000_000)),
                )
                .unwrap(),
            )
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "network.eth0.tx.bytes",
                    "proc.net.dev",
                    lidb_core::Unit::Bytes,
                    0,
                    MetricState::Available(MetricValue::Integer(200_000_000)),
                )
                .unwrap(),
            )
            .unwrap();

        let mut view = fixture();
        view.tab = Tab::Network;
        view.detail = true;
        view.update = Some(Update {
            snapshot,
            network: Some(net_report),
            ebpf: None,
            collected_at: Instant::now(),
            skipped_updates: 0,
        });

        let output = render(100, 25, &mut view);
        assert!(output.contains("Socket detail | Enter returns"));
        assert!(output.contains("Interface: eth0"));
        assert!(output.contains("State: up"));
        assert!(output.contains("MTU: 9000"));
        assert!(output.contains("MAC: aa:bb:cc:dd:ee:ff"));
        assert!(output.contains("Speed: 10000 Mbps"));
        assert!(output.contains("<UP,RUNNING>"));
        assert!(output.contains("RX Rate: 50.0 MB/s"));
        assert!(output.contains("TX Rate: 100.0 MB/s"));
        assert!(output.contains("total: 100000000 B"));
        assert!(output.contains("total: 200000000 B"));
        assert!(output.contains("Network Namespace: container2 (inode: 4026532800)"));
    }

    #[test]
    fn format_rate_value_formats_rates_cleanly() {
        assert_eq!(format_rate_value(0.0), "0 B/s");
        assert_eq!(format_rate_value(500.0), "500 B/s");
        assert_eq!(format_rate_value(1024.0), "1.0 KB/s");
        assert_eq!(format_rate_value(2048.0), "2.0 KB/s");
        assert_eq!(format_rate_value(1_048_576.0), "1.0 MB/s");
        assert_eq!(format_rate_value(1_572_864.0), "1.5 MB/s");
        assert_eq!(format_rate_value(1_073_741_824.0), "1.0 GB/s");
        assert_eq!(format_rate_value(-10.0), "-");
        assert_eq!(format_rate_value(f64::NAN), "-");
        assert_eq!(format_rate_value(f64::INFINITY), "-");
    }

    #[test]
    fn bounded_history_ring_caps_at_max_and_steps_correctly() {
        let mut view = View {
            fixture: true,
            ..View::default()
        };

        // Push 130 updates, each with monotonic_ns = i
        for i in 0u64..130 {
            let mut snapshot = Snapshot::new(i, 10, SnapshotMode::Fixture);
            snapshot
                .push(
                    MetricObservation::new(
                        "sample.id",
                        "procfs/stat",
                        Unit::Count,
                        i,
                        MetricState::Available(MetricValue::Integer(i)),
                    )
                    .unwrap(),
                )
                .unwrap();
            view.push_update(Update {
                snapshot,
                network: None,
                ebpf: None,
                collected_at: Instant::now(),
                skipped_updates: 0,
            });
        }

        // Must be capped at MAX_VIEW_HISTORY (120)
        assert_eq!(view.history.len(), MAX_VIEW_HISTORY);
        assert_eq!(view.update.as_ref().unwrap().snapshot.monotonic_ns(), 129);
        assert_eq!(view.history_cursor, None);
        assert!(!view.paused);

        // Step back once: should pause and show sample 128
        view.step_history_back();
        assert!(view.paused);
        assert_eq!(view.history_cursor, Some(1));
        assert_eq!(view.update.as_ref().unwrap().snapshot.monotonic_ns(), 128);

        // Render header and verify history indicator
        let output = render(120, 30, &mut view);
        assert!(output.contains("PAUSED [-1/120]"));

        // Step back 10 more times
        for _ in 0..10 {
            view.step_history_back();
        }
        assert_eq!(view.history_cursor, Some(11));
        assert_eq!(view.update.as_ref().unwrap().snapshot.monotonic_ns(), 118);

        // Step forward 5 times
        for _ in 0..5 {
            view.step_history_forward();
        }
        assert_eq!(view.history_cursor, Some(6));
        assert_eq!(view.update.as_ref().unwrap().snapshot.monotonic_ns(), 123);

        // Unpause: snap back to latest
        view.paused = false;
        view.history_cursor = None;
        view.update = view.history.back().cloned();
        assert_eq!(view.update.as_ref().unwrap().snapshot.monotonic_ns(), 129);
        assert!(!render(120, 30, &mut view).contains("PAUSED"));
    }

    #[test]
    fn tab_aware_scroll_by_bounds_against_active_tab_items() {
        use lidb_core::ebpf::{EbpfEventType, EbpfLifecycleEvent, EbpfReport};
        use lidb_core::network::{
            NetworkReport, SocketProtocol, SocketRecord, SocketState, SourceAvailability,
        };
        use std::net::{IpAddr, Ipv4Addr};

        let mut snapshot = Snapshot::new(100, 10, SnapshotMode::Fixture);
        for i in 0..3 {
            snapshot
                .push(
                    MetricObservation::new(
                        format!("metric.{i}"),
                        "proc.test",
                        Unit::Count,
                        0,
                        MetricState::Available(MetricValue::Integer(i as u64)),
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        let mut net_report = NetworkReport::new(100, 20, SnapshotMode::Fixture);
        net_report.set_sockets_status(SourceAvailability::Available);
        for i in 0..15 {
            let sock = SocketRecord::new(
                SocketProtocol::Tcp,
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                2000 + i as u16,
                SocketState::Listen,
            );
            net_report.push_socket(sock).unwrap();
        }

        let mut ebpf_report = EbpfReport::new(SnapshotMode::Fixture, 500);
        ebpf_report.set_status(SourceAvailability::Available);
        for i in 0..6 {
            let event = EbpfLifecycleEvent::new(
                EbpfEventType::TcpConnect,
                1000 + i as u64,
                100 + i as u32,
                "app",
                IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
                80,
                IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)),
                5000 + i as u16,
            )
            .unwrap();
            ebpf_report.push_event(event);
        }

        let mut view = View {
            update: Some(Update {
                snapshot,
                network: Some(net_report),
                ebpf: Some(ebpf_report),
                collected_at: Instant::now(),
                skipped_updates: 0,
            }),
            tab: Tab::Metrics,
            ..View::default()
        };

        // 1. Tab::Metrics has 3 metrics (max index = 2)
        view.scroll_by(10);
        assert_eq!(view.scroll, 2);
        view.scroll_by(-1);
        assert_eq!(view.scroll, 1);

        // 2. Tab::Network has 15 sockets (max index = 14)
        view.tab = Tab::Network;
        view.scroll = 0;
        view.scroll_by(8);
        assert_eq!(view.scroll, 8);
        view.scroll_by(20);
        assert_eq!(view.scroll, 14);

        // 3. Tab::Ebpf has 6 events (max index = 5)
        view.tab = Tab::Ebpf;
        view.scroll = 0;
        view.scroll_by(4);
        assert_eq!(view.scroll, 4);
        view.scroll_by(10);
        assert_eq!(view.scroll, 5);
    }

    #[test]
    fn network_tab_detail_modal_renders_complete_routes_and_diagnostics() {
        use lidb_core::network::{
            InterfaceRecord, NetworkNamespace, NetworkReport, RouteRecord, SocketProtocol,
            SocketRecord, SocketState, SourceAvailability,
        };
        use std::net::{IpAddr, Ipv4Addr};

        let mut net_report = NetworkReport::new(100, 20, SnapshotMode::Fixture);
        net_report.set_sockets_status(SourceAvailability::Available);
        net_report.set_interfaces_status(SourceAvailability::Available);
        net_report.set_routes_status(SourceAvailability::Available);
        net_report.set_namespace(Some(
            NetworkNamespace::new("corp-vpn", Some(4026533000)).unwrap(),
        ));

        let iface = InterfaceRecord::new("wg0", lidb_core::network::InterfaceOperState::Up)
            .unwrap()
            .with_mtu(1420);
        net_report.push_interface(iface).unwrap();

        let route1 = RouteRecord::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)), 8, "wg0")
            .unwrap()
            .with_gateway(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)))
            .unwrap()
            .with_metric(50);
        let route2 = RouteRecord::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0, "eth0").unwrap();
        net_report.push_route(route1).unwrap();
        net_report.push_route(route2).unwrap();

        let sock = SocketRecord::new(
            SocketProtocol::Udp,
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            51820,
            SocketState::Closed,
        );
        net_report.push_socket(sock).unwrap();

        let mut view = fixture();
        view.tab = Tab::Network;
        view.detail = true;
        view.update.as_mut().unwrap().network = Some(net_report);

        let output = render(110, 30, &mut view);
        assert!(output.contains("Socket detail | Enter returns"));
        assert!(output.contains("--- Selected Socket [1/1] ---"));
        assert!(output.contains("Protocol: udp"));
        assert!(output.contains("Local Endpoint: 0.0.0.0:51820"));
        assert!(output.contains("--- Network Interfaces Throughput & Errors ---"));
        assert!(output.contains("Interface: wg0"));
        assert!(output.contains("MTU: 1420"));
        assert!(output.contains("--- Active Routing Table (2 routes) ---"));
        assert!(output.contains("10.0.0.0/8 via 10.0.0.1 dev wg0 metric 50 (ipv4)"));
        assert!(output.contains("0.0.0.0/0 via direct / on-link dev eth0 metric - (ipv4)"));
        assert!(output.contains("--- Diagnostics Status & Namespace Attribution ---"));
        assert!(output.contains("Network Namespace: corp-vpn (inode: 4026533000)"));
    }

    #[test]
    fn detail_modal_text_scrolling_and_paging_behavior() {
        let mut view = fixture();
        view.detail = true;
        view.text_scroll = 0;
        view.text_page_size = 10;

        // Line-by-line scrolling
        view.text_scroll = view.text_scroll.saturating_add(1);
        assert_eq!(view.text_scroll, 1);
        view.text_scroll = view.text_scroll.saturating_add(1);
        assert_eq!(view.text_scroll, 2);
        view.text_scroll = view.text_scroll.saturating_sub(1);
        assert_eq!(view.text_scroll, 1);

        // Page scroll
        view.scroll_text_page(true);
        assert_eq!(view.text_scroll, 11);
        view.scroll_text_page(false);
        assert_eq!(view.text_scroll, 1);

        // Home and End text scrolling
        view.text_scroll = u16::MAX;
        assert_eq!(view.text_scroll, u16::MAX);
        view.text_scroll = 0;
        assert_eq!(view.text_scroll, 0);
    }
}
