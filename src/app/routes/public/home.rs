use crate::cores::system::routes::RouteMounter;
use actix_web::web::{ServiceConfig, resource};
use actix_web::{HttpResponse, web};

#[derive(Debug, Default)]
pub(crate) struct Home;

impl RouteMounter for Home {
    fn mount(&self, cfg: &mut ServiceConfig,) {
        cfg.service(resource(""));
        cfg.route(
            "/",
            web::get().to(move || {
                // Gak ada serialize, gak ada mikir, langsung lempar bytes!
                async move {
                    HttpResponse::Ok()
                        .content_type("text/html")
                        .body("OK")
                }
            })
        );
    }
}
