use super::DataSourceStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub mac: String,
    pub ip: String,
    pub is_up: bool,
    pub mtu: u32,
    pub rx_bytes_sec: u64,
    pub tx_bytes_sec: u64,
    pub rx_packets_sec: u64,
    pub tx_packets_sec: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_dropped: u64,
    pub tx_dropped: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketStats {
    pub tcp_established: u32,
    pub tcp_listen: u32,
    pub tcp_time_wait: u32,
    pub udp_total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTelemetry {
    pub interfaces: Vec<NetworkInterface>,
    pub sockets: SocketStats,
    pub rx_history: Vec<u64>,
    pub tx_history: Vec<u64>,
    pub status: DataSourceStatus,
}

impl Default for NetworkTelemetry {
    fn default() -> Self {
        let interfaces = vec![
            NetworkInterface {
                name: "eth0".to_string(),
                mac: "02:42:ac:11:00:02".to_string(),
                ip: "172.17.0.2/16".to_string(),
                is_up: true,
                mtu: 1500,
                rx_bytes_sec: 1_240_000,
                tx_bytes_sec: 840_000,
                rx_packets_sec: 950,
                tx_packets_sec: 720,
                rx_errors: 0,
                tx_errors: 0,
                rx_dropped: 0,
                tx_dropped: 0,
            },
            NetworkInterface {
                name: "lo".to_string(),
                mac: "00:00:00:00:00:00".to_string(),
                ip: "127.0.0.1/8".to_string(),
                is_up: true,
                mtu: 65536,
                rx_bytes_sec: 45_000,
                tx_bytes_sec: 45_000,
                rx_packets_sec: 60,
                tx_packets_sec: 60,
                rx_errors: 0,
                tx_errors: 0,
                rx_dropped: 0,
                tx_dropped: 0,
            },
            NetworkInterface {
                name: "wlan0".to_string(),
                mac: "fa:16:3e:89:12:34".to_string(),
                ip: "192.168.1.150/24".to_string(),
                is_up: false,
                mtu: 1500,
                rx_bytes_sec: 0,
                tx_bytes_sec: 0,
                rx_packets_sec: 0,
                tx_packets_sec: 0,
                rx_errors: 0,
                tx_errors: 0,
                rx_dropped: 0,
                tx_dropped: 0,
            },
        ];

        Self {
            interfaces,
            sockets: SocketStats {
                tcp_established: 42,
                tcp_listen: 12,
                tcp_time_wait: 8,
                udp_total: 19,
            },
            rx_history: vec![120, 150, 180, 130, 210, 240, 190, 220, 260, 290, 270, 250],
            tx_history: vec![80, 95, 110, 85, 130, 150, 120, 140, 160, 175, 165, 155],
            status: DataSourceStatus::SimulatedFixture {
                fixture_name: "p0_network_fixture".to_string(),
            },
        }
    }
}
