use crate::app::commands::servers::server_actions::ServerActions;
use crate::app::commands::servers::server_daemon::ServerDaemon;
use crate::app::commands::servers::server_info::ServerInfo;
use crate::cores::helper::hack::Hack;
use crate::cores::runner::console::{ConsoleArguments, ConsoleCommand, ConsoleResult};
use crate::cores::system::error::ResultError;
use crate::factory::cmd::Cmd;
use crate::factory::server_stats::ServerStats;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use clap::{ArgMatches, Command, CommandFactory, Parser};
use log::{debug, warn};
use ratatui::prelude::Color;
use rlimit::{Resource, getrlimit, setrlimit};

#[derive(Parser, Debug, Clone)]
#[clap(name = "command", about = "Server command")]
pub(crate) enum ServerCommand {
    #[clap(name = "start", about = "Start the server")]
    Start {
        #[arg(
            short = 'D',
            long = "daemonize",
            help = "Run the server in the background as a daemon process"
        )]
        daemonize: bool,
    },
    #[clap(name = "stop", about = "Stop the server")]
    Stop {
        #[arg(
            short = 'g',
            long = "gracefully",
            help = "Wait for active connections to finish before shutting down (default: false)"
        )]
        gracefully: bool,
    },
    #[clap(name = "info", about = "Get the server info")]
    Info,
    #[clap(name = "status", about = "Get the server status")]
    Status,
    #[clap(name = "clean-memory", about = "Cleanup memory")]
    CleanMemory,
    #[clap(name = "reload", about = "Reload the server")]
    Reload {
        #[arg(
            short = 'g',
            long = "gracefully",
            help = "Wait for active connections to finish before shutting down (default: false)"
        )]
        gracefully: bool,
    },
    #[clap(name = "restart", about = "Restart the server")]
    Restart {
        #[arg(
            short = 'F',
            long = "foreground",
            help = "Run the server in the foreground (default: false)"
        )]
        foreground: bool,
        #[arg(
            short = 'g',
            long = "gracefully",
            help = "Wait for active connections to finish before shutting down (default: false)"
        )]
        gracefully: bool,
    },
}

impl Default for ServerCommand {
    fn default() -> Self {
        Self::Start { daemonize: false }
    }
}

#[derive(Debug, Parser, Default)]
#[clap(name = "server", about = "Start the server")]
pub(crate) struct Server {
    #[command(flatten)]
    args: ConsoleArguments,
    #[command(subcommand)]
    command: ServerCommand,
}

impl Server {
    pub(crate) fn server_to_vector(
        info: &ServerStats,
        rata_tui: bool,
        connected: bool,
    ) -> Vec<(String, Color, Vec<(String, String)>)> {
        let ServerStats {
            system,
            app,
            network,
            stats,
            disks,
            cpu,
            memory,
        } = info;

        let start_time_data = Utc
            .timestamp_opt(system.start_time as i64, 0)
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S %Z")
            .to_string();

        let user_data = format!("{} ({})", system.user_name, system.user_id);
        let group_data = format!("{} ({})", system.group_name, system.group_id);
        let total_req_data = stats.total_request.to_string();
        let global_req_data = stats.total_global_request.to_string();
        let active_conn_data = stats.active_connections.to_string();
        let websocket_conn_data = stats.websocket_request.to_string();
        let active_websocket_conn_data = stats.active_websocket_connections.to_string();
        let pid = system.pid.to_string();
        let running = if !connected {
            "Disconnected".to_string()
        } else {
            if system.running {
                "Running".to_string()
            } else {
                "Stopped".to_string()
            }
        };
        let is_daemon = if system.is_daemon {
            "Daemonize"
        } else {
            "Foreground"
        };
        let mem_stat = memory.memory.clone();
        let mut systems = vec![
            ("PID".to_string(), format!("{} ({})", pid, is_daemon)),
            ("Status".to_string(), running),
            ("Start Time".to_string(), start_time_data),
            ("Thread Worker".to_string(), system.workers.to_string()),
            (
                "Current Time".to_string(),
                Utc::now().format("%Y-%m-%d %H:%M:%S %Z").to_string(),
            ),
            ("CPU Usage".to_string(), cpu.usage_percentage.to_string()),
            (
                "Server Uptime".to_string(),
                Hack::format_human_time_second(system.uptime as usize, true).to_string(),
            ),
            (
                "Peak RSS Memory".to_string(),
                Hack::format_size(mem_stat.vm_hwm).to_string(),
            ),
            (
                "RSS Memory".to_string(),
                Hack::format_size(mem_stat.vm_rss).to_string(),
            ),
            (
                "Used Memory".to_string(),
                Hack::format_size(mem_stat.rss_anon).to_string(),
            ),
        ];
        let mut net = vec![
            (
                "Upload".to_string(),
                format!("{}/s", Hack::format_size(network.upload)),
            ),
            (
                "Download".to_string(),
                format!("{}/s", Hack::format_size(network.download)),
            ),
            ("Socket".to_string(), network.socket.clone()),
            ("Master Control".to_string(), network.control_socket.clone()),
        ];
        if rata_tui {
            systems.extend(vec![
                ("User".to_string(), user_data),
                ("Group".to_string(), group_data),
            ]);
            let mut count = 1;
            for http in network.tcp.http.to_vec() {
                net.push((format!("TCP {} (HTTP)", count), http));
                count += 1;
            }
            for https in network.tcp.https.to_vec() {
                net.push((format!("TCP {} (HTTPS)", count), https));
                count += 1;
            }
        } else {
            systems.push((
                "User / Group".to_string(),
                format!("{} / {}", user_data, group_data),
            ));
            let mut str = Vec::new();
            if network.tcp.http.len() > 0 {
                str.push(format!("HTTP: {}", network.tcp.http.join(", ")))
            }
            if network.tcp.https.len() > 0 {
                str.push(format!("HTTPS: {}", network.tcp.https.join(", ")))
            }
            let tcp_data = str.join("\n");
            net.push(("TCP".to_string(), tcp_data));
        }
        if rata_tui {
            systems.extend([
                ("Backlog".to_string(), system.backlog.to_string()),
                (
                    "Max Connections".to_string(),
                    system.max_connections.to_string(),
                ),
                (
                    "Connection Rate".to_string(),
                    format!("{}/sec", system.connection_rate.to_string()),
                ),
            ]);
        }
        systems.push(("Root Directory".to_string(), system.root_dir.to_string()));
        if let Some(config_file) = &system.config_file {
            systems.push(("Config File".to_string(), config_file.clone()));
        }
        let mut info = vec![
            ("System".to_string(), Color::Yellow, systems),
            ("Network".to_string(), Color::Green, net),
            (
                "Stats".to_string(),
                Color::Magenta,
                vec![
                    ("Global Requests".to_string(), global_req_data),
                    ("Total Requests".to_string(), total_req_data),
                    ("Active connections".to_string(), active_conn_data),
                    ("Websocket Request".to_string(), websocket_conn_data),
                    ("Websocket Active".to_string(), active_websocket_conn_data),
                ],
            ),
        ];
        let mut mode = app.environment.clone();
        if system.is_cargo {
            mode = format!("{} (debug)", mode);
        }
        if rata_tui {
            let mut disks_vec: Vec<(String, String)> = vec![];
            for disk in disks {
                let mut label: Option<String> = None;
                let mut file_type: Option<String> = None;
                let mut mount_point: Option<String> = None;
                let mut readonly: Option<bool> = None;
                if let Some(mount) = &disk.device {
                    file_type = Some(mount.fs_type.to_string());
                    mount_point = Some(mount.mount_point.to_string());
                    readonly = Some(mount.is_readonly());
                    label = match &mount.label {
                        None => {
                            if mount.mount_point == "/" {
                                Some("*Root".to_string())
                            } else {
                                None
                            }
                        }
                        Some(e) => Some(e.label.to_string()),
                    }
                }
                let disk_label = format!(
                    "({}) {}",
                    file_type.unwrap_or_else(|| "?".to_string()),
                    disk.name
                );

                disks_vec.push((
                    disk_label,
                    format!(
                        "{:.2} / {:.2} GB",
                        disk.used as f64 / 1024.0 / 1024.0 / 1024.0,
                        disk.total as f64 / 1024.0 / 1024.0 / 1024.0,
                    ),
                ));
                disks_vec.push((
                    format!("  ├─ {}", label.unwrap_or_else(|| "?".to_string())),
                    format!("{}", mount_point.unwrap_or_else(|| "?".to_string()),),
                ));
                disks_vec.push((
                    format!(
                        "  └─ I/O {}",
                        match readonly {
                            None => "N/A".to_string(),
                            Some(e) => { if e { "R/O" } else { "R/W" } }.to_string(),
                        },
                    ),
                    format!(
                        "{}/s | {}/s",
                        Hack::format_size(disk.io_read),
                        Hack::format_size(disk.io_write)
                    ),
                ));
            }
            info.push(("Disk".to_string(), Color::Yellow, disks_vec));
            let hardware = vec![
                ("CPU Model".to_string(), cpu.model.to_string()),
                ("Logical Cores".to_string(), cpu.logical_cores.to_string()),
                ("Physical Cores".to_string(), cpu.physical_cores.to_string()),
                (
                    "CPU Usage".to_string(),
                    cpu.global_usage_percentage.to_string(),
                ),
                (
                    "Total Memory".to_string(),
                    Hack::format_size(memory.total_memory).to_string(),
                ),
                (
                    "Used Memory".to_string(),
                    Hack::format_size(memory.used_memory).to_string(),
                ),
                (
                    "Free Memory".to_string(),
                    Hack::format_size(memory.free_memory).to_string(),
                ),
                (
                    "Available Memory".to_string(),
                    Hack::format_size(memory.available_memory).to_string(),
                ),
            ];
            info.insert(0, ("Hardware".to_string(), Color::Cyan, hardware));
        }
        let mut build = vec![
            ("Name".to_string(), app.name.clone()),
            ("Mode".to_string(), mode),
            ("Version".to_string(), app.version.clone()),
            ("Date".to_string(), app.build_timestamp_date.clone()),
            (
                "Auto Clean Memory".to_string(),
                if app.auto_clean_memory {
                    "Enabled".to_string()
                } else {
                    "Disabled".to_string()
                },
            ),
        ];
        if app.auto_clean_memory {
            build.push((
                "Auto Clean Size".to_string(),
                Hack::format_size(app.auto_clean_memory_size),
            ));
            build.push((
                "Auto Clean Interval".to_string(),
                Hack::format_human_time_second(app.auto_clean_memory_interval, false),
            ));
            let next_auto_clean = app.next_auto_clean;
            if next_auto_clean < 0 {
                build.push(("Next Auto Clean".to_string(), "Never".to_string()));
            } else {
                build.push((
                    "Next Auto Clean".to_string(),
                    Hack::format_human_time_second(next_auto_clean as usize, false),
                ));
            }
        }
        info.insert(0, ("Build".to_string(), Color::Blue, build));
        info
    }
}

#[async_trait(?Send)]
impl ConsoleCommand for Server {
    fn get_command(&self) -> Command {
        <Self as CommandFactory>::command()
    }

    fn reconfigure(&mut self, _: &ConsoleArguments, _: ArgMatches, local: ArgMatches) {
        match self.parse_matches_arg(local) {
            Ok(clone) => {
                self.args = clone.args;
                self.command = clone.command;
            }
            Err(_) => {
                warn!(target: "app::commands", "Failed to reconfigure Server command with provided arguments");
            }
        }
    }
    fn get_console_arguments(&self) -> Option<ConsoleArguments> {
        Some(self.args.clone())
    }

    async fn execute(
        &self,
        global: &ConsoleArguments,
        _: ArgMatches,
        _: ArgMatches,
    ) -> ResultError<ConsoleResult> {
        let config = Cmd::console_config(global, self.get_console_arguments())?;
        let target_nofile = config.app().rlimit_nofile();
        match getrlimit(Resource::NOFILE) {
            Ok((soft, hard)) => {
                let soft = soft as usize;
                let hard = hard as usize;
                debug!(target: "app::commands", "Current process limits: soft={}, hard={}", soft, hard);
                if soft >= target_nofile {
                    debug!(target: "app::commands", "File descriptor limit is already sufficient ({}).", soft);
                } else {
                    let new_soft = if target_nofile > hard {
                        debug!(
                            "Target limit {} exceeds hard limit {}. Capping at hard limit.",
                            target_nofile, hard
                        );
                        hard
                    } else {
                        target_nofile
                    };
                    match setrlimit(Resource::NOFILE, new_soft as u64, hard as u64) {
                        Ok(_) => {
                            debug!(target: "app::commands", "Successfully increased NOFILE soft limit to {}.", new_soft)
                        }
                        Err(e) => warn!(
                            "Failed to set NOFILE limit: {}. Check system permissions.",
                            e
                        ),
                    }
                }
            }
            Err(e) => warn!(target: "app::commands", "Unable to retrieve current resource limits: {}", e),
        };
        Cmd::drop_privilege(&config.app().user())?;
        match self.command {
            ServerCommand::Start { daemonize } => ServerDaemon::start(config, daemonize).await,
            ServerCommand::Restart {
                foreground,
                gracefully,
            } => ServerDaemon::restart(config, foreground, gracefully).await,
            ServerCommand::Stop { gracefully } => ServerActions::stop(gracefully, false).await,
            ServerCommand::Status => ServerActions::status().await,
            ServerCommand::CleanMemory => ServerActions::clean_memory().await,
            ServerCommand::Reload { gracefully } => ServerActions::reload(gracefully).await,
            ServerCommand::Info => ServerInfo::run().await,
        }
    }
}
