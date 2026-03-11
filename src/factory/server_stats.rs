use crate::cores::proc::cpu_info::CpuInfo;
use crate::cores::proc::stat::Stat;
use crate::cores::proc::status::{MemoryStatus, Status};
use crate::cores::system::runtime::Runtime;
use crate::cores::system::stats::{Disk, Stats, StatsRecord};
use crate::factory::config::{
    default_auto_clean_memory, default_backlog, default_max_connections,
    default_max_connections_rate,
};
use crate::factory::constant::{
    DEFAULT_AUTO_CLEAN_MEMORY_INTERVAL, DEFAULT_AUTO_CLEAN_MEMORY_SIZE_BYTES,
};
use crate::factory::server::{Server, TcpSocket};
use nix::unistd::Group;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct CpuStat {
    pub logical_cores: usize,
    pub physical_cores: usize,
    pub model: String,
    pub freq: String,
    pub global_usage_percentage: String,
    pub usage_percentage: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct MemoryStat {
    pub total_memory: usize,
    pub free_memory: usize,
    pub available_memory: usize,
    pub used_memory: usize,
    pub memory: MemoryStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct ServerSystemInfo {
    pub is_cargo: bool,
    pub is_daemon: bool,
    pub running: bool,
    pub pid: u32,
    pub start_time: u64,
    pub uptime: u64,
    pub user_name: String,
    pub user_id: usize,
    pub group_name: String,
    pub group_id: usize,
    pub workers: usize,
    pub backlog: usize,
    pub connection_rate: usize,
    pub max_connections: usize,
    pub root_dir: String,
    pub config_file: Option<String>,
    pub server_start_counter: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct NetWorkInfo {
    pub tcp: TcpSocket,
    pub socket: String,
    pub control_socket: String,
    pub upload: usize,
    pub download: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Statistics {
    pub total_request: usize,
    pub total_global_request: usize,
    pub active_connections: usize,
    pub websocket_request: usize,
    pub active_websocket_connections: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub environment: String,
    pub version_full: String,
    pub build_timestamp: String,
    pub build_timestamp_date: String,
    pub auto_clean_memory: bool,
    pub auto_clean_memory_size: usize,
    pub auto_clean_memory_interval: usize,
    pub next_auto_clean: isize,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct ServerStats {
    pub app: AppInfo,
    pub memory: MemoryStat,
    pub cpu: CpuStat,
    pub system: ServerSystemInfo,
    pub network: NetWorkInfo,
    pub stats: Statistics,
    pub disks: Vec<Disk>,
}

impl ServerStats {
    pub fn from_server(server: &Server) -> Self {
        let current_user = nix::unistd::getuid();
        let current_group = nix::unistd::getgid();
        let user = nix::unistd::User::from_uid(current_user).unwrap().unwrap();
        let group = Group::from_gid(current_group).unwrap().unwrap();
        let socket_tcp: TcpSocket;
        if let Some(tcps) = server.get_tcp_socket() {
            socket_tcp = tcps;
        } else {
            let tcp_socket = server
                .get_current_config()
                .map(|c| c.app().tcp().to_vec())
                .unwrap_or_else(|| vec![]);
            let ssl_socket = server
                .get_current_config()
                .map(|c| c.app().ssl().listen().to_vec())
                .unwrap_or_else(|| vec![]);
            socket_tcp = TcpSocket {
                http: tcp_socket,
                https: ssl_socket,
            }
        }
        let mut network = NetWorkInfo {
            tcp: socket_tcp.clone(),
            socket: "".to_string(),
            control_socket: "".to_string(),
            upload: 0,
            download: 0,
        };
        let server_start_counter = server.get_start_counter();
        let mut max_connections = default_max_connections();
        let mut backlog = default_backlog();
        let mut connection_rate = default_max_connections_rate();
        let mut config_file = None;
        let mut auto_clean_memory = default_auto_clean_memory();
        let mut auto_clean_memory_size = DEFAULT_AUTO_CLEAN_MEMORY_SIZE_BYTES;
        let mut auto_clean_memory_interval = DEFAULT_AUTO_CLEAN_MEMORY_INTERVAL;
        if let Some(config) = server.get_current_config() {
            max_connections = config.app().max_connections();
            backlog = config.app().backlog();
            connection_rate = config.app().max_connections_rate();
            config_file = config.file().to_string_lossy().to_string().into();
            auto_clean_memory = config.app().auto_clean_memory();
            auto_clean_memory_size = config.app().auto_clean_memory_size_bytes();
            auto_clean_memory_interval =
                config.app().auto_clean_memory_interval_duration().as_secs() as usize;
            network = NetWorkInfo {
                tcp: socket_tcp,
                socket: config.get_socket().unwrap_or_else(|_| "".to_string()),
                control_socket: config.get_datagram_server_identity().to_string(),
                upload: 0,
                download: 0,
            };
        }
        let uptime = server.uptime();
        let start_time = server.start_time();

        let total_global_request = server.get_global_requests();
        let total_request = server.get_total_requests();
        let active_connections = server.get_active_connections();
        let websocket_request = server.get_websocket_request();
        let active_websocket_connections = server.get_active_websocket();
        let StatsRecord {
            network_rx,
            network_tx,
            disks,
            cpu_percentage,
            ..
        } = Stats::statistic();
        network.download = network_rx;
        network.upload = network_tx;
        CpuInfo::refresh();
        let one_second_duration = Duration::from_secs(1);
        let app_usage_percentage = Stat::with_duration(one_second_duration).cpu_usage;
        let cpu = CpuInfo::get();
        let (used_memory, free_memory, available_memory, total_memory) =
            Stats::memory_statistic(Duration::from_millis(250));
        let status = Status::with_duration(Duration::from_millis(100));
        Self {
            app: AppInfo {
                name: Runtime::app_name().to_string(),
                version: Runtime::app_version().to_string(),
                environment: server
                    .get_current_config()
                    .map(|c| c.app().mode().to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
                version_full: Runtime::app_version_full().to_string(),
                build_timestamp: Runtime::app_build_timestamp().to_string(),
                build_timestamp_date: Runtime::app_build_timestamp_date().to_string(),
                auto_clean_memory,
                auto_clean_memory_size,
                auto_clean_memory_interval,
                next_auto_clean: server.get_next_auto_clean(),
            },
            memory: MemoryStat {
                total_memory,
                free_memory,
                available_memory,
                used_memory,
                memory: status.memory,
            },
            cpu: CpuStat {
                logical_cores: cpu.logical_cores(),
                physical_cores: cpu.physical_cores(),
                model: cpu.brand().unwrap_or_else(|| "N/A".to_string()),
                freq: cpu
                    .cpu_freq_ghz()
                    .unwrap_or_else(|| format!("{:.2} GHz", "N/A")),
                global_usage_percentage: format!("{:.2}%", cpu_percentage),
                usage_percentage: format!(
                    "{:.2}%",
                    app_usage_percentage / cpu.logical_cores() as f64
                ),
            },
            system: ServerSystemInfo {
                is_cargo: Runtime::is_cargo_run(),
                is_daemon: Runtime::is_daemon(),
                running: server.is_running(),
                pid: Runtime::pid(),
                start_time,
                uptime,
                user_name: user.name,
                user_id: user.uid.as_raw() as usize,
                group_name: group.name,
                group_id: group.gid.as_raw() as usize,
                workers: server.get_total_worker(),
                backlog: backlog as usize,
                connection_rate,
                max_connections,
                root_dir: Runtime::root_dir().to_string_lossy().to_string(),
                config_file,
                server_start_counter,
            },
            network,
            stats: Statistics {
                total_request,
                total_global_request,
                active_connections,
                websocket_request,
                active_websocket_connections,
            },
            disks,
        }
    }
}
