use super::DataSourceStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    pub index: usize,
    pub name: String,
    pub architecture: String,
    pub sm_utilization_pct: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub temperature_c: u32,
    pub power_watts: u32,
    pub power_limit_watts: u32,
    pub nvlink_status: String,
    pub pcie_bandwidth_gb_s: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiWorkload {
    pub name: String,
    pub framework: String,
    pub tokens_per_sec: f32,
    pub ttft_ms: f32,
    pub tpot_ms: f32,
    pub kv_cache_usage_pct: f32,
    pub batch_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuViewMode {
    /// Authentic PRoot / unprivileged state showing unsupported hardware and reasons.
    HostReality,
    /// Simulated Grace Hopper / GB10 superpod layout for UI testing.
    SimulatedFixture,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuAiTelemetry {
    pub view_mode: GpuViewMode,
    pub devices: Vec<GpuDevice>,
    pub workloads: Vec<AiWorkload>,
    pub status: DataSourceStatus,
    pub fabric_topology_note: String,
}

impl Default for GpuAiTelemetry {
    fn default() -> Self {
        // By default, we present HostReality (unsupported in PRoot) with an option to toggle to fixture
        Self {
            view_mode: GpuViewMode::HostReality,
            devices: Vec::new(),
            workloads: Vec::new(),
            status: DataSourceStatus::Unsupported {
                reason: "PRoot container on Linux aarch64 has no NVML driver, /dev/nvidia*, or CUDA runtime. (Rule R10 & R19 compliant)".to_string(),
            },
            fabric_topology_note: "Per Rule R03: NVLink-C2C is intra-node only. Inter-node DGX fabric uses ConnectX Ethernet/RoCE.".to_string(),
        }
    }
}

impl GpuAiTelemetry {
    pub fn simulated_fixture() -> Self {
        let devices = vec![
            GpuDevice {
                index: 0,
                name: "NVIDIA GB10 Blackwell (Simulated Fixture)".to_string(),
                architecture: "Blackwell-Unified".to_string(),
                sm_utilization_pct: 78.4,
                memory_used_bytes: 92 * 1024 * 1024 * 1024,
                memory_total_bytes: 128 * 1024 * 1024 * 1024,
                temperature_c: 64,
                power_watts: 520,
                power_limit_watts: 700,
                nvlink_status: "Intra-node C2C Active (900 GB/s)".to_string(),
                pcie_bandwidth_gb_s: 64.0,
            },
            GpuDevice {
                index: 1,
                name: "NVIDIA GB10 Blackwell (Simulated Fixture)".to_string(),
                architecture: "Blackwell-Unified".to_string(),
                sm_utilization_pct: 82.1,
                memory_used_bytes: 94 * 1024 * 1024 * 1024,
                memory_total_bytes: 128 * 1024 * 1024 * 1024,
                temperature_c: 66,
                power_watts: 545,
                power_limit_watts: 700,
                nvlink_status: "Intra-node C2C Active (900 GB/s)".to_string(),
                pcie_bandwidth_gb_s: 64.0,
            },
        ];

        let workloads = vec![
            AiWorkload {
                name: "llama3-70b-instruct".to_string(),
                framework: "vLLM-0.6".to_string(),
                tokens_per_sec: 1420.5,
                ttft_ms: 18.2,
                tpot_ms: 6.8,
                kv_cache_usage_pct: 68.4,
                batch_size: 64,
            },
            AiWorkload {
                name: "mistral-large-serving".to_string(),
                framework: "TensorRT-LLM".to_string(),
                tokens_per_sec: 890.0,
                ttft_ms: 24.1,
                tpot_ms: 8.2,
                kv_cache_usage_pct: 54.0,
                batch_size: 32,
            },
        ];

        Self {
            view_mode: GpuViewMode::SimulatedFixture,
            devices,
            workloads,
            status: DataSourceStatus::SimulatedFixture {
                fixture_name: "p0_gb10_workload_fixture".to_string(),
            },
            fabric_topology_note: "SIMULATED: Blackwell intra-node NVLink C2C. DGX Spark inter-node uses ConnectX-8 Ethernet/RoCE.".to_string(),
        }
    }
}
