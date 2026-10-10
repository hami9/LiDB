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
use lidb_core::{MetricObservation, MetricState, MetricValue, Snapshot};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table, Tabs, Wrap},
    Frame, Terminal,
};
use std::{
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
}

impl Tab {
    /// All available tabs in presentation order.
    pub const ALL: [Tab; 2] = [Tab::Metrics, Tab::CpuMemory];

    /// Cycles forward to the next tab.
    pub fn next(&self) -> Self {
        match self {
            Self::Metrics => Self::CpuMemory,
            Self::CpuMemory => Self::Metrics,
        }
    }

    /// Cycles backward to the previous tab.
    pub fn prev(&self) -> Self {
        match self {
            Self::Metrics => Self::CpuMemory,
            Self::CpuMemory => Self::Metrics,
        }
    }
}

/// In-memory interactive state for the terminal UI.
#[derive(Default)]
pub struct View {
    pub update: Option<Update>,
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
    /// Adjusts table scroll position by a signed delta bounded to available metrics.
    fn scroll_by(&mut self, count: isize) {
        let max = self.update.as_ref().map_or(0, |update| {
            update.snapshot.metrics().len().saturating_sub(1)
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
    let state = if view.paused { "PAUSED" } else { "running" };
    let freshness = view.update.as_ref().map_or_else(
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
    let header_block = Block::default()
        .title(format!(" LiDB {mode} | read-only host metrics "))
        .borders(Borders::ALL);
    let inner_header = header_block.inner(layout[0]);
    frame.render_widget(header_block, layout[0]);

    if inner_header.width >= 58 {
        let header_layout =
            Layout::horizontal([Constraint::Min(24), Constraint::Length(26)]).split(inner_header);
        frame.render_widget(
            Paragraph::new(format!("{state} | {freshness}")),
            header_layout[0],
        );
        let tabs = Tabs::new(vec!["1: Metrics", "2: CPU/Mem"])
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
        "q quit | PgUp/PgDn scroll text"
    } else {
        "q quit | Tab / 1-2 tab | Space pause | ? help | Enter detail | Up/Down scroll"
    };
    frame.render_widget(Paragraph::new(shortcuts), layout[2]);
    if view.help {
        draw_text(
            frame,
            "q, Esc, Ctrl-C: quit\nTab, 1, 2, Left/Right: switch between 1:Metrics and 2:CPU/Mem tabs\nSpace: pause displayed snapshot (sampling continues)\n?, h: toggle help\nEnter: show full details for the first visible metric (Metrics tab)\nUp/Down, j/k: scroll metrics (scroll text in help/details)\nPageUp/PageDown: scroll ten metrics or one page of text\nHome/End: first/last metric\n\nAge is time since collection; paused values grow older.\nUnavailable states retain their reason; no missing value becomes zero.\nSkipped counts snapshots replaced before the UI consumed them.\nDropped counts metrics omitted by the collector.\nRates require two valid counter samples.\nAll timestamps use this collector's monotonic clock.\nFixture readings are test input, not live host measurements.",
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
        draw_text(
            frame,
            &text,
            " Metric detail | Enter returns ",
            &mut view.text_scroll,
            layout[1],
        );
        return;
    }
    if view.tab == Tab::CpuMemory {
        draw_cpu_mem(frame, &update.snapshot, layout[1]);
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
    let uptime = metric_display(find_metric(snapshot, "uptime.seconds"));

    let psi_load_lines = if area.width >= 70 {
        vec![
            Line::from(format!(
                "PSI Stall avg10: CPU {psi_cpu}  │  Mem {psi_mem}  │  I/O {psi_io}"
            )),
            Line::from(format!(
                "Load Average: 1m: {load1}  5m: {load5}  15m: {load15}  │  Uptime: {uptime}s"
            )),
        ]
    } else if area.width >= 48 {
        vec![
            Line::from(format!(
                "PSI avg10: CPU {psi_cpu} | Mem {psi_mem} | IO {psi_io}"
            )),
            Line::from(format!("Load: {load1} {load5} {load15} │ Up: {uptime}s")),
        ]
    } else {
        vec![
            Line::from(format!("PSI: C:{psi_cpu} M:{psi_mem} IO:{psi_io}")),
            Line::from(format!("Load: {load1} {load5} {load15}")),
            Line::from(format!("Uptime: {uptime}s")),
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
            view.update = Some(update);
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
    // Restore before the default panic hook prints; the guard also covers unwinding/errors.
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous(info);
    }));
    let mut collector = crate::collector(&config);
    let mut worker = Worker::start(move || collector.sample(), config.interval)?;
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
        update_from_worker(&mut view, &worker)?;
        if worker.is_finished() {
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
                    KeyCode::Char(' ') => view.paused = !view.paused,
                    KeyCode::Char('?') | KeyCode::Char('h') => {
                        view.help = !view.help;
                        view.text_scroll = 0;
                    }
                    KeyCode::Enter => {
                        if view.detail || view.tab == Tab::Metrics {
                            view.detail = !view.detail;
                            view.text_scroll = 0;
                        }
                    }
                    KeyCode::Tab => {
                        view.tab = view.tab.next();
                    }
                    KeyCode::BackTab => {
                        view.tab = view.tab.prev();
                    }
                    KeyCode::Char('1') => {
                        view.tab = Tab::Metrics;
                    }
                    KeyCode::Char('2') => {
                        view.tab = Tab::CpuMemory;
                    }
                    KeyCode::Left if !view.help && !view.detail => {
                        view.tab = view.tab.prev();
                    }
                    KeyCode::Right if !view.help && !view.detail => {
                        view.tab = view.tab.next();
                    }
                    KeyCode::Down | KeyCode::Char('j') if view.help => {
                        view.text_scroll = view.text_scroll.saturating_add(1)
                    }
                    KeyCode::Up | KeyCode::Char('k') if view.help => {
                        view.text_scroll = view.text_scroll.saturating_sub(1)
                    }
                    KeyCode::Down | KeyCode::Char('j') => view.scroll_by(1),
                    KeyCode::Up | KeyCode::Char('k') => view.scroll_by(-1),
                    KeyCode::PageDown if view.help || view.detail => view.scroll_text_page(true),
                    KeyCode::PageUp if view.help || view.detail => view.scroll_text_page(false),
                    KeyCode::PageDown => view.scroll_by(10),
                    KeyCode::PageUp => view.scroll_by(-10),
                    KeyCode::Home => {
                        view.scroll = 0;
                        view.text_scroll = 0;
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
        assert_eq!(tab.next().next(), Tab::Metrics);
        assert_eq!(tab.prev(), Tab::CpuMemory);
        assert_eq!(tab.prev().prev(), Tab::Metrics);
    }
}
