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
    EbpfLifecycleEvent, EbpfReport, MetricObservation, MetricState, MetricValue, NetworkReport,
    Snapshot, SocketRecord, SourceAvailability,
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
    collections::{BTreeMap, VecDeque},
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

/// Maximum length (in Unicode scalar values) accepted for interactive search queries.
pub const MAX_FILTER_LEN: usize = 128;

/// Clamps a table scroll position safely into `0..len`.
fn clamp_scroll(scroll: usize, len: usize) -> usize {
    scroll.min(len.saturating_sub(1))
}

/// Returns true if `haystack` contains `needle` case-insensitively without allocating on ASCII inputs.
fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.is_ascii() && needle.is_ascii() {
        let needle_bytes = needle.as_bytes();
        return haystack
            .as_bytes()
            .windows(needle_bytes.len())
            .any(|window| window.eq_ignore_ascii_case(needle_bytes));
    }
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

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
    pub filter: String,
    pub filtering: bool,
}

/// Returns true if a metric observation matches the filter query string.
fn filter_matches_metric(metric: &MetricObservation<MetricValue>, needle: &str) -> bool {
    contains_case_insensitive(metric.name(), needle)
        || contains_case_insensitive(metric.source(), needle)
        || contains_case_insensitive(metric.unit().as_str(), needle)
        || contains_case_insensitive(&value_text(metric), needle)
}

/// Returns true if a socket record matches the filter query string.
fn filter_matches_socket(sock: &SocketRecord, needle: &str) -> bool {
    if contains_case_insensitive(sock.protocol().as_str(), needle)
        || contains_case_insensitive(sock.state().as_str(), needle)
        || contains_case_insensitive(sock.family().as_str(), needle)
    {
        return true;
    }
    let local = crate::output::format_socket_endpoint(sock.local_address(), sock.local_port());
    if contains_case_insensitive(&local, needle) {
        return true;
    }
    if let (Some(addr), Some(port)) = (sock.remote_address(), sock.remote_port()) {
        let remote = crate::output::format_socket_endpoint(addr, port);
        if contains_case_insensitive(&remote, needle) {
            return true;
        }
    }
    if let Some(inode) = sock.inode() {
        let mut buf = String::new();
        let _ = write!(buf, "{inode}");
        if contains_case_insensitive(&buf, needle) {
            return true;
        }
    }
    false
}

/// Returns true if an eBPF lifecycle event matches the filter query string.
fn filter_matches_ebpf(event: &EbpfLifecycleEvent, needle: &str) -> bool {
    if contains_case_insensitive(event.comm(), needle)
        || contains_case_insensitive(event.event_type().as_str(), needle)
        || contains_case_insensitive(event.family().as_str(), needle)
        || event
            .old_state()
            .is_some_and(|s| contains_case_insensitive(s.as_str(), needle))
        || event
            .new_state()
            .is_some_and(|s| contains_case_insensitive(s.as_str(), needle))
    {
        return true;
    }
    let mut buf = String::with_capacity(64);
    let _ = write!(
        buf,
        "{}\n{}:{}\n{}:{}",
        event.pid(),
        event.local_address(),
        event.local_port(),
        event.remote_address(),
        event.remote_port()
    );
    if contains_case_insensitive(&buf, needle) {
        return true;
    }
    if event.local_address().is_ipv6() || event.remote_address().is_ipv6() {
        buf.clear();
        let local =
            crate::output::format_socket_endpoint(event.local_address(), event.local_port());
        let remote =
            crate::output::format_socket_endpoint(event.remote_address(), event.remote_port());
        let _ = write!(buf, "{local}\n{remote}");
        return contains_case_insensitive(&buf, needle);
    }
    false
}

impl View {
    /// Returns the trimmed active filter query string, or `None` when empty or whitespace-only.
    #[must_use]
    pub fn filter_display(&self) -> Option<&str> {
        let trimmed = self.filter.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    }

    /// Returns true if an active non-empty search filter query is configured.
    #[must_use]
    pub fn is_filtered(&self) -> bool {
        self.filter_display().is_some()
    }

    /// Clears the active search filter and resets scroll offsets.
    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.filtering = false;
        self.scroll = 0;
        self.text_scroll = 0;
    }

    /// Returns all metrics matching the active filter query, or all metrics if unfiltered.
    pub fn filtered_metrics<'a>(
        &self,
        metrics: &'a [MetricObservation<MetricValue>],
    ) -> Vec<&'a MetricObservation<MetricValue>> {
        match self.filter_display() {
            Some(query) => metrics
                .iter()
                .filter(|m| filter_matches_metric(m, query))
                .collect(),
            None => metrics.iter().collect(),
        }
    }

    /// Returns all sockets matching the active filter query, or all sockets if unfiltered.
    pub fn filtered_sockets<'a>(&self, sockets: &'a [SocketRecord]) -> Vec<&'a SocketRecord> {
        match self.filter_display() {
            Some(query) => sockets
                .iter()
                .filter(|s| filter_matches_socket(s, query))
                .collect(),
            None => sockets.iter().collect(),
        }
    }

    /// Returns all eBPF events matching the active filter query, or all events if unfiltered.
    pub fn filtered_ebpf_events<'a>(
        &self,
        events: &'a [EbpfLifecycleEvent],
    ) -> Vec<&'a EbpfLifecycleEvent> {
        match self.filter_display() {
            Some(query) => events
                .iter()
                .filter(|e| filter_matches_ebpf(e, query))
                .collect(),
            None => events.iter().collect(),
        }
    }

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

    /// Counts matching items for the active tab without allocating a filtered vector.
    fn filtered_item_count(&self) -> usize {
        let query = self.filter_display();
        self.update.as_ref().map_or(0, |update| match self.tab {
            Tab::Metrics => match query {
                Some(q) => update
                    .snapshot
                    .metrics()
                    .iter()
                    .filter(|m| filter_matches_metric(m, q))
                    .count(),
                None => update.snapshot.metrics().len(),
            },
            Tab::CpuMemory => collect_disk_io_rows(&update.snapshot).len(),
            Tab::Network => update.network.as_ref().map_or(0, |n| match query {
                Some(q) => n
                    .sockets()
                    .iter()
                    .filter(|s| filter_matches_socket(s, q))
                    .count(),
                None => n.sockets().len(),
            }),
            Tab::Ebpf => update.ebpf.as_ref().map_or(0, |e| match query {
                Some(q) => e
                    .events()
                    .iter()
                    .filter(|ev| filter_matches_ebpf(ev, q))
                    .count(),
                None => e.events().len(),
            }),
        })
    }

    /// Adjusts table scroll position by a signed delta bounded to available items on the active tab.
    fn scroll_by(&mut self, count: isize) {
        let max = self.filtered_item_count().saturating_sub(1);
        self.scroll = self.scroll.min(max).saturating_add_signed(count).min(max);
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

    /// Processes a single terminal key event and returns `true` when the UI should quit.
    pub(crate) fn handle_key(&mut self, key: event::KeyEvent) -> bool {
        if key.kind == KeyEventKind::Release {
            return false;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }
        if self.filtering {
            match key.code {
                KeyCode::Esc => {
                    self.clear_filter();
                }
                KeyCode::Enter => {
                    self.filtering = false;
                    if self.filter.trim().is_empty() {
                        self.filter.clear();
                    }
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    self.scroll = 0;
                    self.text_scroll = 0;
                }
                KeyCode::Char(c)
                    if !c.is_control()
                        && !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                        && self.filter.chars().count() < MAX_FILTER_LEN =>
                {
                    self.filter.push(c);
                    self.scroll = 0;
                    self.text_scroll = 0;
                }
                _ => {}
            }
            return false;
        }
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Esc => {
                if self.is_filtered() {
                    self.clear_filter();
                } else {
                    return true;
                }
            }
            KeyCode::Char('/') if !self.help && self.tab != Tab::CpuMemory => {
                self.filtering = true;
                self.detail = false;
                self.text_scroll = 0;
            }
            KeyCode::Char('c') if self.is_filtered() => {
                self.clear_filter();
            }
            KeyCode::Char(' ') => {
                self.paused = !self.paused;
                if !self.paused {
                    self.history_cursor = None;
                    if let Some(latest) = self.history.back() {
                        self.update = Some(latest.clone());
                    }
                }
            }
            KeyCode::Char('[') | KeyCode::Char(',') => {
                self.step_history_back();
            }
            KeyCode::Char(']') | KeyCode::Char('.') => {
                self.step_history_forward();
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                self.help = !self.help;
                self.text_scroll = 0;
            }
            KeyCode::Enter => {
                self.detail = !self.detail;
                self.text_scroll = 0;
            }
            KeyCode::Tab => {
                self.tab = self.tab.next();
                self.scroll = 0;
                self.detail = false;
                self.text_scroll = 0;
            }
            KeyCode::BackTab => {
                self.tab = self.tab.prev();
                self.scroll = 0;
                self.detail = false;
                self.text_scroll = 0;
            }
            KeyCode::Char('1') => {
                self.tab = Tab::Metrics;
                self.scroll = 0;
                self.detail = false;
                self.text_scroll = 0;
            }
            KeyCode::Char('2') => {
                self.tab = Tab::CpuMemory;
                self.scroll = 0;
                self.detail = false;
                self.text_scroll = 0;
            }
            KeyCode::Char('3') => {
                self.tab = Tab::Network;
                self.scroll = 0;
                self.detail = false;
                self.text_scroll = 0;
            }
            KeyCode::Char('4') => {
                self.tab = Tab::Ebpf;
                self.scroll = 0;
                self.detail = false;
                self.text_scroll = 0;
            }
            KeyCode::Left if !self.help && !self.detail => {
                self.tab = self.tab.prev();
                self.scroll = 0;
                self.text_scroll = 0;
            }
            KeyCode::Right if !self.help && !self.detail => {
                self.tab = self.tab.next();
                self.scroll = 0;
                self.text_scroll = 0;
            }
            KeyCode::Down | KeyCode::Char('j') if self.help || self.detail => {
                self.text_scroll = self.text_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') if self.help || self.detail => {
                self.text_scroll = self.text_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => self.scroll_by(1),
            KeyCode::Up | KeyCode::Char('k') => self.scroll_by(-1),
            KeyCode::Char('n') if !(self.help || (self.detail && self.tab == Tab::CpuMemory)) => {
                self.scroll_by(1);
            }
            KeyCode::Char('p') if !(self.help || (self.detail && self.tab == Tab::CpuMemory)) => {
                self.scroll_by(-1);
            }
            KeyCode::PageDown if self.help || self.detail => self.scroll_text_page(true),
            KeyCode::PageUp if self.help || self.detail => self.scroll_text_page(false),
            KeyCode::PageDown => self.scroll_by(10),
            KeyCode::PageUp => self.scroll_by(-10),
            KeyCode::Home if self.help || self.detail => {
                self.text_scroll = 0;
            }
            KeyCode::Home => {
                self.scroll = 0;
                self.text_scroll = 0;
            }
            KeyCode::End if self.help || self.detail => {
                self.text_scroll = u16::MAX;
            }
            KeyCode::End => self.scroll_by(isize::MAX),
            _ => {}
        }
        false
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
    let filter_label = view.filter_display();
    let header_title = match filter_label {
        Some(q) if view.tab == Tab::CpuMemory => {
            format!(" LiDB {mode} | filter: \"{q}\" (Tabs 1/3/4) | read-only host metrics ")
        }
        Some(q) => format!(" LiDB {mode} | filter: \"{q}\" | read-only host metrics "),
        None => format!(" LiDB {mode} | read-only host metrics "),
    };
    let header_block = Block::default().title(header_title).borders(Borders::ALL);
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
    if view.filtering {
        let prompt = format!(
            "/{}█  [Enter: apply | Esc: cancel | Backspace: erase]",
            view.filter
        );
        frame.render_widget(
            Paragraph::new(prompt).style(Style::default().add_modifier(Modifier::BOLD)),
            layout[2],
        );
    } else if let Some(q) = filter_label {
        let shortcuts = if view.help {
            format!("[Filter: \"{q}\" (c to clear)] | q quit | Up/Dn / PgUp/PgDn scroll text")
        } else if view.detail {
            if view.tab == Tab::CpuMemory {
                format!(
                    "[Filter: \"{q}\" (c to clear)] | q quit | Up/Dn / PgUp/PgDn scroll text | Enter returns"
                )
            } else {
                format!(
                    "[Filter: \"{q}\" (c to clear)] | q quit | Up/Dn / PgUp/PgDn scroll text | n/p item"
                )
            }
        } else if view.tab == Tab::CpuMemory {
            format!(
                "[Filter: \"{q}\" (c to clear)] | q quit | Tab / 1-4 tab | ? help | Enter detail | Up/Down scroll"
            )
        } else {
            format!(
                "[Filter: \"{q}\" (c to clear)] | q quit | / search | Tab / 1-4 tab | ? help | Enter detail | Up/Down scroll"
            )
        };
        frame.render_widget(Paragraph::new(shortcuts), layout[2]);
    } else {
        let shortcuts = if view.help {
            "q quit | Up/Dn / PgUp/PgDn scroll text"
        } else if view.detail {
            if view.tab == Tab::CpuMemory {
                "q quit | Up/Dn / PgUp/PgDn scroll text | Enter returns"
            } else {
                "q quit | Up/Dn / PgUp/PgDn scroll text | n/p item"
            }
        } else if view.tab == Tab::CpuMemory {
            "q quit | Tab / 1-4 tab | Space pause | [ / ] history | ? help | Enter detail | Up/Down scroll"
        } else {
            "q quit | / search | Tab / 1-4 tab | Space pause | [ / ] history | ? help | Enter detail | Up/Down scroll"
        };
        frame.render_widget(Paragraph::new(shortcuts), layout[2]);
    }
    if view.help {
        const HELP_TEXT: &str = "q, Esc, Ctrl-C: quit\nTab, 1, 2, 3, 4, Left/Right: switch between 1:Metrics, 2:CPU/Mem, 3:Network, and 4:eBPF tabs\n/: interactive substring search filter across Metrics, Sockets, and eBPF events\nc: clear active search filter (or Esc when not in search input mode)\nSpace: pause displayed snapshot (sampling continues)\n[, ]: step backward/forward in history (auto-pauses; Space resumes live)\n?, h: toggle help\nEnter: toggle full details for active item (or confirm search query in filter mode)\nUp/Down, j/k: scroll items (scroll text in help/details)\nn, p: next/previous item (works in table and detail views)\nPageUp/PageDown: scroll ten items or one page of text\nHome/End: first/last item or start/end of text\n\nSearch queries match case-insensitively across metric names, sources, units, endpoints, family, and states.\nAge is time since collection; paused values grow older.\nUnavailable states retain their reason; no missing value becomes zero.\nSkipped counts snapshots replaced before the UI consumed them.\nDropped counts metrics omitted by the collector.\nRates require two valid counter samples.\nAll timestamps use this collector's monotonic clock.\nFixture readings are test input, not live host measurements.";
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
    let filtered_metrics = view.filtered_metrics(metrics);
    let total_metrics = metrics.len();
    let matched_metrics = filtered_metrics.len();
    let start = clamp_scroll(view.scroll, matched_metrics);
    if view.detail {
        let (text, title) = match view.tab {
            Tab::Metrics => {
                let text = filtered_metrics.get(start).map_or_else(
                    || match filter_label {
                        Some(q) => format!("No metrics match filter \"{q}\""),
                        None => "No metrics collected".into(),
                    },
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
                let title = match filter_label {
                    Some(q) => format!(" Metric detail [Filter: \"{q}\"] | Enter returns "),
                    None => " Metric detail | Enter returns ".to_string(),
                };
                (text, title)
            }
            Tab::Network => {
                let title = match filter_label {
                    Some(q) => format!(" Socket detail [Filter: \"{q}\"] | Enter returns "),
                    None => " Socket detail | Enter returns ".to_string(),
                };
                (
                    format_network_detail(
                        update.network.as_ref(),
                        &update.snapshot,
                        view.scroll,
                        filter_label,
                    ),
                    title,
                )
            }
            Tab::Ebpf => {
                let events = update.ebpf.as_ref().map(|e| e.events()).unwrap_or(&[]);
                let filtered_events = view.filtered_ebpf_events(events);
                let ebpf_start = clamp_scroll(view.scroll, filtered_events.len());
                let text = filtered_events.get(ebpf_start).map_or_else(
                    || match filter_label {
                        Some(q) => format!("No eBPF events match filter \"{q}\""),
                        None => "No eBPF events observed".into(),
                    },
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
                let title = match filter_label {
                    Some(q) => format!(" eBPF Event detail [Filter: \"{q}\"] | Enter returns "),
                    None => " eBPF Event detail | Enter returns ".to_string(),
                };
                (text, title)
            }
            Tab::CpuMemory => (
                format_cpu_memory_detail(&update.snapshot),
                " CPU & Memory detail | Enter returns ".to_string(),
            ),
        };
        draw_text(frame, &text, &title, &mut view.text_scroll, layout[1]);
        return;
    }
    if view.tab == Tab::CpuMemory {
        draw_cpu_mem(frame, &update.snapshot, view.scroll, layout[1]);
        return;
    }
    if view.tab == Tab::Network {
        draw_network(
            frame,
            update.network.as_ref(),
            Some(&update.snapshot),
            view.scroll,
            filter_label,
            layout[1],
        );
        return;
    }
    if view.tab == Tab::Ebpf {
        draw_ebpf(
            frame,
            update.ebpf.as_ref(),
            view.scroll,
            filter_label,
            layout[1],
        );
        return;
    }
    if matched_metrics == 0 {
        if let Some(q) = filter_label {
            let title = format!(
                " Metrics (0 of {total_metrics}) [Filter: \"{q}\"] | {}ns ",
                update.snapshot.monotonic_ns()
            );
            frame.render_widget(
                Paragraph::new(format!(
                    "No metrics match filter \"{q}\" ({total_metrics} total metrics)."
                ))
                .block(Block::default().title(title).borders(Borders::ALL)),
                layout[1],
            );
            return;
        }
    }
    if area.width >= 90 {
        let rows = filtered_metrics.iter().skip(start).map(|metric| {
            Row::new(vec![
                metric.name().to_owned(),
                value_text(metric),
                metric.unit().as_str().to_owned(),
                metric.source().to_owned(),
            ])
        });
        let title = match filter_label {
            Some(q) => format!(
                " Metrics {}..{} of {} [Filter: \"{q}\" ({matched_metrics}/{total_metrics} matched)] | {}ns ",
                start.saturating_add(1),
                (start + layout[1].height.saturating_sub(3) as usize).min(matched_metrics),
                matched_metrics,
                update.snapshot.monotonic_ns()
            ),
            None => format!(
                " Metrics {}..{} of {} | {}ns ",
                if total_metrics == 0 { 0 } else { start.saturating_add(1) },
                (start + layout[1].height.saturating_sub(3) as usize).min(total_metrics),
                total_metrics,
                update.snapshot.monotonic_ns()
            ),
        };
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
        .block(Block::default().title(title).borders(Borders::ALL));
        frame.render_widget(table, layout[1]);
    } else {
        let mut lines = Vec::new();
        for metric in filtered_metrics
            .iter()
            .skip(start)
            .take(layout[1].height as usize)
        {
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
        let title = match filter_label {
            Some(q) => format!(
                " Metrics [Filter: \"{q}\" ({matched_metrics}/{total_metrics})] | {}ns ",
                update.snapshot.monotonic_ns()
            ),
            None => format!(" Metrics | {}ns ", update.snapshot.monotonic_ns()),
        };
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .block(Block::default().title(title).borders(Borders::ALL)),
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

/// Formats a non-negative finite byte quantity or rate with binary (1024) scaling.
fn format_binary_scaled(value: f64, units: &[&str; 5]) -> String {
    if !value.is_finite() || value < 0.0 {
        return "-".to_owned();
    }
    let value = if value == 0.0 { 0.0 } else { value };
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    const TIB: f64 = GIB * 1024.0;
    if value < KIB {
        format!("{value:.0} {}", units[0])
    } else if value < MIB {
        format!("{:.1} {}", value / KIB, units[1])
    } else if value < GIB {
        format!("{:.1} {}", value / MIB, units[2])
    } else if value < TIB {
        format!("{:.1} {}", value / GIB, units[3])
    } else {
        format!("{:.1} {}", value / TIB, units[4])
    }
}

/// Formats a byte rate (bytes per second) into a human-readable throughput string.
fn format_rate_value(value: f64) -> String {
    format_binary_scaled(value, &["B/s", "KB/s", "MB/s", "GB/s", "TB/s"])
}

/// Formats a cumulative byte count into a human-readable IEC quantity string.
fn format_bytes_value(value: f64) -> String {
    format_binary_scaled(value, &["B", "KiB", "MiB", "GiB", "TiB"])
}

/// Returns the available numeric value of a metric observation, if present.
fn metric_as_f64(metric: &MetricObservation<MetricValue>) -> Option<f64> {
    match metric.state().available_value() {
        Some(MetricValue::Number(num)) => Some(*num),
        Some(MetricValue::Integer(int)) => Some(*int as f64),
        None => None,
    }
}

/// Extracts and formats a metric value as a transfer rate if available.
fn format_metric_rate(metric: &MetricObservation<MetricValue>) -> String {
    metric_as_f64(metric).map_or_else(|| "-".to_owned(), format_rate_value)
}

/// Extracts and formats a metric value as a binary byte total if available.
fn format_metric_bytes(metric: &MetricObservation<MetricValue>) -> String {
    metric_as_f64(metric).map_or_else(|| "-".to_owned(), format_bytes_value)
}

/// Grouped `/proc/diskstats` observations for a single block device.
#[derive(Default)]
struct DiskIoRow<'a> {
    device: &'a str,
    read_rate: Option<&'a MetricObservation<MetricValue>>,
    write_rate: Option<&'a MetricObservation<MetricValue>>,
    read_bytes: Option<&'a MetricObservation<MetricValue>>,
    write_bytes: Option<&'a MetricObservation<MetricValue>>,
}

/// Collects per-device Disk I/O metric groups sorted deterministically by device name.
fn collect_disk_io_rows(snapshot: &Snapshot) -> Vec<DiskIoRow<'_>> {
    enum DiskField {
        ReadRate,
        WriteRate,
        ReadBytes,
        WriteBytes,
    }

    const SUFFIXES: [(&str, DiskField); 4] = [
        (".read.bytes_per_second", DiskField::ReadRate),
        (".write.bytes_per_second", DiskField::WriteRate),
        (".read.bytes", DiskField::ReadBytes),
        (".write.bytes", DiskField::WriteBytes),
    ];

    let mut rows = BTreeMap::<&str, DiskIoRow<'_>>::new();
    for metric in snapshot.metrics() {
        let Some(rest) = metric.name().strip_prefix("disk.") else {
            continue;
        };
        for (suffix, field) in &SUFFIXES {
            if let Some(dev) = rest.strip_suffix(suffix).filter(|d| !d.is_empty()) {
                let row = rows.entry(dev).or_insert_with(|| DiskIoRow {
                    device: dev,
                    ..DiskIoRow::default()
                });
                match field {
                    DiskField::ReadRate => row.read_rate = Some(metric),
                    DiskField::WriteRate => row.write_rate = Some(metric),
                    DiskField::ReadBytes => row.read_bytes = Some(metric),
                    DiskField::WriteBytes => row.write_bytes = Some(metric),
                }
                break;
            }
        }
    }
    rows.into_values().collect()
}

/// Formats a rate metric for detail view, preserving typed unavailability reasons.
fn format_disk_rate_detail(metric: Option<&MetricObservation<MetricValue>>) -> String {
    match metric {
        Some(obs) if obs.state().available_value().is_some() => format_metric_rate(obs),
        Some(obs) => value_text(obs),
        None => "not probed".to_owned(),
    }
}

/// Formats a byte counter metric for detail view, preserving exact bytes and typed reasons.
fn format_disk_bytes_detail(metric: Option<&MetricObservation<MetricValue>>) -> String {
    match metric {
        Some(obs) if obs.state().available_value().is_some() => {
            format!("{} ({} B)", format_metric_bytes(obs), value_text(obs))
        }
        Some(obs) => value_text(obs),
        None => "not probed".to_owned(),
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
    let disk_rows = collect_disk_io_rows(snapshot);
    let disk_section = if disk_rows.is_empty() {
        format!(
            "\n\n--- Block Devices / Disk I/O (/proc/diskstats) ---\n{}",
            format_disk_io_unavailable_message(snapshot)
        )
    } else {
        let disk_source_status = match find_metric(snapshot, "disk.state") {
            Some(obs) => match obs.state().available_value() {
                Some(MetricValue::Integer(n)) => {
                    let noun = device_noun(usize::try_from(*n).unwrap_or(2));
                    format!("available ({n} {noun})")
                }
                Some(MetricValue::Number(n)) => format!("available ({n} devices)"),
                None => value_text(obs),
            },
            None => "not probed".to_owned(),
        };
        let mut section = format!(
            "\n\n--- Block Devices / Disk I/O (/proc/diskstats) ---\nSource Status: {disk_source_status}"
        );
        for row in &disk_rows {
            let read_rate = format_disk_rate_detail(row.read_rate);
            let write_rate = format_disk_rate_detail(row.write_rate);
            let read_total = format_disk_bytes_detail(row.read_bytes);
            let write_total = format_disk_bytes_detail(row.write_bytes);
            let _ = write!(
                section,
                "\n\nDevice: {}\nRead Rate: {read_rate} (total: {read_total})\nWrite Rate: {write_rate} (total: {write_total})",
                row.device
            );
        }
        section
    };

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
         Collector Monotonic: {}ns{disk_section}",
        snapshot.monotonic_ns()
    )
}

/// Formats a comprehensive deep-dive network subsystem summary for the Network tab.
fn format_network_detail(
    report: Option<&lidb_core::network::NetworkReport>,
    snapshot: &Snapshot,
    selected_socket_idx: usize,
    filter_query: Option<&str>,
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

    let filtered_sockets: Vec<&SocketRecord> = match filter_query {
        Some(q) => sockets
            .iter()
            .filter(|s| filter_matches_socket(s, q))
            .collect(),
        None => sockets.iter().collect(),
    };

    // Section 1: Selected Socket Inspection
    let sock_start = clamp_scroll(selected_socket_idx, filtered_sockets.len());

    if let Some(sock) = filtered_sockets.get(sock_start) {
        let local = crate::output::format_socket_endpoint(sock.local_address(), sock.local_port());
        let remote = match (sock.remote_address(), sock.remote_port()) {
            (Some(addr), Some(port)) => crate::output::format_socket_endpoint(addr, port),
            _ => "*".to_owned(),
        };
        let ino = sock.inode().map_or("-".into(), |i| i.to_string());
        let filter_tag = match filter_query {
            Some(q) => format!(" [Filter: \"{q}\"]"),
            None => String::new(),
        };
        let _ = write!(
            detail,
            "--- Selected Socket [{}/{}]{filter_tag} ---\nProtocol: {}\nLocal Endpoint: {}\nRemote Endpoint: {}\nState: {}\nInode: {}\nNetwork Namespace: {}",
            sock_start + 1,
            filtered_sockets.len(),
            sock.protocol().as_str(),
            local,
            remote,
            sock.state().as_str(),
            ino,
            ns_desc
        );
    } else if let Some(q) = filter_query {
        let _ = write!(
            detail,
            "--- Selected Socket ---\nNo network sockets match filter \"{q}\" ({} total sockets)",
            sockets.len()
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

/// Returns singular/plural noun for a block-device count.
const fn device_noun(count: usize) -> &'static str {
    if count == 1 {
        "device"
    } else {
        "devices"
    }
}

/// Consolidated rate-calculation status across observed block devices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiskRateStatus {
    Available,
    AwaitingBaseline,
    Unavailable(&'static str),
}

/// Returns an explicit availability/empty explanation when no Disk I/O rows exist.
fn format_disk_io_unavailable_message(snapshot: &Snapshot) -> String {
    match find_metric(snapshot, "disk.state") {
        Some(obs) => match obs.state().available_value() {
            Some(MetricValue::Integer(0)) => {
                "No block devices observed in /proc/diskstats.".to_owned()
            }
            Some(_) => "Block-device entries omitted or absent from this snapshot.".to_owned(),
            None => format!(
                "Disk I/O {}: {}",
                obs.state().name(),
                obs.state().reason().unwrap_or("no reason reported")
            ),
        },
        None => "Block-device I/O telemetry not probed in this snapshot.".to_owned(),
    }
}

/// Classifies aggregate rate availability across block-device rows.
fn classify_disk_rate_status(disks: &[DiskIoRow<'_>]) -> DiskRateStatus {
    let any_rate_available = disks.iter().any(|row| {
        row.read_rate
            .and_then(metric_as_f64)
            .or_else(|| row.write_rate.and_then(metric_as_f64))
            .is_some()
    });
    if any_rate_available {
        return DiskRateStatus::Available;
    }
    let mut awaiting = false;
    let mut error_label: Option<&'static str> = None;
    for obs in disks
        .iter()
        .flat_map(|row| [row.read_rate, row.write_rate])
        .flatten()
    {
        match obs.state() {
            MetricState::TemporarilyUnavailable(_) => awaiting = true,
            MetricState::Error(_) => error_label = Some("rate error"),
            MetricState::PermissionDenied(_) if error_label.is_none() => {
                error_label = Some("rate denied");
            }
            MetricState::Unsupported(_) | MetricState::Disabled(_) | MetricState::Stale(_)
                if error_label.is_none() =>
            {
                error_label = Some("rate unavailable");
            }
            _ => {}
        }
    }
    if awaiting {
        DiskRateStatus::AwaitingBaseline
    } else if let Some(label) = error_label {
        DiskRateStatus::Unavailable(label)
    } else {
        DiskRateStatus::Available
    }
}

/// Formats a single-line Disk I/O summary for medium-height CPU & Memory layouts.
fn format_disk_io_compact_line(snapshot: &Snapshot, disks: &[DiskIoRow<'_>]) -> String {
    if disks.is_empty() {
        return format_disk_io_unavailable_message(snapshot);
    }
    let noun = device_noun(disks.len());
    match classify_disk_rate_status(disks) {
        DiskRateStatus::AwaitingBaseline => {
            return format!("Disk I/O: {} {noun} (awaiting rate baseline)", disks.len());
        }
        DiskRateStatus::Unavailable(label) => {
            return format!("Disk I/O: {} {noun} ({label})", disks.len());
        }
        DiskRateStatus::Available => {}
    }
    let lead = disks
        .iter()
        .find(|row| {
            row.read_rate
                .and_then(metric_as_f64)
                .or_else(|| row.write_rate.and_then(metric_as_f64))
                .is_some()
        })
        .unwrap_or(&disks[0]);
    let read_rate = lead
        .read_rate
        .map_or_else(|| "-".into(), format_metric_rate);
    let write_rate = lead
        .write_rate
        .map_or_else(|| "-".into(), format_metric_rate);
    format!(
        "Disk I/O ({} {noun}): {} R {read_rate} / W {write_rate}",
        disks.len(),
        lead.device
    )
}

/// Renders the scrollable Block Devices / Disk I/O panel inside Tab 2 (`CPU/Mem`).
fn draw_disk_io_panel(
    frame: &mut Frame<'_>,
    snapshot: &Snapshot,
    disks: &[DiskIoRow<'_>],
    scroll: usize,
    area: Rect,
) {
    if disks.is_empty() {
        let msg = format_disk_io_unavailable_message(snapshot);
        frame.render_widget(
            Paragraph::new(msg).wrap(Wrap { trim: true }).block(
                Block::default()
                    .title(" Disk I/O (/proc/diskstats) ")
                    .borders(Borders::ALL),
            ),
            area,
        );
        return;
    }

    let visible_cap = area.height.saturating_sub(3).max(1) as usize;
    let total = disks.len();
    let start = clamp_scroll(scroll, total);
    let end = start.saturating_add(visible_cap).min(total);
    let (rate_tag, short_tag) = match classify_disk_rate_status(disks) {
        DiskRateStatus::Available => ("", ""),
        DiskRateStatus::AwaitingBaseline => (" [awaiting rate baseline]", " [baseline]"),
        DiskRateStatus::Unavailable("rate error") => (" [rate error]", " [rate err]"),
        DiskRateStatus::Unavailable("rate denied") => (" [rate denied]", " [denied]"),
        DiskRateStatus::Unavailable(_) => (" [rate unavailable]", " [no rate]"),
    };
    let full_title = if total > visible_cap || start > 0 {
        format!(
            " Disk I/O {}..{} of {} (/proc/diskstats){rate_tag} ",
            start + 1,
            end,
            total
        )
    } else {
        let noun = device_noun(total);
        format!(" Disk I/O ({total} {noun}, /proc/diskstats){rate_tag} ")
    };
    let title = if full_title.len() <= area.width as usize {
        full_title
    } else if total > visible_cap || start > 0 {
        format!(" Disk I/O {}..{} of {}{short_tag} ", start + 1, end, total)
    } else {
        let noun = device_noun(total);
        format!(" Disk I/O ({total} {noun}){short_tag} ")
    };

    let (headers, widths, table_rows): (Vec<&str>, Vec<Constraint>, Vec<Row>) = if area.width >= 70
    {
        (
            vec![
                "Device",
                "Read Rate",
                "Write Rate",
                "Total Read",
                "Total Write",
            ],
            vec![
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ],
            disks
                .iter()
                .skip(start)
                .take(visible_cap)
                .map(|disk| {
                    Row::new(vec![
                        disk.device.to_string(),
                        disk.read_rate
                            .map_or_else(|| "-".to_owned(), format_metric_rate),
                        disk.write_rate
                            .map_or_else(|| "-".to_owned(), format_metric_rate),
                        disk.read_bytes
                            .map_or_else(|| "-".to_owned(), format_metric_bytes),
                        disk.write_bytes
                            .map_or_else(|| "-".to_owned(), format_metric_bytes),
                    ])
                })
                .collect(),
        )
    } else {
        let headers = if area.width >= 48 {
            vec!["Device", "Read/s", "Write/s"]
        } else {
            vec!["Dev", "R/s", "W/s"]
        };
        (
            headers,
            vec![
                Constraint::Percentage(34),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ],
            disks
                .iter()
                .skip(start)
                .take(visible_cap)
                .map(|disk| {
                    Row::new(vec![
                        disk.device.to_string(),
                        disk.read_rate
                            .map_or_else(|| "-".to_owned(), format_metric_rate),
                        disk.write_rate
                            .map_or_else(|| "-".to_owned(), format_metric_rate),
                    ])
                })
                .collect(),
        )
    };

    let table = Table::new(table_rows, widths)
        .header(Row::new(headers).style(Style::default().add_modifier(Modifier::BOLD)))
        .block(Block::default().title(title).borders(Borders::ALL));
    frame.render_widget(table, area);
}

/// Renders the dedicated CPU and Memory overview dashboard tab.
fn draw_cpu_mem(frame: &mut Frame<'_>, snapshot: &Snapshot, scroll: usize, area: Rect) {
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

    let disk_rows = collect_disk_io_rows(snapshot);

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

    let mut psi_load_lines = if area.width >= 70 {
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

    let inner_width = area.width.saturating_sub(2);
    let psi_base_lines = Paragraph::new(psi_load_lines.clone())
        .wrap(Wrap { trim: true })
        .line_count(inner_width) as u16;
    let psi_height = psi_base_lines.saturating_add(2);
    // Reserve 10 fixed rows (CPU 3, Mem/Swap 3, CPU Ticks 4) + psi_height + min 5 for Disk I/O.
    let show_disk_panel = area.height >= 10_u16.saturating_add(psi_height).saturating_add(5);
    if !show_disk_panel {
        let compact = Line::from(format_disk_io_compact_line(snapshot, &disk_rows));
        let compact_lines = Paragraph::new(compact.clone())
            .wrap(Wrap { trim: true })
            .line_count(inner_width) as u16;
        if area.height.saturating_sub(12) >= psi_base_lines.saturating_add(compact_lines) {
            psi_load_lines.push(compact);
        }
    }

    let constraints: Vec<Constraint> = if show_disk_panel {
        vec![
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(psi_height),
            Constraint::Min(5),
        ]
    } else {
        vec![
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Min(4),
        ]
    };
    let rows = Layout::vertical(constraints).split(area);

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

    let psi_load_paragraph = Paragraph::new(psi_load_lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" PSI & Load (/proc/pressure & /proc/loadavg) ")
                .borders(Borders::ALL),
        );
    frame.render_widget(psi_load_paragraph, rows[3]);

    if show_disk_panel {
        draw_disk_io_panel(frame, snapshot, &disk_rows, scroll, rows[4]);
    }
}

/// Renders the dedicated Network overview dashboard tab.
fn draw_network(
    frame: &mut Frame<'_>,
    network: Option<&lidb_core::network::NetworkReport>,
    snapshot: Option<&Snapshot>,
    scroll: usize,
    filter_query: Option<&str>,
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

    let sockets = report.sockets();
    let filtered_sockets: Vec<&SocketRecord> = match filter_query {
        Some(q) => sockets
            .iter()
            .filter(|s| filter_matches_socket(s, q))
            .collect(),
        None => sockets.iter().collect(),
    };
    let socket_count = filtered_sockets.len();
    let total_socket_count = sockets.len();

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
        let sock_desc = match filter_query {
            Some(q) => format!(
                "{socket_count}/{total_socket_count} matching \"{q}\" ({})",
                report.sockets_status().as_str()
            ),
            None => format!(
                "{total_socket_count} ({})",
                report.sockets_status().as_str()
            ),
        };
        let text = format!(
            "Ifaces: {} ({}) │ Routes: {} ({})\nSockets: {sock_desc}\n{ns_desc}",
            report.interfaces().len(),
            report.interfaces_status().as_str(),
            report.routes().len(),
            report.routes_status().as_str(),
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

    let sock_inner_height = rows[2].height.saturating_sub(2) as usize;
    let start = clamp_scroll(scroll, socket_count);
    let end = (start + sock_inner_height).min(socket_count);
    let title = match filter_query {
        Some(q) => {
            if socket_count == 0 {
                format!(" Sockets (0 of {total_socket_count}) [Filter: \"{q}\"] ")
            } else {
                format!(
                    " Sockets {}..{} of {} [Filter: \"{q}\" ({socket_count}/{total_socket_count} matched)] ",
                    start + 1,
                    end,
                    socket_count
                )
            }
        }
        None => {
            if socket_count == 0 {
                " Sockets (0) ".to_string()
            } else {
                format!(" Sockets {}..{} of {} ", start + 1, end, socket_count)
            }
        }
    };

    let socket_block = Block::default().title(title).borders(Borders::ALL);

    if socket_count == 0 {
        if let Some(q) = filter_query {
            frame.render_widget(
                Paragraph::new(format!(
                    "No network sockets match filter \"{q}\" ({total_socket_count} total sockets)."
                ))
                .block(socket_block),
                rows[2],
            );
            return;
        }
    }

    let sock_rows: Vec<Row> = filtered_sockets
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
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(["Proto", "Local", "Remote", "State", "Inode"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(socket_block);

    frame.render_widget(sock_table, rows[2]);
}

fn draw_ebpf(
    frame: &mut Frame<'_>,
    ebpf: Option<&EbpfReport>,
    scroll: usize,
    filter_query: Option<&str>,
    area: Rect,
) {
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

    let events = report.events();
    let filtered_events: Vec<&EbpfLifecycleEvent> = match filter_query {
        Some(q) => events
            .iter()
            .filter(|e| filter_matches_ebpf(e, q))
            .collect(),
        None => events.iter().collect(),
    };
    let event_count = filtered_events.len();
    let total_event_count = events.len();
    let events_label = match filter_query {
        Some(q) => format!("{event_count}/{total_event_count} matching \"{q}\""),
        None => format!("{total_event_count}"),
    };

    let summary_text = format!(
        "Tracepoint: sock:inet_sock_set_state │ Status: {} │ Events: {events_label} (dropped: {}) │ Ringbuf: 256KB",
        report.status().as_str(),
        report.dropped_events(),
    );

    if area.height <= 10 {
        let text = format!(
            "{summary_text}\nEvents: {events_label}\nMode: {}",
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

    if event_count == 0 {
        let q = filter_query.unwrap_or("");
        let msg = format!(
            "No eBPF events match filter \"{q}\" ({total_event_count} total events observed)."
        );
        frame.render_widget(
            Paragraph::new(msg).block(
                Block::default()
                    .title(format!(
                        " Events (0 of {total_event_count}) [Filter: \"{q}\"] "
                    ))
                    .borders(Borders::ALL),
            ),
            rows[1],
        );
        return;
    }

    let inner_height = rows[1].height.saturating_sub(2) as usize;
    let start = clamp_scroll(scroll, event_count);
    let table_rows: Vec<Row> = filtered_events
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

    let title = match filter_query {
        Some(q) => format!(
            " Events {}..{} of {} [Filter: \"{q}\" ({event_count}/{total_event_count} matched)] ",
            start.saturating_add(1),
            (start + inner_height).min(event_count),
            event_count
        ),
        None => format!(
            " Events {}..{} of {} ",
            start.saturating_add(1),
            (start + inner_height).min(event_count),
            event_count
        ),
    };

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
    .block(Block::default().title(title).borders(Borders::ALL));
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
                if view.handle_key(key) {
                    break;
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

    #[test]
    fn filter_matching_metrics_sockets_and_ebpf_events() {
        use lidb_core::ebpf::{EbpfEventType, EbpfLifecycleEvent};
        use lidb_core::network::{SocketProtocol, SocketRecord, SocketState};
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

        // 1. Metric matching
        let metric = MetricObservation::new(
            "memory.active.bytes",
            "procfs/meminfo",
            Unit::Bytes,
            0,
            MetricState::Available(MetricValue::Integer(12345)),
        )
        .unwrap();

        assert!(filter_matches_metric(&metric, "memory"));
        assert!(filter_matches_metric(&metric, "ACTIVE"));
        assert!(filter_matches_metric(&metric, "meminfo"));
        assert!(filter_matches_metric(&metric, "bytes"));
        assert!(!filter_matches_metric(&metric, "cpu"));

        // 2. Socket matching (IPv4, endpoints, family, IPv6 bracketed endpoints)
        let sock = SocketRecord::new(
            SocketProtocol::Tcp,
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
            SocketState::Listen,
        )
        .with_inode(99999);

        assert!(filter_matches_socket(&sock, "tcp"));
        assert!(filter_matches_socket(&sock, "TCP"));
        assert!(filter_matches_socket(&sock, "ipv4"));
        assert!(filter_matches_socket(&sock, "8080"));
        assert!(filter_matches_socket(&sock, ":8080"));
        assert!(filter_matches_socket(&sock, "127.0.0.1"));
        assert!(filter_matches_socket(&sock, "127.0.0.1:8080"));
        assert!(filter_matches_socket(&sock, "listen"));
        assert!(filter_matches_socket(&sock, "99999"));
        assert!(!filter_matches_socket(&sock, "udp"));
        assert!(!filter_matches_socket(&sock, "ipv6"));

        let sock_v6 = SocketRecord::new(
            SocketProtocol::Tcp,
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            8443,
            SocketState::Listen,
        );
        assert!(filter_matches_socket(&sock_v6, "ipv6"));
        assert!(filter_matches_socket(&sock_v6, "[::1]:8443"));

        // 3. eBPF event matching (IPv4 & IPv6 endpoints, family, states, comm, pid)
        let event = EbpfLifecycleEvent::new(
            EbpfEventType::TcpClose,
            500000,
            1337,
            "nginx",
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)),
            443,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
            54321,
        )
        .unwrap()
        .with_states(Some(SocketState::Established), Some(SocketState::Closed));

        assert!(filter_matches_ebpf(&event, "nginx"));
        assert!(filter_matches_ebpf(&event, "1337"));
        assert!(filter_matches_ebpf(&event, "tcp_close"));
        assert!(filter_matches_ebpf(&event, "ipv4"));
        assert!(filter_matches_ebpf(&event, "443"));
        assert!(filter_matches_ebpf(&event, "10.0.0.2:443"));
        assert!(filter_matches_ebpf(&event, "192.168.1.50"));
        assert!(filter_matches_ebpf(&event, "192.168.1.50:54321"));
        assert!(filter_matches_ebpf(&event, "established"));
        assert!(filter_matches_ebpf(&event, "closed"));
        assert!(!filter_matches_ebpf(&event, "redis"));

        let event_v6 = EbpfLifecycleEvent::new(
            EbpfEventType::TcpConnect,
            600000,
            4242,
            "curl",
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            8443,
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            443,
        )
        .unwrap();
        assert!(filter_matches_ebpf(&event_v6, "ipv6"));
        assert!(filter_matches_ebpf(&event_v6, "[::1]:8443"));
        assert!(filter_matches_ebpf(&event_v6, "[2001:db8::1]:443"));
    }

    #[test]
    fn scroll_by_clamps_dynamically_against_filtered_results() {
        use lidb_core::ebpf::{EbpfEventType, EbpfLifecycleEvent, EbpfReport};
        use lidb_core::network::{
            NetworkReport, SocketProtocol, SocketRecord, SocketState, SourceAvailability,
        };
        use std::net::{IpAddr, Ipv4Addr};

        let mut snapshot = Snapshot::new(0, 0, SnapshotMode::Fixture);
        for i in 0..10 {
            let name = if i < 3 {
                format!("cpu.core{i}.usage")
            } else {
                format!("disk.sda{i}.bytes")
            };
            snapshot
                .push(
                    MetricObservation::new(
                        name,
                        "procfs",
                        Unit::Count,
                        0,
                        MetricState::Available(MetricValue::Integer(i as u64)),
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        let mut net_report = NetworkReport::new(100, 20, SnapshotMode::Fixture);
        for port in [80, 443, 8080, 9000, 9090] {
            let sock = SocketRecord::new(
                SocketProtocol::Tcp,
                IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                port,
                SocketState::Listen,
            );
            net_report.push_socket(sock).unwrap();
        }

        let mut ebpf_report = EbpfReport::new(SnapshotMode::Fixture, 500);
        ebpf_report.set_status(SourceAvailability::Available);
        for (idx, comm) in ["nginx", "nginx", "nginx", "sshd", "sshd"]
            .iter()
            .enumerate()
        {
            let event = EbpfLifecycleEvent::new(
                EbpfEventType::TcpConnect,
                1000 + idx as u64,
                2000 + idx as u32,
                *comm,
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                1000 + idx as u16,
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)),
                443,
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

        // Without filter: 10 metrics (max index = 9)
        view.scroll_by(50);
        assert_eq!(view.scroll, 9);

        // Filter for "cpu": only 3 metrics match (max index = 2).
        // Verify Up from stale large scroll clamps to max (2) before decrementing to 1.
        view.filter = "cpu".to_string();
        view.scroll = 10;
        view.scroll_by(-1);
        assert_eq!(view.scroll, 1);
        view.scroll_by(50);
        assert_eq!(view.scroll, 2);

        // Zero matches clamps scroll to 0.
        view.filter = "nonexistent".to_string();
        view.scroll_by(10);
        assert_eq!(view.scroll, 0);

        // Switch to Tab::Network with filter "80" (matches 80 and 8080 => 2 sockets, max index = 1)
        view.tab = Tab::Network;
        view.filter = "80".to_string();
        view.scroll = 0;
        view.scroll_by(50);
        assert_eq!(view.scroll, 1);

        // Clear filter on Network tab: all 5 sockets (max index = 4)
        view.filter.clear();
        view.scroll_by(50);
        assert_eq!(view.scroll, 4);

        // Switch to Tab::Ebpf: 5 events total (max index = 4), filter "nginx" leaves 3 (max index = 2)
        view.tab = Tab::Ebpf;
        view.scroll = 0;
        view.scroll_by(50);
        assert_eq!(view.scroll, 4);
        view.filter = "nginx".to_string();
        view.scroll_by(-1);
        assert_eq!(view.scroll, 1);
    }

    #[test]
    fn handle_key_search_and_filter_state_machine() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

        let press = |code: KeyCode| KeyEvent::new(code, KeyModifiers::NONE);
        let with_mods = |code: KeyCode, modifiers: KeyModifiers| KeyEvent::new(code, modifiers);

        let mut view = fixture();
        view.tab = Tab::Metrics;

        // Ignore key release events.
        let release_q = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        };
        assert!(!view.handle_key(release_q));

        // '/' opens search prompt on Metrics tab.
        assert!(!view.handle_key(press(KeyCode::Char('/'))));
        assert!(view.filtering);

        // Append printable characters, ignore control/alt modifiers and control chars.
        assert!(!view.handle_key(press(KeyCode::Char('M'))));
        assert!(!view.handle_key(press(KeyCode::Char('e'))));
        assert!(!view.handle_key(press(KeyCode::Char('m'))));
        assert!(!view.handle_key(with_mods(KeyCode::Char('x'), KeyModifiers::CONTROL)));
        assert!(!view.handle_key(with_mods(KeyCode::Char('y'), KeyModifiers::ALT)));
        assert!(!view.handle_key(press(KeyCode::Char('\n'))));
        assert_eq!(view.filter, "Mem");

        // Backspace removes one character; Enter applies and trims/normalizes.
        assert!(!view.handle_key(press(KeyCode::Backspace)));
        assert_eq!(view.filter, "Me");
        assert!(!view.handle_key(press(KeyCode::Char('m'))));
        assert!(!view.handle_key(press(KeyCode::Enter)));
        assert!(!view.filtering);
        assert!(view.is_filtered());
        assert_eq!(view.filter_display(), Some("Mem"));

        // Max length cap enforced at MAX_FILTER_LEN.
        assert!(!view.handle_key(press(KeyCode::Char('/'))));
        view.filter.clear();
        for _ in 0..(MAX_FILTER_LEN + 20) {
            assert!(!view.handle_key(press(KeyCode::Char('a'))));
        }
        assert_eq!(view.filter.chars().count(), MAX_FILTER_LEN);

        // Esc in prompt mode cancels prompt and clears filter without quitting.
        assert!(!view.handle_key(press(KeyCode::Esc)));
        assert!(!view.filtering);
        assert!(!view.is_filtered());

        // Whitespace-only input normalizes to empty filter on Enter.
        assert!(!view.handle_key(press(KeyCode::Char('/'))));
        assert!(!view.handle_key(press(KeyCode::Char(' '))));
        assert!(!view.handle_key(press(KeyCode::Char(' '))));
        assert_eq!(view.filter_display(), None);
        assert!(!view.is_filtered());
        assert!(!view.handle_key(press(KeyCode::Enter)));
        assert!(!view.filtering);
        assert!(view.filter.is_empty());

        // Esc outside prompt clears an active filter first before quitting.
        view.filter = "mem".to_string();
        assert!(!view.handle_key(press(KeyCode::Esc)));
        assert!(!view.is_filtered());
        assert!(view.handle_key(press(KeyCode::Esc)));

        // 'c' clears active filter even when Help or Detail modals are open.
        view.filter = "tcp".to_string();
        view.detail = true;
        assert!(!view.handle_key(press(KeyCode::Char('c'))));
        assert!(!view.is_filtered());
        assert!(view.detail);

        view.detail = false;
        view.help = true;
        view.filter = "udp".to_string();
        assert!(!view.handle_key(press(KeyCode::Char('c'))));
        assert!(!view.is_filtered());
        assert!(view.help);

        // '/' is ignored when Help modal is open or on CpuMemory tab.
        assert!(!view.handle_key(press(KeyCode::Char('/'))));
        assert!(!view.filtering);
        view.help = false;
        view.tab = Tab::CpuMemory;
        assert!(!view.handle_key(press(KeyCode::Char('/'))));
        assert!(!view.filtering);

        // Ctrl-C exits immediately even while typing inside search prompt.
        let mut quit_view = fixture();
        quit_view.tab = Tab::Metrics;
        assert!(!quit_view.handle_key(press(KeyCode::Char('/'))));
        assert!(quit_view.filtering);
        assert!(quit_view.handle_key(with_mods(KeyCode::Char('c'), KeyModifiers::CONTROL)));
    }

    #[test]
    fn tui_renders_search_prompt_and_filtered_tables_and_modals() {
        use lidb_core::ebpf::{EbpfEventType, EbpfLifecycleEvent, EbpfReport};
        use lidb_core::network::{
            NetworkReport, SocketProtocol, SocketRecord, SocketState, SourceAvailability,
        };
        use std::net::{IpAddr, Ipv4Addr};

        let mut view = fixture();

        // 1. Render active search prompt mode with original case preserved in display
        view.filtering = true;
        view.filter = "Mem".to_string();
        let prompt_out = render(100, 30, &mut view);
        assert!(prompt_out.contains("/Mem█"));
        assert!(prompt_out.contains("Enter: apply"));
        assert!(prompt_out.contains("Esc: cancel"));
        view.filtering = false;

        // 2. Render filtered metrics table (wide and narrow)
        let filtered_table = render(100, 30, &mut view);
        assert!(filtered_table.contains("[Filter: \"Mem\" (c to clear)]"));
        assert!(filtered_table.contains("memory.total.bytes"));
        assert!(!filtered_table.contains("pressure.cpu"));

        let narrow_filtered = render(70, 30, &mut view);
        assert!(narrow_filtered.contains("Metrics [Filter: \"Mem\""));
        assert!(narrow_filtered.contains("memory.total.bytes"));

        // 3. Render filtered metric detail modal preserving original filter case
        view.detail = true;
        let detail_out = render(100, 30, &mut view);
        assert!(detail_out.contains("Metric detail [Filter: \"Mem\"]"));
        assert!(detail_out.contains("memory.total.bytes"));

        // 4. Render Metrics filter matching 0 items (wide and narrow)
        view.filter = "nonexistent".to_string();
        let empty_detail = render(100, 30, &mut view);
        assert!(empty_detail.contains("No metrics match filter \"nonexistent\""));
        view.detail = false;
        let empty_table = render(100, 30, &mut view);
        assert!(empty_table.contains("No metrics match filter \"nonexistent\""));
        let narrow_empty = render(70, 30, &mut view);
        assert!(narrow_empty.contains("Metrics (0 of"));

        // 5. Network tab filtered rendering (wide, compact, 0-match, detail modal)
        view.tab = Tab::Network;
        view.filter = "51820".to_string();
        let mut net_report = NetworkReport::new(100, 20, SnapshotMode::Fixture);
        net_report.set_sockets_status(SourceAvailability::Available);
        let sock = SocketRecord::new(
            SocketProtocol::Udp,
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            51820,
            SocketState::Closed,
        );
        net_report.push_socket(sock).unwrap();
        view.update.as_mut().unwrap().network = Some(net_report);

        let net_out = render(100, 30, &mut view);
        assert!(net_out.contains("Sockets 1..1 of 1 [Filter: \"51820\" (1/1 matched)]"));
        assert!(net_out.contains("51820"));

        let net_compact = render(80, 10, &mut view);
        assert!(net_compact.contains("1/1 matching \"51820\""));

        view.detail = true;
        let net_detail = render(100, 30, &mut view);
        assert!(net_detail.contains("--- Selected Socket [1/1] [Filter: \"51820\"] ---"));
        assert!(net_detail.contains("0.0.0.0:51820"));

        view.detail = false;
        view.filter = "no_such_socket".to_string();
        let net_empty = render(100, 30, &mut view);
        assert!(net_empty.contains("No network sockets match filter \"no_such_socket\""));

        // 6. eBPF tab filtered rendering (table, compact, 0-match, detail modal)
        let mut ebpf_report = EbpfReport::new(SnapshotMode::Fixture, 500);
        ebpf_report.set_status(SourceAvailability::Available);
        let ev1 = EbpfLifecycleEvent::new(
            EbpfEventType::TcpConnect,
            1000,
            101,
            "nginx",
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            80,
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)),
            5001,
        )
        .unwrap();
        let ev2 = EbpfLifecycleEvent::new(
            EbpfEventType::TcpClose,
            2000,
            202,
            "sshd",
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            22,
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3)),
            5002,
        )
        .unwrap();
        ebpf_report.push_event(ev1);
        ebpf_report.push_event(ev2);
        view.update.as_mut().unwrap().ebpf = Some(ebpf_report);

        view.tab = Tab::Ebpf;
        view.filter = "Nginx".to_string();
        let ebpf_out = render(120, 30, &mut view);
        assert!(ebpf_out.contains("nginx"));
        assert!(!ebpf_out.contains("sshd"));
        assert!(ebpf_out.contains("[Filter: \"Nginx\" (1/2 matched)]"));

        let ebpf_compact = render(80, 10, &mut view);
        assert!(ebpf_compact.contains("eBPF Lifecycle Overview"));
        assert!(ebpf_compact.contains("1/2 matching \"Nginx\""));

        view.detail = true;
        let ebpf_detail = render(100, 30, &mut view);
        assert!(ebpf_detail.contains("nginx"));
        assert!(ebpf_detail.contains("eBPF Event detail [Filter: \"Nginx\"]"));

        view.detail = false;
        view.filter = "no_such_event".to_string();
        let ebpf_empty = render(120, 30, &mut view);
        assert!(ebpf_empty.contains("No eBPF events match filter \"no_such_event\""));
        assert!(ebpf_empty.contains("Events (0 of 2) [Filter: \"no_such_event\"]"));
    }

    #[test]
    fn format_bytes_value_and_binary_scaling() {
        assert_eq!(format_bytes_value(0.0), "0 B");
        assert_eq!(format_bytes_value(-0.0), "0 B");
        assert_eq!(format_rate_value(-0.0), "0 B/s");
        assert_eq!(format_bytes_value(512.0), "512 B");
        assert_eq!(format_bytes_value(1024.0), "1.0 KiB");
        assert_eq!(format_bytes_value(1_048_576.0), "1.0 MiB");
        assert_eq!(format_bytes_value(2_147_483_648.0), "2.0 GiB");
        assert_eq!(format_bytes_value(2.0 * 1_099_511_627_776.0), "2.0 TiB");
        assert_eq!(format_rate_value(1_099_511_627_776.0), "1.0 TB/s");
        assert_eq!(format_bytes_value(-1.0), "-");
        assert_eq!(format_bytes_value(f64::NAN), "-");
        assert_eq!(format_bytes_value(f64::INFINITY), "-");
    }

    #[test]
    fn collect_disk_io_rows_handles_dotted_names_and_ignores_state() {
        let mut snapshot = Snapshot::new(0, 10, SnapshotMode::Fixture);
        for (name, unit, val) in [
            ("disk.state", Unit::Count, MetricValue::Integer(2)),
            (
                "disk.nvme0n1.read.bytes",
                Unit::Bytes,
                MetricValue::Integer(1_048_576),
            ),
            (
                "disk.nvme0n1.read.bytes_per_second",
                Unit::BytesPerSecond,
                MetricValue::Number(2048.0),
            ),
            (
                "disk.nvme0n1.write.bytes",
                Unit::Bytes,
                MetricValue::Integer(2_097_152),
            ),
            (
                "disk.nvme0n1.write.bytes_per_second",
                Unit::BytesPerSecond,
                MetricValue::Number(4096.0),
            ),
            (
                "disk.loop.0.read.bytes",
                Unit::Bytes,
                MetricValue::Integer(512),
            ),
            (
                "disk.loop.0.write.bytes",
                Unit::Bytes,
                MetricValue::Integer(0),
            ),
        ] {
            snapshot
                .push(
                    MetricObservation::new(
                        name,
                        "proc.diskstats",
                        unit,
                        0,
                        MetricState::Available(val),
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        let rows = collect_disk_io_rows(&snapshot);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].device, "loop.0");
        assert!(rows[0].read_bytes.is_some());
        assert!(rows[0].read_rate.is_none());
        assert_eq!(rows[1].device, "nvme0n1");
        assert_eq!(
            rows[1].read_rate.map(format_metric_rate).as_deref(),
            Some("2.0 KB/s")
        );
        assert_eq!(
            rows[1].write_bytes.map(format_metric_bytes).as_deref(),
            Some("2.0 MiB")
        );
    }

    #[test]
    fn cpu_memory_tab_renders_disk_io_panel_scrolling_and_detail_modal() {
        let mut snapshot = Snapshot::new(500, 25, SnapshotMode::Fixture);
        snapshot
            .push(
                MetricObservation::new(
                    "disk.state",
                    "proc.diskstats",
                    Unit::Count,
                    500,
                    MetricState::Available(MetricValue::Integer(4)),
                )
                .unwrap(),
            )
            .unwrap();
        for (dev, r_bps, w_bps, r_tot, w_tot) in [
            ("dm-0", 1_048_576.0, 524_288.0, 10_485_760, 5_242_880),
            ("loop.0", 0.0, 0.0, 4096, 0),
            (
                "nvme0n1",
                52_428_800.0,
                10_485_760.0,
                1_073_741_824,
                536_870_912,
            ),
            ("sda", 2048.0, 4096.0, 65536, 131_072),
        ] {
            for (suffix, unit, state) in [
                (
                    "read.bytes_per_second",
                    Unit::BytesPerSecond,
                    MetricState::Available(MetricValue::Number(r_bps)),
                ),
                (
                    "write.bytes_per_second",
                    Unit::BytesPerSecond,
                    MetricState::Available(MetricValue::Number(w_bps)),
                ),
                (
                    "read.bytes",
                    Unit::Bytes,
                    MetricState::Available(MetricValue::Integer(r_tot)),
                ),
                (
                    "write.bytes",
                    Unit::Bytes,
                    MetricState::Available(MetricValue::Integer(w_tot)),
                ),
            ] {
                snapshot
                    .push(
                        MetricObservation::new(
                            format!("disk.{dev}.{suffix}"),
                            "proc.diskstats",
                            unit,
                            500,
                            state,
                        )
                        .unwrap(),
                    )
                    .unwrap();
            }
        }

        let mut view = View {
            update: Some(Update {
                snapshot,
                network: None,
                ebpf: None,
                collected_at: Instant::now(),
                skipped_updates: 0,
            }),
            tab: Tab::CpuMemory,
            fixture: true,
            ..View::default()
        };

        // 1. Wide layout (>= 19 body rows, width 120) renders full Disk I/O table
        let wide = render(120, 30, &mut view);
        assert!(wide.contains("Disk I/O (4 devices, /proc/diskstats)"));
        assert!(wide.contains("nvme0n1"));
        assert!(wide.contains("50.0 MB/s"));
        assert!(wide.contains("10.0 MB/s"));
        assert!(wide.contains("1.0 GiB"));
        assert!(wide.contains("512.0 MiB"));
        assert!(wide.contains("loop.0"));

        // 2. Narrow layout (< 70 width) and medium-short layout (height 20 => body 16)
        let narrow = render(50, 30, &mut view);
        assert!(narrow.contains("Disk I/O (4 devices, /proc/diskstats)"));
        assert!(narrow.contains("nvme0n1"));

        let narrow_40 = render(40, 30, &mut view);
        assert!(narrow_40.contains("Uptime"));
        assert!(narrow_40.contains("Disk I/O (4 devices"));

        let medium_short = render(100, 20, &mut view);
        assert!(medium_short.contains("Disk I/O (4 devices): dm-0 R 1.0 MB/s / W 512.0 KB/s"));

        // Narrow medium-height suppresses compact line that would wrap & clip mid-rate
        let narrow_compact = render(45, 20, &mut view);
        assert!(narrow_compact.contains("Uptime"));
        assert!(!narrow_compact.contains("dm-0 R 1.0 MB/s"));

        // 3. Tab::CpuMemory scrolling bounds cleanly against disk device count (4 => max 3)
        view.scroll = 1;
        let mid_scrolled = render(120, 24, &mut view);
        assert!(mid_scrolled.contains("Disk I/O 2..4 of 4 (/proc/diskstats)"));
        assert!(mid_scrolled.contains("loop.0"));
        assert!(!mid_scrolled.contains("dm-0"));

        view.scroll_by(10);
        assert_eq!(view.scroll, 3);
        let scrolled = render(120, 23, &mut view);
        assert!(scrolled.contains("Disk I/O 4..4 of 4 (/proc/diskstats)"));
        assert!(scrolled.contains("sda"));
        assert!(!scrolled.contains("dm-0"));
        view.scroll = 0;

        // 4. Detail modal shows Section 5 Block Devices / Disk I/O with human + exact bytes
        view.detail = true;
        view.text_scroll = 2;
        assert!(!view.handle_key(event::KeyEvent::from(KeyCode::Char('n'))));
        assert_eq!(view.text_scroll, 2);
        assert_eq!(view.scroll, 0);
        let detail_out = render(100, 60, &mut view);
        assert!(detail_out.contains("--- Block Devices / Disk I/O (/proc/diskstats) ---"));
        assert!(detail_out.contains("Source Status: available (4 devices)"));
        assert!(detail_out.contains("Device: nvme0n1"));
        assert!(detail_out.contains("Read Rate: 50.0 MB/s (total: 1.0 GiB (1073741824 B))"));
        assert!(detail_out.contains("Write Rate: 10.0 MB/s (total: 512.0 MiB (536870912 B))"));
        assert!(!detail_out.contains("n/p item"));
        view.text_scroll = 0;

        // 5. Sample #1 awaiting rate baseline marks status truthfully (never zero!)
        let mut first_snap = Snapshot::new(100, 5, SnapshotMode::Fixture);
        first_snap
            .push(
                MetricObservation::new(
                    "disk.state",
                    "proc.diskstats",
                    Unit::Count,
                    100,
                    MetricState::Available(MetricValue::Integer(1)),
                )
                .unwrap(),
            )
            .unwrap();
        first_snap
            .push(
                MetricObservation::new(
                    "disk.sda.read.bytes_per_second",
                    "proc.diskstats",
                    Unit::BytesPerSecond,
                    100,
                    MetricState::TemporarilyUnavailable(
                        "rate requires a second successful observation".into(),
                    ),
                )
                .unwrap(),
            )
            .unwrap();
        first_snap
            .push(
                MetricObservation::new(
                    "disk.sda.read.bytes",
                    "proc.diskstats",
                    Unit::Bytes,
                    100,
                    MetricState::Available(MetricValue::Integer(4096)),
                )
                .unwrap(),
            )
            .unwrap();
        view.detail = false;
        view.update.as_mut().unwrap().snapshot = first_snap;
        let awaiting = render(110, 30, &mut view);
        assert!(awaiting.contains("Disk I/O (1 device, /proc/diskstats) [awaiting rate baseline]"));
        assert!(awaiting.contains("4.0 KiB"));
        let awaiting_narrow = render(50, 30, &mut view);
        assert!(awaiting_narrow.contains("Disk I/O (1 device) [baseline]"));
        view.detail = true;
        let awaiting_detail = render(110, 45, &mut view);
        assert!(awaiting_detail.contains("Source Status: available (1 device)"));
        assert!(awaiting_detail.contains("rate requires a second successful observation"));

        // 6. Rate counter reset / error surfaces clear status cue
        let mut rate_err_snap = Snapshot::new(150, 5, SnapshotMode::Fixture);
        rate_err_snap
            .push(
                MetricObservation::new(
                    "disk.state",
                    "proc.diskstats",
                    Unit::Count,
                    150,
                    MetricState::Available(MetricValue::Integer(1)),
                )
                .unwrap(),
            )
            .unwrap();
        rate_err_snap
            .push(
                MetricObservation::new(
                    "disk.sda.read.bytes_per_second",
                    "proc.diskstats",
                    Unit::BytesPerSecond,
                    150,
                    MetricState::Error("transfer counter decreased; baseline reset".into()),
                )
                .unwrap(),
            )
            .unwrap();
        view.detail = false;
        view.update.as_mut().unwrap().snapshot = rate_err_snap;
        let rate_err = render(100, 30, &mut view);
        assert!(rate_err.contains("[rate error]"));
        let rate_err_compact = render(100, 20, &mut view);
        assert!(rate_err_compact.contains("Disk I/O: 1 device (rate error)"));

        // 7. Unavailable / permission denied /proc/diskstats renders explicit reason cleanly
        let mut denied_snap = Snapshot::new(200, 5, SnapshotMode::Fixture);
        denied_snap
            .push(
                MetricObservation::new(
                    "disk.state",
                    "proc.diskstats",
                    Unit::Count,
                    200,
                    MetricState::PermissionDenied("diskstats access denied".into()),
                )
                .unwrap(),
            )
            .unwrap();
        view.detail = false;
        view.update.as_mut().unwrap().snapshot = denied_snap;
        let denied = render(100, 30, &mut view);
        assert!(denied.contains("Disk I/O permission_denied: diskstats access denied"));
        view.detail = true;
        let denied_detail = render(100, 45, &mut view);
        assert!(denied_detail.contains("--- Block Devices / Disk I/O (/proc/diskstats) ---"));
        assert!(denied_detail.contains("Disk I/O permission_denied: diskstats access denied"));
        assert!(!denied_detail.contains("Source Status: permission_denied"));
    }
}
