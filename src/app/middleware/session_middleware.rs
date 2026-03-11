use crate::app::libs::user_session::UserSession;
use crate::cores::system::middleware_manager::{Middleware, MiddlewareResult, NextFn};
use crate::factory::factory::Factory;
use crate::factory::server::Server;
use actix_web::HttpMessage;
use actix_web::dev::ServiceRequest;
use std::sync::Arc;

#[derive(Debug)]
pub struct SessionMiddleware {
    server: Arc<Server>,
}

impl Default for SessionMiddleware {
    fn default() -> Self {
        Self {
            server: Factory::pick_or_register::<Server>(),
        }
    }
}

impl Middleware for SessionMiddleware {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn get_priority(&self) -> isize {
        999
    }
    fn handle(&self, req: ServiceRequest, next: NextFn) -> MiddlewareResult {
        if let Some(config) = self.server.get_current_config() {
            {
                let user_session = UserSession::new(
                    config.get_session_manager().session_from_request(&req),
                    config.get_connection(),
                );
                req.extensions_mut().insert(user_session);
            }
        }
        next(req)
    }
}
