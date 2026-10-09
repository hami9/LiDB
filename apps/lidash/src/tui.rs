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
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
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

#[derive(Default)]
pub struct View {
    pub update: Option<Update>,
    pub paused: bool,
    pub help: bool,
    pub detail: bool,
    pub scroll: usize,
    pub text_scroll: u16,
    pub fixture: bool,
}

impl View {
    fn scroll_by(&mut self, count: isize) {
        let max = self.update.as_ref().map_or(0, |update| {
            update.snapshot.metrics().len().saturating_sub(1)
        });
        self.scroll = self.scroll.saturating_add_signed(count).min(max);
        self.text_scroll = 0;
    }
}

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
    let header = Paragraph::new(format!("{state} | {freshness}")).block(
        Block::default()
            .title(format!(" LiDB {mode} | read-only host metrics "))
            .borders(Borders::ALL),
    );
    frame.render_widget(header, layout[0]);
    let shortcuts = if view.help || view.detail {
        "q quit | PgUp/PgDn scroll text"
    } else {
        "q quit | Space pause | ? help | Enter detail | arrows scroll"
    };
    frame.render_widget(Paragraph::new(shortcuts), layout[2]);
    if view.help {
        draw_text(frame,
            "q, Esc, Ctrl-C: quit\nSpace: pause displayed snapshot (sampling continues)\n?, h: toggle help\nEnter: show full details for the first visible metric\nUp/Down, j/k: select metrics (scroll text in help)\nPageUp/PageDown: scroll ten metrics or detail/help text\nHome/End: first/last metric\n\nAge is time since collection; paused values grow older.\nUnavailable states retain their reason; no missing value becomes zero.\nSkipped counts snapshots replaced before the UI consumed them.\nDropped counts metrics omitted by the collector.\nRates require two valid counter samples.\nAll timestamps use this collector's monotonic clock.\nFixture readings are test input, not live host measurements.",
            " Help ", &mut view.text_scroll, layout[1]);
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

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
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
        if let Some(update) = worker.take_latest()? {
            if !view.paused {
                view.update = Some(update);
            }
        }
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
                        view.detail = !view.detail;
                        view.text_scroll = 0;
                    }
                    KeyCode::Down | KeyCode::Char('j') if view.help => {
                        view.text_scroll = view.text_scroll.saturating_add(1)
                    }
                    KeyCode::Up | KeyCode::Char('k') if view.help => {
                        view.text_scroll = view.text_scroll.saturating_sub(1)
                    }
                    KeyCode::Down | KeyCode::Char('j') => view.scroll_by(1),
                    KeyCode::Up | KeyCode::Char('k') => view.scroll_by(-1),
                    KeyCode::PageDown if view.help || view.detail => {
                        view.text_scroll = view.text_scroll.saturating_add(10)
                    }
                    KeyCode::PageUp if view.help || view.detail => {
                        view.text_scroll = view.text_scroll.saturating_sub(10)
                    }
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
}
