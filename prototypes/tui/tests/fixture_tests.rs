use lidb_tui_prototype::{
    fixtures::FixtureManager,
    model::{
        cpu_mem::CpuMemoryTelemetry,
        diagnostics::DiagnosticsTelemetry,
        gpu_ai::{GpuAiTelemetry, GpuViewMode},
        network::NetworkTelemetry,
        processes::ProcessTelemetry,
        system::SystemTelemetry,
        DataSourceStatus,
    },
};

#[test]
fn test_fixture_deterministic_advance() {
    let mut fixture = FixtureManager::new();

    let mut system = SystemTelemetry::default();
    let mut cpu_mem = CpuMemoryTelemetry::default();
    let mut net = NetworkTelemetry::default();
    let mut gpu = GpuAiTelemetry::simulated_fixture();
    let mut proc = ProcessTelemetry::default();

    let initial_uptime = system.uptime_seconds;

    // Advance 10 ticks
    for _ in 0..10 {
        fixture.advance_tick(&mut system, &mut cpu_mem, &mut net, &mut gpu, &mut proc);
    }

    // Assert uptime increased by 10
    assert_eq!(system.uptime_seconds, initial_uptime + 10);

    // Assert CPU within valid bounds
    assert!(cpu_mem.overall_cpu_pct >= 0.0 && cpu_mem.overall_cpu_pct <= 100.0);
    assert!(!cpu_mem.cpu_history.is_empty());
    assert!(cpu_mem.cpu_history.len() <= 30);

    // Assert Network history is bounded
    assert!(net.rx_history.len() <= 30);
    assert!(net.tx_history.len() <= 30);

    // Assert GPU devices updated in simulated mode
    assert_eq!(gpu.view_mode, GpuViewMode::SimulatedFixture);
    for dev in &gpu.devices {
        assert!(dev.sm_utilization_pct >= 0.0 && dev.sm_utilization_pct <= 100.0);
    }
}

#[test]
fn test_unsupported_hardware_reporting() {
    let gpu = GpuAiTelemetry::default();
    assert_eq!(gpu.view_mode, GpuViewMode::HostReality);
    assert!(matches!(gpu.status, DataSourceStatus::Unsupported { .. }));
    assert!(gpu.devices.is_empty());

    let diag = DiagnosticsTelemetry::default();
    let ebpf_check = diag.checks.iter().find(|c| c.id == "DIAG-02");
    assert!(ebpf_check.is_some());
    let ebpf = ebpf_check.unwrap();
    assert_eq!(
        ebpf.status,
        lidb_tui_prototype::model::diagnostics::DiagnosticStatus::Unavailable
    );
}
