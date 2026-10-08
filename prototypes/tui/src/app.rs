use crate::{
    fixtures::{initial_fixtures, FixtureManager},
    model::{
        cpu_mem::CpuMemoryTelemetry,
        diagnostics::{DiagnosticCheck, DiagnosticsTelemetry},
        gpu_ai::{GpuAiTelemetry, GpuViewMode},
        network::NetworkTelemetry,
        processes::{ProcessItem, ProcessSortField, ProcessTelemetry},
        settings::AppSettings,
        system::SystemTelemetry,
    },
    theme::Theme,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    System = 0,
    CpuMemory = 1,
    Networking = 2,
    GpuAi = 3,
    Processes = 4,
    Diagnostics = 5,
    SettingsHelp = 6,
}

impl Tab {
    pub const ALL: [Tab; 7] = [
        Tab::System,
        Tab::CpuMemory,
        Tab::Networking,
        Tab::GpuAi,
        Tab::Processes,
        Tab::Diagnostics,
        Tab::SettingsHelp,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Self::System => "1:System",
            Self::CpuMemory => "2:CPU/Mem",
            Self::Networking => "3:Network",
            Self::GpuAi => "4:GPU/AI",
            Self::Processes => "5:Processes",
            Self::Diagnostics => "6:Diagnostics",
            Self::SettingsHelp => "7:Settings/Help",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::System => Self::CpuMemory,
            Self::CpuMemory => Self::Networking,
            Self::Networking => Self::GpuAi,
            Self::GpuAi => Self::Processes,
            Self::Processes => Self::Diagnostics,
            Self::Diagnostics => Self::SettingsHelp,
            Self::SettingsHelp => Self::System,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::System => Self::SettingsHelp,
            Self::CpuMemory => Self::System,
            Self::Networking => Self::CpuMemory,
            Self::GpuAi => Self::Networking,
            Self::Processes => Self::GpuAi,
            Self::Diagnostics => Self::Processes,
            Self::SettingsHelp => Self::Diagnostics,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Modal {
    None,
    Help,
    ProcessDetail(ProcessItem),
    DiagnosticDetail(DiagnosticCheck),
}

pub struct App {
    pub current_tab: Tab,
    pub active_modal: Modal,
    pub theme: Theme,
    pub settings: AppSettings,

    pub system: SystemTelemetry,
    pub cpu_mem: CpuMemoryTelemetry,
    pub network: NetworkTelemetry,
    pub gpu_ai: GpuAiTelemetry,
    pub processes: ProcessTelemetry,
    pub diagnostics: DiagnosticsTelemetry,

    pub fixture_manager: FixtureManager,

    pub process_selected_idx: usize,
    pub diagnostics_selected_idx: usize,
    pub network_selected_idx: usize,

    pub is_filtering_processes: bool,
    pub should_quit: bool,
    pub tick_counter: u64,
}

impl Default for App {
    fn default() -> Self {
        let (system, cpu_mem, network, gpu_ai, processes, diagnostics) = initial_fixtures();
        let settings = AppSettings::default();
        let theme = Theme::new(settings.theme_mode);

        Self {
            current_tab: Tab::System,
            active_modal: Modal::None,
            theme,
            settings,
            system,
            cpu_mem,
            network,
            gpu_ai,
            processes,
            diagnostics,
            fixture_manager: FixtureManager::new(),
            process_selected_idx: 0,
            diagnostics_selected_idx: 0,
            network_selected_idx: 0,
            is_filtering_processes: false,
            should_quit: false,
            tick_counter: 0,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_tick(&mut self) {
        if self.settings.paused {
            return;
        }
        self.tick_counter = self.tick_counter.wrapping_add(1);
        self.fixture_manager.advance_tick(
            &mut self.system,
            &mut self.cpu_mem,
            &mut self.network,
            &mut self.gpu_ai,
            &mut self.processes,
        );
    }

    pub fn filtered_processes(&self) -> Vec<&ProcessItem> {
        let q = self.processes.filter.to_lowercase();
        self.processes
            .items
            .iter()
            .filter(|p| {
                if q.is_empty() {
                    true
                } else {
                    p.command.to_lowercase().contains(&q)
                        || p.user.to_lowercase().contains(&q)
                        || p.pid.to_string().contains(&q)
                }
            })
            .collect()
    }

    pub fn toggle_pause(&mut self) {
        self.settings.paused = !self.settings.paused;
    }

    pub fn cycle_theme(&mut self) {
        self.settings.theme_mode = self.settings.theme_mode.next();
        self.theme = Theme::new(self.settings.theme_mode);
    }

    pub fn toggle_gpu_mode(&mut self) {
        if self.gpu_ai.view_mode == GpuViewMode::HostReality {
            self.gpu_ai = GpuAiTelemetry::simulated_fixture();
        } else {
            self.gpu_ai = GpuAiTelemetry::default();
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Handle filter input typing mode on Processes tab
        if self.is_filtering_processes {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.is_filtering_processes = false;
                }
                KeyCode::Backspace => {
                    self.processes.filter.pop();
                    self.process_selected_idx = 0;
                }
                KeyCode::Char(c) => {
                    self.processes.filter.push(c);
                    self.process_selected_idx = 0;
                }
                _ => {}
            }
            return;
        }

        // Handle modal dismiss / key events
        if self.active_modal != Modal::None {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                    self.active_modal = Modal::None;
                }
                _ => {}
            }
            return;
        }

        // Normal global key handling
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('?') | KeyCode::F(1) => self.active_modal = Modal::Help,
            KeyCode::Char(' ') => self.toggle_pause(),
            KeyCode::Char('t') => self.cycle_theme(),
            KeyCode::Char('g') => self.toggle_gpu_mode(),

            // Tab selection via numbers
            KeyCode::Char('1') => self.current_tab = Tab::System,
            KeyCode::Char('2') => self.current_tab = Tab::CpuMemory,
            KeyCode::Char('3') => self.current_tab = Tab::Networking,
            KeyCode::Char('4') => self.current_tab = Tab::GpuAi,
            KeyCode::Char('5') => self.current_tab = Tab::Processes,
            KeyCode::Char('6') => self.current_tab = Tab::Diagnostics,
            KeyCode::Char('7') => self.current_tab = Tab::SettingsHelp,

            // Tab navigation
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                self.current_tab = self.current_tab.next();
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                self.current_tab = self.current_tab.prev();
            }

            // Vertical list navigation
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Up | KeyCode::Char('k') => self.select_prev(),

            // Actions per tab
            KeyCode::Enter => self.open_detail_modal(),
            KeyCode::Char('/') if self.current_tab == Tab::Processes => {
                self.is_filtering_processes = true;
            }
            KeyCode::Char('s') if self.current_tab == Tab::Processes => {
                self.cycle_process_sort();
            }
            _ => {}
        }
    }

    fn select_next(&mut self) {
        match self.current_tab {
            Tab::Processes => {
                let count = self.filtered_processes().len();
                if count > 0 && self.process_selected_idx + 1 < count {
                    self.process_selected_idx += 1;
                }
            }
            Tab::Diagnostics => {
                let count = self.diagnostics.checks.len();
                if count > 0 && self.diagnostics_selected_idx + 1 < count {
                    self.diagnostics_selected_idx += 1;
                }
            }
            Tab::Networking => {
                let count = self.network.interfaces.len();
                if count > 0 && self.network_selected_idx + 1 < count {
                    self.network_selected_idx += 1;
                }
            }
            _ => {}
        }
    }

    fn select_prev(&mut self) {
        match self.current_tab {
            Tab::Processes => {
                self.process_selected_idx = self.process_selected_idx.saturating_sub(1);
            }
            Tab::Diagnostics => {
                self.diagnostics_selected_idx = self.diagnostics_selected_idx.saturating_sub(1);
            }
            Tab::Networking => {
                self.network_selected_idx = self.network_selected_idx.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn open_detail_modal(&mut self) {
        match self.current_tab {
            Tab::Processes => {
                let items = self.filtered_processes();
                if let Some(item) = items.get(self.process_selected_idx) {
                    self.active_modal = Modal::ProcessDetail((*item).clone());
                }
            }
            Tab::Diagnostics => {
                if let Some(check) = self.diagnostics.checks.get(self.diagnostics_selected_idx) {
                    self.active_modal = Modal::DiagnosticDetail(check.clone());
                }
            }
            _ => {}
        }
    }

    fn cycle_process_sort(&mut self) {
        self.processes.sort_field = match self.processes.sort_field {
            ProcessSortField::Cpu => ProcessSortField::Mem,
            ProcessSortField::Mem => ProcessSortField::Pid,
            ProcessSortField::Pid => ProcessSortField::Command,
            ProcessSortField::Command => ProcessSortField::Cpu,
        };

        // Sort items in place according to sort_field
        match self.processes.sort_field {
            ProcessSortField::Cpu => {
                self.processes
                    .items
                    .sort_by(|a, b| b.cpu_pct.partial_cmp(&a.cpu_pct).unwrap());
            }
            ProcessSortField::Mem => {
                self.processes
                    .items
                    .sort_by(|a, b| b.mem_pct.partial_cmp(&a.mem_pct).unwrap());
            }
            ProcessSortField::Pid => {
                self.processes.items.sort_by_key(|a| a.pid);
            }
            ProcessSortField::Command => {
                self.processes
                    .items
                    .sort_by(|a, b| a.command.cmp(&b.command));
            }
        }
    }
}
