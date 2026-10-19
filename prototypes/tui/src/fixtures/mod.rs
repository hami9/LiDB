use crate::model::{
    cpu_mem::CpuMemoryTelemetry,
    diagnostics::DiagnosticsTelemetry,
    gpu_ai::{GpuAiTelemetry, GpuViewMode},
    network::NetworkTelemetry,
    processes::ProcessTelemetry,
    system::SystemTelemetry,
};

/// Deterministic, realistic telemetry simulator for development,
/// automated testing, and TUI presentation without requiring root or hardware probes.
#[derive(Debug, Default)]
pub struct FixtureManager {
    tick_count: u64,
    accumulated_subsecond_ms: u64,
}

impl FixtureManager {
    pub fn new() -> Self {
        Self {
            tick_count: 0,
            accumulated_subsecond_ms: 0,
        }
    }

    /// Advance simulated telemetry values by one tick interval.
    pub fn advance_tick(
        &mut self,
        system: &mut SystemTelemetry,
        cpu_mem: &mut CpuMemoryTelemetry,
        net: &mut NetworkTelemetry,
        gpu: &mut GpuAiTelemetry,
        proc: &mut ProcessTelemetry,
        tick_duration_ms: u64,
    ) {
        self.tick_count = self.tick_count.wrapping_add(1);
        let tick = self.tick_count as f32;

        // 1. Advance System uptime by elapsed refresh time (accumulating subsecond ms)
        self.accumulated_subsecond_ms = self
            .accumulated_subsecond_ms
            .saturating_add(tick_duration_ms);
        let elapsed_seconds = self.accumulated_subsecond_ms / 1000;
        if elapsed_seconds > 0 {
            system.uptime_seconds = system.uptime_seconds.saturating_add(elapsed_seconds);
            self.accumulated_subsecond_ms %= 1000;
        }
        system.load_average[0] = (1.10 + 0.15 * (tick * 0.1).sin()).max(0.1);
        system.load_average[1] = (0.95 + 0.08 * (tick * 0.05).sin()).max(0.1);

        // 2. Advance CPU and Memory
        let cpu_wave = 28.0 + 12.0 * (tick * 0.2).sin() + 5.0 * (tick * 0.5).cos();
        let cpu_val = cpu_wave.clamp(5.0, 95.0);
        cpu_mem.overall_cpu_pct = cpu_val;

        for (i, core) in cpu_mem.cores.iter_mut().enumerate() {
            let offset = i as f32 * 0.7;
            let usage = (cpu_val + 15.0 * (tick * 0.3 + offset).sin()).clamp(2.0, 99.0);
            core.usage_pct = usage;
        }

        // Keep CPU history bounded to 30 points
        cpu_mem.cpu_history.push(cpu_val as u64);
        if cpu_mem.cpu_history.len() > 30 {
            cpu_mem.cpu_history.remove(0);
        }

        // Memory variation
        let mem_used_gb = 6.2 + 0.5 * (tick * 0.05).sin();
        let mem_used_bytes = (mem_used_gb * 1024.0 * 1024.0 * 1024.0) as u64;
        cpu_mem.memory.used_bytes = mem_used_bytes;
        cpu_mem.memory.available_bytes = cpu_mem.memory.total_bytes.saturating_sub(mem_used_bytes);
        let mem_pct = (mem_used_bytes as f64 / cpu_mem.memory.total_bytes as f64 * 100.0) as u64;
        cpu_mem.mem_history.push(mem_pct);
        if cpu_mem.mem_history.len() > 30 {
            cpu_mem.mem_history.remove(0);
        }

        // 3. Advance Network metrics
        if let Some(eth0) = net.interfaces.iter_mut().find(|i| i.name == "eth0") {
            let rx_rate = (1_200_000.0 + 400_000.0 * (tick * 0.25).sin()) as u64;
            let tx_rate = (800_000.0 + 300_000.0 * (tick * 0.3).cos()) as u64;
            eth0.rx_bytes_sec = rx_rate;
            eth0.tx_bytes_sec = tx_rate;
            eth0.rx_packets_sec = rx_rate / 1400;
            eth0.tx_packets_sec = tx_rate / 1400;

            net.rx_history.push(rx_rate / 10_000);
            if net.rx_history.len() > 30 {
                net.rx_history.remove(0);
            }
            net.tx_history.push(tx_rate / 10_000);
            if net.tx_history.len() > 30 {
                net.tx_history.remove(0);
            }
        }

        // 4. Advance GPU if in SimulatedFixture view mode (DGX Spark GB10)
        if gpu.view_mode == GpuViewMode::SimulatedFixture {
            for (i, dev) in gpu.devices.iter_mut().enumerate() {
                let offset = i as f32 * 1.2;
                dev.sm_utilization_pct =
                    (75.0 + 15.0 * (tick * 0.15 + offset).sin()).clamp(10.0, 99.0);
                // Realistic power profile bounded by 140 W GB10 SoC TDP
                dev.power_watts = (115.0 + 10.0 * (tick * 0.2 + offset).cos()) as u32;
                dev.temperature_c = (62.0 + 3.0 * (tick * 0.1).sin()) as u32;

                // Coherent unified system memory variation
                let mem_gb = 84.0 + (i as f32 * 4.0) + 2.0 * (tick * 0.08 + offset).sin();
                dev.memory_used_bytes = (mem_gb * 1024.0 * 1024.0 * 1024.0) as u64;
            }

            for (j, wl) in gpu.workloads.iter_mut().enumerate() {
                let offset = j as f32 * 0.8;
                wl.tokens_per_sec = (1400.0 + 150.0 * (tick * 0.18 + offset).sin()).max(200.0);
                wl.kv_cache_usage_pct =
                    (65.0 + 8.0 * (tick * 0.08 + offset).cos()).clamp(20.0, 95.0);
            }
        }

        // 5. Advance Process metrics
        for p in proc.items.iter_mut() {
            if p.command.contains("vllm") {
                p.cpu_pct = (18.0 + 6.0 * (tick * 0.2).sin()).clamp(5.0, 45.0);
            } else if p.command.contains("lidb-tui") {
                p.cpu_pct = (2.5 + 0.8 * (tick * 0.4).cos()).clamp(1.0, 5.0);
            }
        }
    }
}

pub fn initial_fixtures() -> (
    SystemTelemetry,
    CpuMemoryTelemetry,
    NetworkTelemetry,
    GpuAiTelemetry,
    ProcessTelemetry,
    DiagnosticsTelemetry,
) {
    (
        SystemTelemetry::default(),
        CpuMemoryTelemetry::default(),
        NetworkTelemetry::default(),
        GpuAiTelemetry::default(),
        ProcessTelemetry::default(),
        DiagnosticsTelemetry::default(),
    )
}
