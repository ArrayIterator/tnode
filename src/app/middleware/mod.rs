use crate::app::middleware::asset_manager_middleware::AssetManagerMiddleware;
use crate::app::middleware::session_middleware::SessionMiddleware;
use crate::cores::system::error::ResultError;
use crate::cores::system::middleware_manager::MiddlewareManager;
use crate::factory::app::App;
use crate::factory::factory::Factory;

mod asset_manager_middleware;
mod session_middleware;

pub(crate) fn bind_middleware(app: &App) -> ResultError<&App> {
    Factory::pick_mut::<MiddlewareManager>()?
        .register::<AssetManagerMiddleware>()
        .register::<SessionMiddleware>();
    Ok(app)
}
