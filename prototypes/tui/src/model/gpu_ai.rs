use super::DataSourceStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    pub index: usize,
    pub name: String,
    pub host_node: String,
    pub architecture: String,
    pub sm_utilization_pct: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_type: String,
    pub temperature_c: u32,
    pub power_watts: u32,
    pub power_limit_watts: u32,
    pub nvlink_status: String,
    pub inter_node_fabric: String,
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
    /// Default host state showing unprobed accelerator status pending capability detection.
    HostReality,
    /// Simulated DGX Spark (GB10 Grace Blackwell) 2-node cluster layout for UI testing.
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
        // Truthful unprobed default: do not hardcode PRoot assumptions or fake hardware absence
        Self {
            view_mode: GpuViewMode::HostReality,
            devices: Vec::new(),
            workloads: Vec::new(),
            status: DataSourceStatus::NotProbed {
                reason: "Accelerator and GPU driver capability detection not yet executed in standalone prototype. (P0 boundary)".to_string(),
            },
            fabric_topology_note: "Per Rule R03: NVLink-C2C is intra-node only. DGX Spark inter-node uses ConnectX-7 Ethernet/RoCE.".to_string(),
        }
    }
}

impl GpuAiTelemetry {
    pub fn simulated_fixture() -> Self {
        let devices = vec![
            GpuDevice {
                index: 0,
                name: "NVIDIA GB10 Grace Blackwell (Simulated Fixture)".to_string(),
                host_node: "spark-node-01".to_string(),
                architecture: "Grace Blackwell GB10 (Coherent UMA)".to_string(),
                sm_utilization_pct: 78.4,
                memory_used_bytes: 84 * 1024 * 1024 * 1024,
                memory_total_bytes: 128 * 1024 * 1024 * 1024,
                memory_type: "128 GB LPDDR5x Unified System Memory (273 GB/s)".to_string(),
                temperature_c: 62,
                power_watts: 115,
                power_limit_watts: 140, // 140 W GB10 SoC TDP (240 W system PSU)
                nvlink_status: "Intra-node NVLink-C2C Active (900 GB/s)".to_string(),
                inter_node_fabric: "ConnectX-7 200GbE RoCEv2 (QSFP)".to_string(),
                pcie_bandwidth_gb_s: 64.0,
            },
            GpuDevice {
                index: 1,
                name: "NVIDIA GB10 Grace Blackwell (Simulated Fixture)".to_string(),
                host_node: "spark-node-02".to_string(),
                architecture: "Grace Blackwell GB10 (Coherent UMA)".to_string(),
                sm_utilization_pct: 82.1,
                memory_used_bytes: 88 * 1024 * 1024 * 1024,
                memory_total_bytes: 128 * 1024 * 1024 * 1024,
                memory_type: "128 GB LPDDR5x Unified System Memory (273 GB/s)".to_string(),
                temperature_c: 64,
                power_watts: 122,
                power_limit_watts: 140, // 140 W GB10 SoC TDP (240 W system PSU)
                nvlink_status: "Intra-node NVLink-C2C Active (900 GB/s)".to_string(),
                inter_node_fabric: "ConnectX-7 200GbE RoCEv2 (QSFP)".to_string(),
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
            fabric_topology_note: "SIMULATED 2-NODE CLUSTER: Each DGX Spark node has 1x GB10 with NVLink-C2C intra-node. Inter-node fabric: ConnectX-7 200GbE QSFP (RoCEv2). No external GPU-to-GPU NVLink.".to_string(),
        }
    }
}
