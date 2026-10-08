use super::DataSourceStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessSortField {
    Pid,
    Cpu,
    Mem,
    Command,
}

impl ProcessSortField {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pid => "PID",
            Self::Cpu => "CPU %",
            Self::Mem => "MEM %",
            Self::Command => "COMMAND",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessItem {
    pub pid: u32,
    pub user: String,
    pub cpu_pct: f32,
    pub mem_pct: f32,
    pub rss_bytes: u64,
    pub state: String,
    pub command: String,
    pub threads: u32,
    pub io_read_kb_s: u32,
    pub io_write_kb_s: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTelemetry {
    pub items: Vec<ProcessItem>,
    pub sort_field: ProcessSortField,
    pub sort_ascending: bool,
    pub filter: String,
    pub status: DataSourceStatus,
}

impl Default for ProcessTelemetry {
    fn default() -> Self {
        let items = vec![
            ProcessItem {
                pid: 1,
                user: "root".to_string(),
                cpu_pct: 0.1,
                mem_pct: 0.2,
                rss_bytes: 32 * 1024 * 1024,
                state: "S".to_string(),
                command: "systemd --system --deserialize 34".to_string(),
                threads: 1,
                io_read_kb_s: 0,
                io_write_kb_s: 4,
            },
            ProcessItem {
                pid: 412,
                user: "root".to_string(),
                cpu_pct: 18.4,
                mem_pct: 4.8,
                rss_bytes: 820 * 1024 * 1024,
                state: "R".to_string(),
                command: "/usr/bin/vllm-serve --model /models/llama3".to_string(),
                threads: 32,
                io_read_kb_s: 120,
                io_write_kb_s: 85,
            },
            ProcessItem {
                pid: 890,
                user: "lidb".to_string(),
                cpu_pct: 4.2,
                mem_pct: 1.1,
                rss_bytes: 180 * 1024 * 1024,
                state: "S".to_string(),
                command: "lidashd --config /etc/lidb/lidashd.toml".to_string(),
                threads: 8,
                io_read_kb_s: 45,
                io_write_kb_s: 60,
            },
            ProcessItem {
                pid: 1204,
                user: "lidb".to_string(),
                cpu_pct: 2.8,
                mem_pct: 0.8,
                rss_bytes: 128 * 1024 * 1024,
                state: "R".to_string(),
                command: "lidb-tui-prototype --theme dark".to_string(),
                threads: 4,
                io_read_kb_s: 12,
                io_write_kb_s: 2,
            },
            ProcessItem {
                pid: 1420,
                user: "postgres".to_string(),
                cpu_pct: 1.5,
                mem_pct: 2.4,
                rss_bytes: 400 * 1024 * 1024,
                state: "S".to_string(),
                command: "postgres: checkpointer process".to_string(),
                threads: 1,
                io_read_kb_s: 4,
                io_write_kb_s: 180,
            },
            ProcessItem {
                pid: 1894,
                user: "root".to_string(),
                cpu_pct: 0.0,
                mem_pct: 0.4,
                rss_bytes: 64 * 1024 * 1024,
                state: "S".to_string(),
                command: "/usr/sbin/sshd -D".to_string(),
                threads: 1,
                io_read_kb_s: 0,
                io_write_kb_s: 0,
            },
            ProcessItem {
                pid: 2210,
                user: "redis".to_string(),
                cpu_pct: 0.8,
                mem_pct: 1.5,
                rss_bytes: 250 * 1024 * 1024,
                state: "S".to_string(),
                command: "redis-server 127.0.0.1:6379".to_string(),
                threads: 4,
                io_read_kb_s: 15,
                io_write_kb_s: 20,
            },
        ];

        Self {
            items,
            sort_field: ProcessSortField::Cpu,
            sort_ascending: false,
            filter: String::new(),
            status: DataSourceStatus::SimulatedFixture {
                fixture_name: "p0_process_sample".to_string(),
            },
        }
    }
}
