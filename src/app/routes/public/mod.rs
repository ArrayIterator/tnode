pub(crate) mod home;
pub(crate) mod ws;

use crate::app::routes::public::home::Home;
use crate::app::routes::public::ws::Ws;
use crate::cores::system::routes::{Orchestrator, Routes};

#[derive(Debug)]
pub struct Public
where
    Self: Orchestrator;

impl Orchestrator for Public {
    fn ensemble(routes: &mut Routes) -> &mut Routes {
        routes
            .orchestra::<Home>()
            // routes.orchestra::<Home>()
            .orchestra::<Ws>()
    }
}
