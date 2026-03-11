#![deny(unused_imports)]

mod app;
mod cores;
mod factory;

use factory::app::App;
use log::error;
use std::process::exit;
use crate::app::bind_apps;

#[actix_web::main]
async fn main() {
    match bind_apps(&App::instance()) {
        Ok(app) => {
            if let Err(e) = app.run().await {
                error!(target: "main", "Execution failed: {}", e);
                exit(1);
            }
            exit(0);
        }
        Err(e) => {
            error!(target: "main", "Failed to binding routes: {}", e);
            exit(1);
        }
    };
}
