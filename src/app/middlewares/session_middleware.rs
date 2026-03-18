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
    fn get_priority(&self) -> isize {
        999
    }
    fn handle(&self, req: ServiceRequest, next: NextFn) -> MiddlewareResult {
        let server = self.server.get_current_config();
        Box::pin(async move {
            if let Some(config) = server {
                {
                    let user_session = UserSession::new(
                        config.get_session_manager().session_from_request(&req),
                        config.get_connection(),
                    );
                    req.extensions_mut().insert(user_session.clone());
                    if let Some(user) = user_session.get_user().await {
                        // If user is found, insert it into the request extensions for later use
                        req.extensions_mut().insert(user.as_ref().clone());
                    }
                }
            }
            next(req).await
        })
    }
}
