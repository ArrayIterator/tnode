use crate::app::commands::servers::server_daemon::MemoryCleanUpInfo;
use crate::cores::generator::uuid::Uuid;
use crate::cores::helper::hack::Hack;
use crate::cores::runner::console::ConsoleResult;
use crate::cores::system::commander::{ControlCommander, ControlResponder, ParserInto};
use crate::cores::system::error::{Error, ResultError};
use crate::factory::factory::Factory;
use crossterm::style::Stylize;
use log::warn;
use std::process::exit;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::task;

pub(crate) struct ServerActions;

impl ServerActions {
    fn commander() -> Arc<ControlCommander> {
        Factory::pick_unsafe::<ControlCommander>()
    }
    pub async fn status() -> ResultError<ConsoleResult> {
        let commander = Self::commander();
        let uuid = Uuid::v7().to_string();
        let (buff, size, ..) = commander
            .send_receive("status", uuid, None)
            .await
            .map_err(|e| {
                println!(
                    "{}: {}",
                    "Failed to get server status!".red(),
                    e.message.red()
                );
                exit(1)
            })
            .unwrap();
        let res = commander
            .parse_into::<ControlResponder>(&buff, size)
            .map_err(|e| {
                Error::parse_error(format!(
                    "Failed to parse status command response: {} -> {}",
                    e,
                    String::from_utf8_lossy(&buff[..size])
                ))
            })?;
        if res.status != 0 {
            println!("{}", res.message.red());
        } else {
            println!("{}", res.message.green());
        }
        Ok(ConsoleResult::Ok)
    }
    pub async fn clean_memory() -> ResultError<ConsoleResult> {
        let commander = Self::commander();
        let uuid = Uuid::v7().to_string();
        let (buff, size, ..) = commander
            .send_receive("clean_memory", uuid, None)
            .await
            .map_err(|e| {
                println!(
                    "{}: {}",
                    "Failed to get clean memory!".red(),
                    e.message.red()
                );
                exit(1)
            })
            .unwrap();
        let res = commander
            .parse_into::<MemoryCleanUpInfo>(&buff, size)
            .map_err(|e| {
                Error::parse_error(format!(
                    "Failed to parse clean memory command response: {} -> {}",
                    e,
                    String::from_utf8_lossy(&buff[..size])
                ))
            })?;
        let elapsed = Hack::format_duration(Duration::from_nanos(res.time_usage as u64), false);
        println!(
            "{}",
            format!(
                "Cleanup memory from {} to {} & take {}",
                Hack::format_size(res.memory_status_start.rss_anon),
                Hack::format_size(res.memory_status_end.rss_anon),
                elapsed
            )
            .green()
        );
        Ok(ConsoleResult::Ok)
    }
    pub async fn stop(gracefully: bool, silent: bool) -> ResultError<ConsoleResult> {
        let commander = Self::commander();
        let uuid = Uuid::v7().to_string();
        let is_gracefully = gracefully;
        let gracefully = if gracefully {
            Some("gracefully".to_string())
        } else {
            None
        };
        match commander.send_receive("stop", uuid, gracefully).await {
            Ok(s) => {
                let (buff, size, ..) = s;
                let res = commander.parse_into::<ControlResponder>(&buff, size)?;
                if !silent {
                    if res.status != 0 {
                        println!("{}", res.message.red());
                    } else {
                        println!("{}", res.message.green());
                    }
                }
                if !is_gracefully {
                    return Ok(ConsoleResult::Ok);
                }
                let stop_flag = Arc::new(AtomicBool::new(false));
                let stop_clone = stop_flag.clone();
                let arc_error_stop = Arc::new(AtomicBool::new(false));
                let arc_error_err = arc_error_stop.clone();
                let loop_task = task::spawn_local(async move {
                    let mut max_tries = 60;
                    loop {
                        if stop_flag.load(Ordering::SeqCst) {
                            break;
                        }
                        max_tries -= 1;
                        if max_tries < 0 {
                            println!("{}", "Can not stop server! after 60 retries".red());
                            stop_flag.store(true, Ordering::SeqCst);
                            arc_error_stop.store(true, Ordering::SeqCst);
                            break;
                        }
                        let arc = commander.connect();
                        if let Ok(arc) = arc {
                            if let Ok((final_data, size, ..)) = commander
                                .receive_timeout(Duration::from_secs(5), Some(arc))
                                .await
                            {
                                match commander.parse_into::<ControlResponder>(&final_data, size) {
                                    Ok(res) => {
                                        if !silent {
                                            if res.status == 0 {
                                                println!("{}", res.message.green());
                                            } else {
                                                println!("{}", res.message.red());
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        if !silent {
                                            println!("{}", e.message.red());
                                        }
                                    }
                                }
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                });
                tokio::select! {
                    _ = loop_task => {
                    }
                    _ = tokio::signal::ctrl_c() => {
                        stop_clone.store(true, Ordering::SeqCst);
                    }
                }
                if arc_error_err.load(Ordering::SeqCst) {
                    return Err(Error::timeout(
                        "Can not stop server gracefully after 60 retries!",
                    ));
                }
                Ok(ConsoleResult::Ok)
            }
            Err(e) => {
                if !silent {
                    println!("{}", "Server stopped".green());
                    warn!(target: "app::commands", "{:?}", e.message);
                }
                Ok(ConsoleResult::Err)
            }
        }
    }
    pub async fn reload(gracefully: bool) -> ResultError<ConsoleResult> {
        let commander = Self::commander();
        let uuid = Uuid::v7().to_string();
        let gracefully = if gracefully {
            Some("gracefully".to_string())
        } else {
            None
        };
        let (buff, size, ..) = commander
            .send_receive("reload", uuid, gracefully)
            .await
            .map_err(|e| {
                println!(
                    "{}: {}",
                    "Failed to communicate to server!".red(),
                    e.message.red()
                );
                exit(1);
            })
            .unwrap();
        let res = commander
            .parse_into::<ControlResponder>(&buff, size)
            .map_err(|e| {
                Error::parse_error(format!(
                    "Failed to parse reload command response: {} -> {}",
                    e,
                    String::from_utf8_lossy(&buff[..size])
                ))
            })?;
        if res.status != 0 {
            println!("{}", res.message.red());
        } else {
            println!("{}", res.message.green());
        }
        Ok(ConsoleResult::Ok)
    }
}
