use super::DataSourceStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTelemetry {
    pub hostname: String,
    pub os_name: String,
    pub kernel_version: String,
    pub architecture: String,
    pub uptime_seconds: u64,
    pub load_average: [f32; 3],
    pub proot_detected: bool,
    pub total_cpus: usize,
    pub total_memory_bytes: u64,
    pub lidb_version: String,
    pub lidb_agent: String,
    pub status: DataSourceStatus,
    pub status_message: String,
}

impl Default for SystemTelemetry {
    fn default() -> Self {
        Self {
            hostname: "lidb-node-01".to_string(),
            os_name: "Ubuntu 26.04.1 LTS".to_string(),
            kernel_version: "6.17.0-lidb".to_string(),
            architecture: "aarch64".to_string(),
            uptime_seconds: 142560,
            load_average: [1.14, 0.98, 0.85],
            proot_detected: false,
            total_cpus: 8,
            total_memory_bytes: 16 * 1024 * 1024 * 1024,
            lidb_version: "0.1.0-alpha.1".to_string(),
            lidb_agent: "antigravity/p0-tui-laptop".to_string(),
            status: DataSourceStatus::SimulatedFixture {
                fixture_name: "p0_standard_linux".to_string(),
            },
            status_message: "Telemetry simulated via P0 fixture generator. Capability detection not yet implemented in standalone prototype.".to_string(),
        }
    }
}
