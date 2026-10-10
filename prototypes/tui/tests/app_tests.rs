use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lidb_tui_prototype::{
    app::{App, Modal, Tab},
    model::gpu_ai::GpuViewMode,
    theme::ThemeMode,
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn test_tab_direct_navigation() {
    let mut app = App::new();
    assert_eq!(app.current_tab, Tab::System);

    app.handle_key(key(KeyCode::Char('2')));
    assert_eq!(app.current_tab, Tab::CpuMemory);

    app.handle_key(key(KeyCode::Char('3')));
    assert_eq!(app.current_tab, Tab::Networking);

    app.handle_key(key(KeyCode::Char('4')));
    assert_eq!(app.current_tab, Tab::GpuAi);

    app.handle_key(key(KeyCode::Char('5')));
    assert_eq!(app.current_tab, Tab::Processes);

    app.handle_key(key(KeyCode::Char('6')));
    assert_eq!(app.current_tab, Tab::Diagnostics);

    app.handle_key(key(KeyCode::Char('7')));
    assert_eq!(app.current_tab, Tab::SettingsHelp);
}

#[test]
fn test_tab_cycle_navigation() {
    let mut app = App::new();
    assert_eq!(app.current_tab, Tab::System);

    // Forward cycle
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.current_tab, Tab::CpuMemory);

    app.handle_key(key(KeyCode::Right));
    assert_eq!(app.current_tab, Tab::Networking);

    // Backward cycle
    app.handle_key(key(KeyCode::BackTab));
    assert_eq!(app.current_tab, Tab::CpuMemory);

    app.handle_key(key(KeyCode::Left));
    assert_eq!(app.current_tab, Tab::System);

    app.handle_key(key(KeyCode::Left));
    assert_eq!(app.current_tab, Tab::SettingsHelp);
}

#[test]
fn test_pause_and_theme_toggle() {
    let mut app = App::new();
    assert!(!app.settings.paused);

    app.handle_key(key(KeyCode::Char(' ')));
    assert!(app.settings.paused);

    app.handle_key(key(KeyCode::Char(' ')));
    assert!(!app.settings.paused);

    // Theme cycling
    assert_eq!(app.theme.mode, ThemeMode::Dark);
    app.handle_key(key(KeyCode::Char('t')));
    assert_eq!(app.theme.mode, ThemeMode::Light);
    app.handle_key(key(KeyCode::Char('t')));
    assert_eq!(app.theme.mode, ThemeMode::HighContrast);
    app.handle_key(key(KeyCode::Char('t')));
    assert_eq!(app.theme.mode, ThemeMode::Monochrome);
    app.handle_key(key(KeyCode::Char('t')));
    assert_eq!(app.theme.mode, ThemeMode::Dark);
}

#[test]
fn test_gpu_mode_toggle() {
    let mut app = App::new();
    assert_eq!(app.gpu_ai.view_mode, GpuViewMode::HostReality);

    app.handle_key(key(KeyCode::Char('g')));
    assert_eq!(app.gpu_ai.view_mode, GpuViewMode::SimulatedFixture);
    assert!(!app.gpu_ai.devices.is_empty());

    app.handle_key(key(KeyCode::Char('g')));
    assert_eq!(app.gpu_ai.view_mode, GpuViewMode::HostReality);
}

#[test]
fn test_process_search_filter() {
    let mut app = App::new();
    app.current_tab = Tab::Processes;
    let initial_count = app.filtered_processes().len();
    assert!(initial_count > 0);

    // Activate search mode
    app.handle_key(key(KeyCode::Char('/')));
    assert!(app.is_filtering_processes);

    // Type "vllm"
    app.handle_key(key(KeyCode::Char('v')));
    app.handle_key(key(KeyCode::Char('l')));
    app.handle_key(key(KeyCode::Char('l')));
    app.handle_key(key(KeyCode::Char('m')));

    assert_eq!(app.processes.filter, "vllm");
    let filtered = app.filtered_processes();
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].command.contains("vllm"));

    // Exit search mode with Enter
    app.handle_key(key(KeyCode::Enter));
    assert!(!app.is_filtering_processes);
}

#[test]
fn test_help_and_detail_modals() {
    let mut app = App::new();
    assert_eq!(app.active_modal, Modal::None);

    // Open Help modal
    app.handle_key(key(KeyCode::Char('?')));
    assert_eq!(app.active_modal, Modal::Help);

    // Dismiss with Esc
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.active_modal, Modal::None);

    // Open detail modal on Processes tab
    app.current_tab = Tab::Processes;
    app.handle_key(key(KeyCode::Enter));
    assert!(matches!(app.active_modal, Modal::ProcessDetail(_)));

    // Dismiss with 'q'
    app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(app.active_modal, Modal::None);
}

#[test]
fn test_quit_key() {
    let mut app = App::new();
    assert!(!app.should_quit);

    app.handle_key(key(KeyCode::Char('q')));
    assert!(app.should_quit);
}

#[test]
fn test_process_sorting_and_nan_safety() {
    use lidb_tui_prototype::model::processes::{ProcessItem, ProcessSortField};

    let mut app = App::new();
    app.current_tab = Tab::Processes;

    // Inject processes including NaN floats
    app.processes.items.push(ProcessItem {
        pid: 9999,
        user: "nobody".to_string(),
        cpu_pct: f32::NAN,
        mem_pct: f32::NAN,
        rss_bytes: 1024,
        state: "S".to_string(),
        command: "nan-proc".to_string(),
        threads: 1,
        io_read_kb_s: 0,
        io_write_kb_s: 0,
    });

    // Default sort is CPU
    assert_eq!(app.processes.sort_field, ProcessSortField::Cpu);

    // Cycle to Mem (should not panic with NaN)
    app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(app.processes.sort_field, ProcessSortField::Mem);

    // Cycle to Pid
    app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(app.processes.sort_field, ProcessSortField::Pid);

    // Cycle to Command
    app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(app.processes.sort_field, ProcessSortField::Command);

    // Cycle back to Cpu (should not panic with NaN)
    app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(app.processes.sort_field, ProcessSortField::Cpu);
}

#[test]
fn test_process_list_navigation() {
    let mut app = App::new();
    app.current_tab = Tab::Processes;
    assert_eq!(app.process_selected_idx, 0);

    // Navigate down
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.process_selected_idx, 1);

    // Navigate up
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.process_selected_idx, 0);

    // Navigate up at top boundary stays at 0
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.process_selected_idx, 0);
}
