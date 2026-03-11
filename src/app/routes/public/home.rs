// use std::sync::Arc;
use crate::cores::system::routes::Route;
use actix_web::web::{ServiceConfig};
use actix_web::{HttpResponse, web};

#[derive(Debug, Default)]
pub(crate) struct Home;

impl Route for Home {
    fn mount(&self, cfg: &mut ServiceConfig, _prefix: &str) {
        cfg.route(
            "/",
            web::get().to(move || {
                // Gak ada serialize, gak ada mikir, langsung lempar bytes!
                async move {
                    HttpResponse::Ok()
                        .content_type("text/html")
                        .body("OK")
                }
            }),
        );
    }
}
