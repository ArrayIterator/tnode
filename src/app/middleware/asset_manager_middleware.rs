use crate::cores::assets::asset_manager::AssetManager;
use crate::cores::assets::assets::Assets;
use crate::cores::system::middleware_manager::{Middleware, MiddlewareResult, NextFn};
use crate::factory::factory::Factory;
use actix_web::{HttpMessage, dev::ServiceRequest};
use std::sync::Arc;

#[derive(Debug)]
pub struct AssetManagerMiddleware {
    assets: Arc<Assets>,
}

impl Default for AssetManagerMiddleware {
    fn default() -> Self {
        Self {
            assets: Factory::pick_or_register::<Assets>(),
        }
    }
}

impl Middleware for AssetManagerMiddleware {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn get_priority(&self) -> isize {
        998
    }
    fn handle(&self, req: ServiceRequest, next: NextFn) -> MiddlewareResult {
        {
            req.extensions_mut()
                .get_or_insert::<AssetManager>(self.assets.new_manager());
        }
        next(req)
    }
}
