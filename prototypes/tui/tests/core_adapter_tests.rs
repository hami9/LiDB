use lidb_core::{Capability, CapabilityRegistry, CapabilityState};
use lidb_tui_prototype::{
    app::App,
    core_adapter::{CapabilityView, GPU_CAPABILITY_ID},
    model::{gpu_ai::GpuViewMode, DataSourceStatus},
    ui::tabs::gpu_ai,
};
use ratatui::{backend::TestBackend, Terminal};

fn registry_with(state: CapabilityState) -> CapabilityRegistry {
    let mut registry = CapabilityRegistry::new();
    let reason = (state != CapabilityState::Available).then(|| "Reason from core: denied".into());
    registry
        .register(Capability::new(GPU_CAPABILITY_ID, state, reason).unwrap())
        .unwrap();
    registry
}

fn render_gpu(app: &App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| {
            let area = frame.area();
            gpu_ai::render(frame, area, app);
        })
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn bootstrap_disabled_is_visible_without_becoming_a_sample() {
    let mut app = App::new();
    assert_eq!(app.gpu_capability.state(), Some(CapabilityState::Disabled));
    assert_eq!(app.gpu_capability.reason(), "Collector not implemented in P0");
    for _ in 0..100 {
        app.on_tick();
    }
    assert!(app.gpu_ai.devices.is_empty());
    assert!(app.gpu_ai.workloads.is_empty());
    assert!(matches!(app.gpu_ai.status, DataSourceStatus::NotProbed { .. }));
    let output = render_gpu(&app, 120, 40);
    assert!(output.contains("[DISABLED]"));
    assert!(output.contains("Collector not implemented in P0"));
    assert!(output.contains("Telemetry: [NOT PROBED]"));
    assert!(output.contains("lidb-core registry"));
}

#[test]
fn core_states_render_distinctly_and_never_imply_live_samples() {
    for (state, badge) in [
        (CapabilityState::Available, "[AVAILABLE]"),
        (CapabilityState::Unsupported, "[UNSUPPORTED]"),
        (CapabilityState::Disabled, "[DISABLED]"),
        (CapabilityState::PermissionDenied, "[PERMISSION DENIED]"),
        (
            CapabilityState::TemporarilyUnavailable,
            "[TEMPORARILY UNAVAILABLE]",
        ),
        (CapabilityState::Stale, "[STALE]"),
        (CapabilityState::Error, "[ERROR]"),
    ] {
        let app = App::with_capabilities(&registry_with(state));
        assert_eq!(app.gpu_capability.state(), Some(state));
        let output = render_gpu(&app, 120, 40);
        assert!(output.contains(badge), "{state:?}: {output}");
        if state != CapabilityState::Available {
            assert!(output.contains("Reason from core: denied"));
        }
        assert!(output.contains("Telemetry: [NOT PROBED]"));
        assert!(!output.contains("[LIVE TELEMETRY]"));
        assert!(!output.contains("[SIMULATED FIXTURE]"));
        assert!(!output.contains('%'));
        assert!(app.gpu_ai.devices.is_empty());
        assert!(app.gpu_ai.workloads.is_empty());
    }
}

#[test]
fn absent_registration_is_unknown_and_snapshot_is_owned() {
    let mut registry = CapabilityRegistry::new();
    let view = CapabilityView::gpu(&registry);
    registry
        .register(
            Capability::new(
                GPU_CAPABILITY_ID,
                CapabilityState::Unsupported,
                Some("Unsupported by provider".into()),
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(view.state(), None);
    assert_eq!(view.badge_label(), "[NOT PROBED]");
    let app = App::with_capabilities(&CapabilityRegistry::new());
    let output = render_gpu(&app, 120, 40);
    assert!(output.contains("Capability not registered; host support is unknown."));
    assert!(!output.contains("[UNSUPPORTED]"));
    assert!(!output.contains("[DISABLED]"));
}

#[test]
fn explicit_demo_toggle_preserves_core_state_and_returns_to_unprobed() {
    let mut app = App::with_capabilities(&registry_with(CapabilityState::PermissionDenied));
    app.toggle_gpu_mode();
    assert_eq!(app.gpu_ai.view_mode, GpuViewMode::SimulatedFixture);
    assert!(app.gpu_ai.status.is_simulated());
    assert_eq!(app.gpu_capability.state(), Some(CapabilityState::PermissionDenied));
    app.toggle_gpu_mode();
    assert_eq!(app.gpu_ai.view_mode, GpuViewMode::HostReality);
    assert!(matches!(app.gpu_ai.status, DataSourceStatus::NotProbed { .. }));
    assert!(app.gpu_ai.devices.is_empty());
    assert!(render_gpu(&app, 120, 40).contains("[PERMISSION DENIED]"));
}

#[test]
fn unavailable_capability_renders_on_narrow_and_unicode_terminals() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register(
            Capability::new(
                GPU_CAPABILITY_ID,
                CapabilityState::TemporarilyUnavailable,
                Some("Provider unavailable: GPU \u{1f5a5}".into()),
            )
            .unwrap(),
        )
        .unwrap();
    let app = App::with_capabilities(&registry);
    for (width, height) in [(80, 24), (120, 40)] {
        let output = render_gpu(&app, width, height);
        assert!(output.contains("[TEMPORARILY UNAVAILABLE]"));
    }
    render_gpu(&app, 40, 10);
    assert!(render_gpu(&app, 120, 40).contains("Provider unavailable: GPU \u{1f5a5}"));
}
