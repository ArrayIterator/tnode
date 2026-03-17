use crate::cores::system::routes::{Orchestrator, Routes};

#[derive(Debug)]
pub struct Admin
where
    Self: Orchestrator;

impl Orchestrator for Admin {
    fn orchestra(routes: &mut Routes) -> &mut Routes {
        routes
    }
}
