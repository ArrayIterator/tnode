use crate::cores::system::error::ResultError;
use crate::factory::app::App;

pub mod user;
mod user_meta;

pub(crate) fn bind_models(app: &App) -> ResultError<&App> {
    Ok(app)
}
