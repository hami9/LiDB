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

    // Advance 10 ticks with 1000ms duration each
    for _ in 0..10 {
        fixture.advance_tick(
            &mut system,
            &mut cpu_mem,
            &mut net,
            &mut gpu,
            &mut proc,
            1000,
        );
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
        assert!(dev.power_watts <= dev.power_limit_watts);
    }
}

#[test]
fn test_unsupported_hardware_reporting() {
    let gpu = GpuAiTelemetry::default();
    assert_eq!(gpu.view_mode, GpuViewMode::HostReality);
    assert!(matches!(gpu.status, DataSourceStatus::NotProbed { .. }));
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

#[test]
fn test_subsecond_uptime_accumulation() {
    let mut fixture = FixtureManager::new();
    let mut system = SystemTelemetry::default();
    let mut cpu_mem = CpuMemoryTelemetry::default();
    let mut net = NetworkTelemetry::default();
    let mut gpu = GpuAiTelemetry::simulated_fixture();
    let mut proc = ProcessTelemetry::default();

    let initial_uptime = system.uptime_seconds;

    // 4 ticks of 250 ms should increment uptime_seconds by 1
    for _ in 0..4 {
        fixture.advance_tick(
            &mut system,
            &mut cpu_mem,
            &mut net,
            &mut gpu,
            &mut proc,
            250,
        );
    }
    assert_eq!(system.uptime_seconds, initial_uptime + 1);

    // Another 2 ticks of 250 ms: accumulated 500 ms, uptime_seconds unchanged
    for _ in 0..2 {
        fixture.advance_tick(
            &mut system,
            &mut cpu_mem,
            &mut net,
            &mut gpu,
            &mut proc,
            250,
        );
    }
    assert_eq!(system.uptime_seconds, initial_uptime + 1);

    // Another 2 ticks of 250 ms: accumulated 1000 ms, uptime_seconds increments to initial + 2
    for _ in 0..2 {
        fixture.advance_tick(
            &mut system,
            &mut cpu_mem,
            &mut net,
            &mut gpu,
            &mut proc,
            250,
        );
    }
    assert_eq!(system.uptime_seconds, initial_uptime + 2);
}

#[test]
fn test_dgx_spark_gb10_specifications() {
    let gpu = GpuAiTelemetry::simulated_fixture();

    assert_eq!(gpu.devices.len(), 2);
    assert_eq!(gpu.devices[0].host_node, "spark-node-01");
    assert_eq!(gpu.devices[1].host_node, "spark-node-02");

    for dev in &gpu.devices {
        // Architecture must be Grace Blackwell, not Grace Hopper
        assert!(dev.architecture.contains("Grace Blackwell"));
        assert!(!dev.architecture.contains("Grace Hopper"));

        // Unified memory pool: LPDDR5x 128 GB, not dedicated HBM3e
        assert!(dev.memory_type.contains("LPDDR5x"));
        assert!(dev.memory_type.contains("Unified"));
        assert!(!dev.memory_type.contains("HBM3e"));
        assert_eq!(dev.memory_total_bytes, 128 * 1024 * 1024 * 1024);

        // Power envelope: 140 W SoC TDP
        assert_eq!(dev.power_limit_watts, 140);
        assert!(dev.power_watts <= 140);

        // Interconnect: NVLink-C2C intra-node only
        assert!(dev.nvlink_status.contains("Intra-node NVLink-C2C"));

        // Inter-node fabric: ConnectX-7 200GbE RoCEv2, not external NVLink
        assert!(dev.inter_node_fabric.contains("ConnectX-7"));
        assert!(dev.inter_node_fabric.contains("RoCE"));
    }

    // Fabric topology note
    assert!(gpu.fabric_topology_note.contains("ConnectX-7"));
    assert!(gpu
        .fabric_topology_note
        .contains("No external GPU-to-GPU NVLink"));
}

#[test]
fn test_diagnostics_truthful_states() {
    use lidb_tui_prototype::model::diagnostics::DiagnosticStatus;

    let diag = DiagnosticsTelemetry::default();

    // Verify NO unverified hardcoded "Pass" exists
    for check in &diag.checks {
        assert_ne!(
            check.status,
            DiagnosticStatus::Pass,
            "Check {} must not claim verified Pass without actual runtime audit",
            check.id
        );
    }

    // Verify NotProbed checks
    let not_probed_ids = ["DIAG-01", "DIAG-03", "DIAG-04"];
    for id in not_probed_ids {
        let check = diag.checks.iter().find(|c| c.id == id).unwrap();
        assert_eq!(check.status, DiagnosticStatus::NotProbed);
    }

    // Verify Unavailable checks
    let unavail_ids = ["DIAG-02", "DIAG-05"];
    for id in unavail_ids {
        let check = diag.checks.iter().find(|c| c.id == id).unwrap();
        assert_eq!(check.status, DiagnosticStatus::Unavailable);
    }

    // Verify SimulatedPass checks have explicit simulated notes
    let sim_pass_ids = ["DIAG-06", "DIAG-07"];
    for id in sim_pass_ids {
        let check = diag.checks.iter().find(|c| c.id == id).unwrap();
        assert_eq!(check.status, DiagnosticStatus::SimulatedPass);
        assert!(check.finding.contains("SIMULATED CHECK"));
    }
}
