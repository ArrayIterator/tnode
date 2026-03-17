#![allow(dead_code)]
#![deny(unused_imports)]

use crate::commands::init::Init;
use crate::commands::monitor::Monitor;
use crate::commands::server::Server;
use crate::cores::runner::cli::Cli;
use crate::cores::system::error::ResultError;
use crate::factory::app::App;
use crate::factory::factory::Factory;

mod init;
mod monitor;
mod server;
mod servers;

pub(crate) fn bind_commands(app: &App) -> ResultError<&App> {
    Factory::pick_mut_fn::<Cli, ()>(|cli| {
        cli.add_command(Init::default())
            .expect("Failed to add init command");
        cli.add_command(Server::default())
            .expect("Failed to add server command");
        cli.add_command(Monitor::default())
            .expect("Failed to add monitor command");
        cli.set_empty_arg(vec!["server", "start"])
    })?;
    Ok(app)
}
