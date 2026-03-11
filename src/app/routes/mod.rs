use crate::app::routes::admin::Admin;
use crate::app::routes::public::Public;
use crate::cores::system::error::ResultError;
use crate::cores::system::routes::Routes;
use crate::factory::app::App;
use crate::factory::factory::Factory;

mod admin;
mod public;

pub(crate) fn bind_routes(app: &App) -> ResultError<&App> {
    Factory::pick_mut::<Routes>()?
        .ensembles::<Admin>()
        .ensembles::<Public>();
    Ok(app)
}
