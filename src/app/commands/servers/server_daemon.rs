use crate::app::commands::servers::server_actions::ServerActions;
use crate::cores::proc::status::{MemoryStatus, Status};
use crate::cores::runner::console::ConsoleResult;
use crate::cores::system::commander::{ControlCommand, ControlResponder, Datagram, ParserInto};
use crate::cores::system::error::{Error, ErrorType, ResultError};
use crate::cores::system::runtime::Runtime;
use crate::factory::cmd::Cmd;
use crate::factory::config::Config;
use crate::factory::factory::Factory;
use crate::factory::server::Server;
use crate::factory::server_stats::ServerStats;
use chrono::Timelike;
use log::{debug, info, trace, warn};
use nix::libc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::os::unix::net::UnixDatagram as UdGram;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::net::UnixDatagram;
use tokio::task;

#[derive(Debug, Default)]
pub(crate) struct ServerDaemon {
    mutex: Arc<Mutex<Option<ResultError<()>>>>,
    control_address: Mutex<Option<Arc<UnixDatagram>>>,
}

impl ServerDaemon {
    pub(crate) async fn restart(
        config: Arc<Config>,
        foreground: bool,
        gracefully: bool,
    ) -> ResultError<ConsoleResult> {
        println!("Restarting server ...");
        ServerActions::stop(gracefully, true).await?;
        // execute current binary
        Ok(ServerDaemon::start(config, !foreground).await?)
    }

    pub(crate) async fn start(config: Arc<Config>, daemonize: bool) -> ResultError<ConsoleResult> {
        let instance = Arc::new(Self::default());

        if daemonize {
            let config_clone = config.clone();
            let arc = Arc::new(Mutex::new(None));
            let arc_clone = arc.clone();
            let pid_arc = Arc::new(Mutex::new(None::<i32>));
            unsafe {
                let pid = libc::fork();
                if pid < 0 {
                    return Err(Error::from_io_error(std::io::Error::last_os_error()));
                }
                if pid > 0 {
                    // parent
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    if let Some(Err(e)) = arc.lock().take() {
                        return Err(e);
                    }
                    if let Some(pid) = pid_arc.lock().take() {
                        info!(target: "app::commands", "Server daemon started with PID: {}", pid);
                    }
                    return Ok(ConsoleResult::Ok);
                }
                // child
                libc::setsid(); // create a process group
                // fork again to make sure leased (Double Fork)
                let pid2 = libc::fork();
                if pid2 < 0 {
                    libc::_exit(1);
                }
                if pid2 > 0 {
                    libc::_exit(0);
                }
                let pid = libc::getpid();
                *pid_arc.lock() = Some(pid);
                Cmd::set_damon_name("daemon server", Some(pid), Some(&config));
                // spawning
                let handle = std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    rt.block_on(async move {
                        if let Ok(dev_null) = std::fs::File::open("/dev/null") {
                            let fd = std::os::unix::io::AsRawFd::as_raw_fd(&dev_null);
                            libc::dup2(fd, libc::STDIN_FILENO);
                            libc::dup2(fd, libc::STDOUT_FILENO);
                            libc::dup2(fd, libc::STDERR_FILENO);
                        }
                        let e = instance.start_daemon(config_clone).await;
                        let mut arc_clone = arc_clone.lock();
                        *arc_clone = Some(e);
                    });
                });
                let _ = handle.join();
                if let Some(Err(e)) = arc.lock().take() {
                    return Err(e);
                }
                libc::_exit(0);
            }
        } else {
            Cmd::set_damon_name("server", None, Some(&config));
            instance.start_daemon(config).await
        }
    }
    async fn start_daemon(self: Arc<Self>, config: Arc<Config>) -> ResultError<ConsoleResult> {
        let conf = config.clone();
        let mut option_control = self.control_address.lock();
        if option_control.is_some() {
            warn!(target: "app::commands", "Server daemon is already running!");
            return Ok(ConsoleResult::Canceled);
        }
        let arc_server = Factory::pick::<Server>()?;
        Server::socket_path_info(&config)?;
        let server_dgram_addr = config.create_datagram_server_address()?;
        if let Ok(test_socket) = UdGram::unbound() {
            if test_socket.connect_addr(&server_dgram_addr).is_ok() {
                if test_socket.send(&[]).is_ok() {
                    return Err(Error::address_in_use(
                        "Command -> Server is already running!",
                    ));
                }
            }
        }
        let sync_dgram = UdGram::bind_addr(&server_dgram_addr).map_err(Error::from)?;
        sync_dgram.set_nonblocking(true)?;

        let async_control = UnixDatagram::from_std(sync_dgram)?;
        let arc = Arc::new(async_control);
        *option_control = Some(arc.clone());
        let arc_server = arc_server.clone();
        let should_exit = Arc::new(AtomicBool::new(false));
        let should_skip_loop = Arc::new(AtomicBool::new(false));
        let late_should_skip_loop = should_skip_loop.clone();
        let local = task::LocalSet::new();
        let this = self.clone();
        let dgram = Datagram::new(arc.clone());
        let server_mutex = Arc::clone(&self.mutex);
        let final_mutex = Arc::clone(&server_mutex);
        local
            .run_until(async move {
                let runner_server = Arc::clone(&arc_server); // Clone for the runner task
                let server_runner = Arc::clone(&arc_server);
                let runner_control = Arc::clone(&arc_server);
                let exit_flag = should_exit.clone();
                let late_flag = should_exit.clone();
                let ctrl_c_flag = should_exit.clone();
                let mut c_conf = conf.clone();

                // CONTROL DATAGRAM LOOP
                let control_task = task::spawn_local({
                    async move {
                        loop {
                            if exit_flag.load(Ordering::SeqCst) {
                                debug!(target: "app::commands", "[DATAGRAM] Exiting control socket loop...");
                                break;
                            }
                            let mut buff = [0u8; 512];
                            let recv = tokio::time::timeout(
                                Duration::from_millis(500),
                                dgram.recv_from(&mut buff),
                            ).await;
                            if let Ok(Ok((size, src_addr))) = recv {
                                if size < 14 {
                                    continue;
                                }
                                let ControlCommand { id, command, message: cmd_message } = match dgram.parse_into::<ControlCommand>(&buff, size) {
                                    Ok(res) => res,
                                    Err(e) => {
                                        warn!(target: "app::commands", "[DATAGRAM] Failed to parse control command: {}", e);
                                        continue
                                    }
                                };
                                trace!(target: "app::commands", "[DATAGRAM] Received command from {:?} (id: {}, command: {})", src_addr, id, command);
                                match format!("{}", command).as_str() {
                                    "stop" => {
                                        info!(target: "app::commands", "[DATAGRAM] Received stop command from {:?}", src_addr);
                                        should_skip_loop.store(true, Ordering::SeqCst);
                                        let gracefully = if let Some(txt) = cmd_message {
                                            txt == "gracefully".to_string()
                                        } else {
                                            false
                                        };
                                        if gracefully {
                                            dgram.send_object_to(&src_addr, ControlResponder {
                                                command: command.clone(),
                                                id: id.clone(),
                                                status: 0,
                                                message: format!("Stopping server with pid: {} gracefully", Runtime::pid()),
                                            }).await.ok();
                                        }
                                        runner_control.stop_action(gracefully).await;
                                        match dgram.send_object_to(&src_addr, ControlResponder {
                                            command,
                                            id,
                                            status: 0,
                                            message: format!("Server with pid {} stopped", Runtime::pid()),
                                        }).await {
                                            Ok(_) => {}
                                            Err(e) => {
                                                warn!(target: "app::commands", "Error while reply to {:?}: {}", src_addr, e)
                                            }
                                        }
                                        exit_flag.store(true, Ordering::SeqCst);
                                        break;
                                    }
                                    "reload" => {
                                        info!(target: "app::commands", "[DATAGRAM] Received reload command from {:?}", src_addr);
                                        let config_file = c_conf.file();
                                        if !config_file.exists() {
                                            dgram.send_object_to(
                                                &src_addr,
                                                ControlResponder {
                                                    command,
                                                    id,
                                                    status: 1,
                                                    message: format!("Configuration file does not exists: {}", config_file.display()),
                                                },
                                            ).await.ok();
                                            continue;
                                        }
                                        info!(target: "app::commands", "[DATAGRAM] Reloading configuration file: {}", config_file.display());
                                        let conf = Config::load_from_file(config_file);
                                        let conf = match conf {
                                            Ok(c) => Arc::new(c),
                                            Err(e) => {
                                                dgram.send_object_to(
                                                    &src_addr,
                                                    ControlResponder {
                                                        command,
                                                        id,
                                                        status: 1,
                                                        message: format!("Failed to load configuration file {}: {}", config_file.display(), e),
                                                    },
                                                ).await.ok();
                                                continue;
                                            }
                                        };
                                        c_conf = conf;
                                        let cloned = c_conf.clone();
                                        let arc_wait = Arc::new(AtomicBool::new(true));
                                        let spawn_clone = arc_wait.clone();
                                        let arc_err = Arc::new(Mutex::new(None::<Error>));
                                        let spawn_mutex = arc_err.clone();
                                        let restart_handle = Arc::clone(&runner_control);
                                        let mutex_server = Arc::clone(&server_mutex);
                                        should_skip_loop.store(true, Ordering::SeqCst);
                                        task::spawn_local(
                                            async move {
                                                let res = match restart_handle.restart(cloned).await {
                                                    Ok(e) => Ok(e),
                                                    Err(e) => {
                                                        warn!(target: "app::commands", "Failed to restart server: {}", e);
                                                        let mut m = spawn_mutex.lock();
                                                        *m = Some(e.clone());
                                                        spawn_clone.store(false, Ordering::SeqCst);
                                                        Err(e)
                                                    }
                                                };
                                                {
                                                    let mut h = mutex_server.lock();
                                                    *h = Some(res);
                                                }
                                            }
                                        );
                                        tokio::time::sleep(Duration::from_millis(500)).await;
                                        if arc_wait.load(Ordering::SeqCst) {
                                            dgram.send_object_to(&src_addr, ControlResponder {
                                                command,
                                                id,
                                                status: if runner_control.is_running() { 0 } else { 1 },
                                                message: if runner_control.is_running() {
                                                    format!("Server pid: {} reloaded", Runtime::pid())
                                                } else {
                                                    "Server is not running, but reload already initiated".to_string()
                                                },
                                            }).await.ok();
                                        } else {
                                            let err = arc_err.lock().take();
                                            if let Some(e) = err {
                                                dgram.send_object_to(&src_addr, ControlResponder {
                                                    command,
                                                    id,
                                                    status: 1,
                                                    message: if e.error_type == ErrorType::AddrInUse {
                                                        "Server is already running".to_string()
                                                    } else {
                                                        format!("Failed reload: {}", e)
                                                    },
                                                }).await.ok();
                                            } else {
                                                dgram.send_object_to(&src_addr, ControlResponder {
                                                    command,
                                                    id,
                                                    status: 1,
                                                    message: "Failed to reload server!".to_string(),
                                                }).await.ok();
                                            }
                                        }
                                        should_skip_loop.store(false, Ordering::SeqCst);
                                    }
                                    "status" => {
                                        debug!(target: "app::commands", "[DATAGRAM] Sending server status...");
                                        let status = if runner_control.is_running() { 0 } else { 1 };
                                        let message = if runner_control.is_running() {
                                            format!("Running with PID: {}", Runtime::pid())
                                        } else { "Stopped".to_string() };
                                        dgram.send_object_to(&src_addr, ControlResponder {
                                            command,
                                            id,
                                            status,
                                            message,
                                        }).await.ok();
                                    }
                                    "pid" => {
                                        debug!(target: "app::commands", "[DATAGRAM] Sending server PID...");
                                        dgram.send_object_to(&src_addr, ControlResponder {
                                            command,
                                            id,
                                            status: 0,
                                            message: Runtime::pid().to_string(),
                                        }).await.ok();
                                    }
                                    "statistic" => {
                                        debug!(target: "app::commands", "[DATAGRAM] Sending server info...");
                                        let _ = dgram.send_object_to::<ServerStats>(&src_addr, ServerStats::from_server(&arc_server)).await;
                                    }
                                    "clean_memory" => {
                                        let begin_memory_status = Status::with_duration(Duration::from_nanos(1)).memory;
                                        let time = chrono::Utc::now().nanosecond();
                                        unsafe {
                                            libc::malloc_trim(0); // clean up
                                        }
                                        let end_memory_status = Status::with_duration(Duration::from_nanos(1)).memory;
                                        let end_time = chrono::Utc::now().nanosecond();
                                        debug!(target: "app::commands", "[DATAGRAM] Sending server cleanup...");
                                        let _ = dgram.send_object_to::<MemoryCleanUpInfo>(&src_addr,
                                            MemoryCleanUpInfo {
                                                pid: id,
                                                time_start: time,
                                                time_end: end_time,
                                                time_usage: end_time - time,
                                                memory_status_start: begin_memory_status,
                                                memory_status_end: end_memory_status,
                                            }
                                        ).await;
                                    }
                                    e => {
                                        debug!(target: "app::commands", "[DATAGRAM] Unknown command: {}", e);
                                    }
                                }
                            }
                        }
                        info!(target: "app::commands", "[DATAGRAM] Control socket closed!");
                    }
                });
                // SERVER RUNNER LOOP
                let runner_task = task::spawn_local(async move {
                    loop {
                        if late_flag.load(Ordering::SeqCst) {
                            break;
                        }
                        if server_runner.is_running() || server_runner.is_processing() || late_should_skip_loop.load(Ordering::SeqCst) {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            continue;
                        }
                        let res = server_runner.start(conf.clone()).await;
                        if let Err(ref e) = res {
                            late_flag.store(true, Ordering::SeqCst);
                            warn!(target: "app::commands", "Server runner exited with error: {}", e);
                        } else if !late_should_skip_loop.load(Ordering::SeqCst) {
                            info!(target: "app::commands", "Server runner loop exited");
                        }
                        {
                            let mut m = this.mutex.lock();
                            *m = Some(res);
                        };
                        if late_flag.load(Ordering::SeqCst) { // re-check
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                });
                // SHUTDOWN HANDLER
                tokio::select! {
                    _ = control_task => debug!(target: "app::commands", "Control loop exited"),
                    _ = runner_task => debug!(target: "app::commands", "Server runner exited"),
                    _ = tokio::signal::ctrl_c() => {
                        info!(target: "app::commands", "Ctrl+C detected! Shutting down...");
                        ctrl_c_flag.store(true, Ordering::SeqCst);
                        runner_server.stop().await;
                    }
                }
            })
            .await;
        let app = config.app();
        let socket_path = app.socket();
        if !socket_path.starts_with('@') && std::path::Path::new(socket_path).exists() {
            let _ = std::fs::remove_file(socket_path);
        }
        option_control.take(); // freed
        let final_res = final_mutex.clone().lock().take();
        match final_res {
            Some(Ok(_)) | None => Ok(ConsoleResult::Ok),
            Some(Err(e)) => Err(e),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct MemoryCleanUpInfo {
    pub(crate) pid: String,
    pub(crate) time_start: u32,
    pub(crate) time_end: u32,
    pub(crate) time_usage: u32,
    pub(crate) memory_status_start: MemoryStatus,
    pub(crate) memory_status_end: MemoryStatus,
}
