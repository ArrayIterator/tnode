use crate::app::middlewares::asset_manager_middleware::AssetManagerMiddleware;
use crate::app::middlewares::session_middleware::SessionMiddleware;
use crate::cores::system::error::ResultError;
use crate::cores::system::middleware_manager::MiddlewareManager;
use crate::factory::app::App;
use crate::factory::factory::Factory;

mod asset_manager_middleware;
mod session_middleware;

pub(crate) fn bind_middleware(app: &App) -> ResultError<&App> {
    Factory::pick_mut::<MiddlewareManager>()?
        .register_by_type::<AssetManagerMiddleware>()
        .register_by_type::<SessionMiddleware>();
    Ok(app)
}
