#![deny(unused_imports)]

mod app;
mod cores;
mod factory;
mod commands;

use factory::app::App;
use log::error;
use std::process::exit;
use crate::app::bind_apps;
use crate::commands::bind_commands;

#[actix_web::main]
async fn main() {
    let app = App::instance();
    if let Err(e) = bind_commands(&app) {
        error!(target: "main", "Failed to binding commands: {}", e);
        exit(1);
    }
    if let Err(e) = bind_apps(&app) {
        error!(target: "main", "Failed to binding apps: {}", e);
        exit(1);
    }
    match app.run().await {
        Ok(_) => exit(9),
        Err(e) => {
            error!(target: "main", "Execution failed: {}", e);
            exit(1);
        },
    }
}
