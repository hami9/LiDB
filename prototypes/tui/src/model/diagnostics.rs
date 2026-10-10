use super::DataSourceStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticStatus {
    Pass,
    SimulatedPass,
    Warn,
    Fail,
    Unavailable,
    NotProbed,
}

impl DiagnosticStatus {
    pub fn badge(&self) -> &'static str {
        match self {
            Self::Pass => " PASS ",
            Self::SimulatedPass => " SIM-PASS ",
            Self::Warn => " WARN ",
            Self::Fail => " FAIL ",
            Self::Unavailable => " N/A  ",
            Self::NotProbed => " UNPROBED ",
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
                status: DiagnosticStatus::NotProbed,
                finding:
                    "procfs base telemetry not probed in standalone TUI prototype; integration scheduled for P1."
                        .to_string(),
                remediation: "Read-only procfs collectors will run via lidashd or integrated engine in P1."
                    .to_string(),
            },
            DiagnosticCheck {
                id: "DIAG-02".to_string(),
                name: "eBPF Subsystem Access".to_string(),
                category: "Host Kernel".to_string(),
                status: DiagnosticStatus::Unavailable,
                finding: "bpf() syscall access not enabled in unprivileged standalone prototype."
                    .to_string(),
                remediation: "Telemetry engine uses safe procfs/sysfs fallback per Rule R10."
                    .to_string(),
            },
            DiagnosticCheck {
                id: "DIAG-03".to_string(),
                name: "cgroup v2 Controllers".to_string(),
                category: "Resource Control".to_string(),
                status: DiagnosticStatus::NotProbed,
                finding: "cgroup v2 controller delegation not probed in standalone prototype."
                    .to_string(),
                remediation:
                    "cgroup hierarchy discovery scheduled for P1 host telemetry."
                        .to_string(),
            },
            DiagnosticCheck {
                id: "DIAG-04".to_string(),
                name: "Linux Netlink Routing".to_string(),
                category: "Networking".to_string(),
                status: DiagnosticStatus::NotProbed,
                finding: "NETLINK_ROUTE interface/routing discovery not opened in standalone prototype."
                    .to_string(),
                remediation: "Netlink routing collector scheduled for P2.".to_string(),
            },
            DiagnosticCheck {
                id: "DIAG-05".to_string(),
                name: "NVML Hardware Runtime".to_string(),
                category: "Accelerators".to_string(),
                status: DiagnosticStatus::Unavailable,
                finding:
                    "NVML driver (/dev/nvidia*) capability probing not yet implemented in prototype."
                        .to_string(),
                remediation:
                    "Optional GPU adapter probe scheduled for P5; use 'g' to inspect simulated DGX Spark fixture."
                        .to_string(),
            },
            DiagnosticCheck {
                id: "DIAG-06".to_string(),
                name: "Security & Privacy Contract".to_string(),
                category: "Governance".to_string(),
                status: DiagnosticStatus::SimulatedPass,
                finding:
                    "SIMULATED CHECK: Architectural policy prohibits payload, key, and prompt capture (Rule R07). Runtime audit not executed."
                        .to_string(),
                remediation: "Maintain strict metadata-only collection policy across all collectors.".to_string(),
            },
            DiagnosticCheck {
                id: "DIAG-07".to_string(),
                name: "Unprivileged Execution".to_string(),
                category: "Security".to_string(),
                status: DiagnosticStatus::SimulatedPass,
                finding:
                    "SIMULATED CHECK: Process runs unprivileged without CAP_SYS_ADMIN/CAP_NET_ADMIN (Rule R05). Environment privileges not audited."
                        .to_string(),
                remediation: "Preserve principle of least privilege per Rule R05.".to_string(),
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
