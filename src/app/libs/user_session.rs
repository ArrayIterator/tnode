use crate::app::models::user::User;
use crate::cores::{auth::session::Session, database::connection::Connection};
use parking_lot::{RwLock};
use std::sync::{Arc, atomic::AtomicBool};

#[derive(Debug)]
pub struct UserSession {
    session: Arc<Session>,
    user: RwLock<Option<Arc<User>>>,
    queried: AtomicBool,
    connection: Arc<Connection>,
}

impl UserSession {
    pub fn new(session: Session, connection: Arc<Connection>) -> Self {
        Self {
            session: Arc::new(session),
            user: RwLock::new(None),
            queried: AtomicBool::new(false),
            connection,
        }
    }
    pub fn get_session(&self) -> Arc<Session> {
        self.session.clone()
    }
    pub fn get_session_ref(&self) -> &Session {
        &self.session
    }
    pub fn get_session_deref(&self) -> Session {
        self.get_session_ref().clone()
    }
    pub async fn get_user(&self) -> Option<Arc<User>> {
        if self.queried.load(std::sync::atomic::Ordering::Acquire) {
            return self.user.read().clone();
        }
        let user_id = self.session.get_payload().as_ref().ok()?.user_id();
        if let Ok(u) = User::find(&*self.connection, user_id).await {
            let u_arc = Arc::new(u);
            *self.user.write() = Some(u_arc.clone());
            self.queried.store(true, std::sync::atomic::Ordering::Release);
            return Some(u_arc);
        }
        self.queried
            .store(true, std::sync::atomic::Ordering::Release);
        None
    }
}
