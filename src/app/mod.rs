#![allow(dead_code)]
#![deny(unused_imports)]

use crate::app::commands::bind_commands;
use crate::app::middleware::bind_middleware;
use crate::app::models::bind_models;
use crate::app::routes::bind_routes;
use crate::app::themes::bind_themes;
use crate::cores::system::error::ResultError;
use crate::factory::app::App;

mod commands;
pub mod libs;
pub mod models;
mod routes;
mod themes;
mod middleware;

// only model & libs has pub access
pub(crate) fn bind_apps(app: &App) -> ResultError<&App> {
    bind_commands(app)?;
    bind_models(app)?;
    bind_routes(app)?;
    bind_themes(app)?;
    bind_middleware(app)?;
    Ok(app)
}
