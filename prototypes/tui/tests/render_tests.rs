use lidb_tui_prototype::{
    app::{App, Modal, Tab},
    model::gpu_ai::GpuAiTelemetry,
    ui,
};
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn test_render_all_tabs_headless() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("Failed to create TestBackend");
    let mut app = App::new();

    for tab in Tab::ALL {
        app.current_tab = tab;
        terminal
            .draw(|f| ui::draw(f, &app))
            .expect("Render should not fail");
    }
}

#[test]
fn test_render_gpu_fixture_mode() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("Failed to create TestBackend");
    let mut app = App::new();
    app.current_tab = Tab::GpuAi;
    app.gpu_ai = GpuAiTelemetry::simulated_fixture();

    terminal
        .draw(|f| ui::draw(f, &app))
        .expect("Render should not fail in GPU fixture mode");
}

#[test]
fn test_render_all_modals() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("Failed to create TestBackend");
    let mut app = App::new();

    // 1. Help Modal
    app.active_modal = Modal::Help;
    terminal
        .draw(|f| ui::draw(f, &app))
        .expect("Render Help modal should not fail");

    // 2. Process Detail Modal
    if let Some(proc) = app.processes.items.first().cloned() {
        app.active_modal = Modal::ProcessDetail(proc);
        terminal
            .draw(|f| ui::draw(f, &app))
            .expect("Render Process detail modal should not fail");
    }

    // 3. Diagnostic Detail Modal
    if let Some(check) = app.diagnostics.checks.first().cloned() {
        app.active_modal = Modal::DiagnosticDetail(check);
        terminal
            .draw(|f| ui::draw(f, &app))
            .expect("Render Diagnostic detail modal should not fail");
    }
}

#[test]
fn test_render_small_terminal_fallback() {
    // Narrow terminal: 40 cols, 10 rows (below 50x12 threshold)
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("Failed to create TestBackend");
    let app = App::new();

    terminal
        .draw(|f| ui::draw(f, &app))
        .expect("Render on small terminal should gracefully fallback without panic");
}
