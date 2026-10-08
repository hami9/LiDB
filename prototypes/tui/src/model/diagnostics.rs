use super::DataSourceStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticStatus {
    Pass,
    Warn,
    Fail,
    Unavailable,
}

impl DiagnosticStatus {
    pub fn badge(&self) -> &'static str {
        match self {
            Self::Pass => " PASS ",
            Self::Warn => " WARN ",
            Self::Fail => " FAIL ",
            Self::Unavailable => " N/A  ",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticCheck {
    pub id: String,
    pub name: String,
    pub category: String,
    pub status: DiagnosticStatus,
    pub finding: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsTelemetry {
    pub checks: Vec<DiagnosticCheck>,
    pub status: DataSourceStatus,
}

impl Default for DiagnosticsTelemetry {
    fn default() -> Self {
        let checks = vec![
            DiagnosticCheck {
                id: "DIAG-01".to_string(),
                name: "procfs Base Telemetry".to_string(),
                category: "Host Kernel".to_string(),
                status: DiagnosticStatus::Pass,
                finding:
                    "/proc/stat, /proc/meminfo, /proc/net/dev are readable without root privileges."
                        .to_string(),
                remediation: "None required. Safe read-only metrics collection enabled."
                    .to_string(),
            },
            DiagnosticCheck {
                id: "DIAG-02".to_string(),
                name: "eBPF Subsystem Access".to_string(),
                category: "Host Kernel".to_string(),
                status: DiagnosticStatus::Unavailable,
                finding: "bpf() syscall restricted in PRoot/unprivileged user namespace."
                    .to_string(),
                remediation: "Telemetry engine uses safe procfs/sysfs fallback per Rule R10."
                    .to_string(),
            },
            DiagnosticCheck {
                id: "DIAG-03".to_string(),
                name: "cgroup v2 Controllers".to_string(),
                category: "Resource Control".to_string(),
                status: DiagnosticStatus::Warn,
                finding: "Unified hierarchy detected but memory/io controllers not delegated."
                    .to_string(),
                remediation:
                    "Run in container with systemd cgroup delegation or use host cgroup mount."
                        .to_string(),
            },
            DiagnosticCheck {
                id: "DIAG-04".to_string(),
                name: "Linux Netlink Routing".to_string(),
                category: "Networking".to_string(),
                status: DiagnosticStatus::Pass,
                finding: "NETLINK_ROUTE socket readable for interface discovery and link state."
                    .to_string(),
                remediation: "None required. Route lookup functional.".to_string(),
            },
            DiagnosticCheck {
                id: "DIAG-05".to_string(),
                name: "NVML Hardware Runtime".to_string(),
                category: "Accelerators".to_string(),
                status: DiagnosticStatus::Unavailable,
                finding:
                    "libnvidia-ml.so and /dev/nvidia* device nodes absent in current environment."
                        .to_string(),
                remediation:
                    "Expected in PRoot container. Use --fixture-mode to simulate GPU dashboards."
                        .to_string(),
            },
            DiagnosticCheck {
                id: "DIAG-06".to_string(),
                name: "Security & Privacy Contract".to_string(),
                category: "Governance".to_string(),
                status: DiagnosticStatus::Pass,
                finding:
                    "Rule R07 verified: zero payload capture, no decrypted TLS or prompt snooping."
                        .to_string(),
                remediation: "Maintain strict metadata-only collection policy.".to_string(),
            },
            DiagnosticCheck {
                id: "DIAG-07".to_string(),
                name: "Unprivileged Execution".to_string(),
                category: "Security".to_string(),
                status: DiagnosticStatus::Pass,
                finding:
                    "Dashboard operates safely in user mode without CAP_SYS_ADMIN or CAP_NET_ADMIN."
                        .to_string(),
                remediation: "None required. Preserves principle of least privilege.".to_string(),
            },
        ];

        Self {
            checks,
            status: DataSourceStatus::SimulatedFixture {
                fixture_name: "p0_diagnostics_baseline".to_string(),
            },
        }
    }
}
