use super::DataSourceStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCoreMetrics {
    pub core_id: usize,
    pub frequency_mhz: u32,
    pub usage_pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub available_bytes: u64,
    pub buffers_bytes: u64,
    pub cached_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsiMetrics {
    pub cpu_some_avg10: f32,
    pub mem_some_avg10: f32,
    pub mem_full_avg10: f32,
    pub io_some_avg10: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMemoryTelemetry {
    pub overall_cpu_pct: f32,
    pub cores: Vec<CpuCoreMetrics>,
    pub memory: MemoryMetrics,
    pub psi: PsiMetrics,
    pub cpu_history: Vec<u64>,
    pub mem_history: Vec<u64>,
    pub status: DataSourceStatus,
}

impl Default for CpuMemoryTelemetry {
    fn default() -> Self {
        let cores = (0..8)
            .map(|i| CpuCoreMetrics {
                core_id: i,
                frequency_mhz: 2400,
                usage_pct: 12.0 + (i as f32 * 4.5),
            })
            .collect();

        Self {
            overall_cpu_pct: 27.5,
            cores,
            memory: MemoryMetrics {
                total_bytes: 16 * 1024 * 1024 * 1024,
                used_bytes: 6 * 1024 * 1024 * 1024,
                free_bytes: 4 * 1024 * 1024 * 1024,
                available_bytes: 9 * 1024 * 1024 * 1024,
                buffers_bytes: 512 * 1024 * 1024,
                cached_bytes: 5 * 1024 * 1024 * 1024,
                swap_total_bytes: 4 * 1024 * 1024 * 1024,
                swap_used_bytes: 128 * 1024 * 1024,
            },
            psi: PsiMetrics {
                cpu_some_avg10: 0.12,
                mem_some_avg10: 0.04,
                mem_full_avg10: 0.00,
                io_some_avg10: 0.08,
            },
            cpu_history: vec![20, 22, 25, 24, 28, 30, 27, 26, 29, 31, 28, 27],
            mem_history: vec![35, 36, 36, 37, 37, 38, 38, 37, 38, 39, 38, 38],
            status: DataSourceStatus::SimulatedFixture {
                fixture_name: "p0_cpu_mem_standard".to_string(),
            },
        }
    }
}
