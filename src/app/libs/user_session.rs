use crate::app::models::user::User;
use crate::cores::{auth::session::Session, database::connection::Connection};
use parking_lot::RwLock;
use std::sync::{Arc, atomic::AtomicBool};

#[derive(Debug, Clone)]
pub struct UserSession {
    session: Arc<Session>,
    user: Arc<RwLock<Option<Arc<User>>>>,
    queried: Arc<AtomicBool>,
    connection: Arc<Connection>,
}

impl UserSession {
    pub fn new(session: Session, connection: Arc<Connection>) -> Self {
        Self {
            session: Arc::new(session),
            user: Arc::new(RwLock::new(None)),
            queried: Arc::new(AtomicBool::new(false)),
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
        if let Ok(u) = User::find_by_id(&*self.connection, user_id).await {
            self.queried.store(true, std::sync::atomic::Ordering::Release);
            let u = Arc::new(u);
            *self.user.write() = Some(u.clone());
            Some(u);
        }
        self.queried
            .store(true, std::sync::atomic::Ordering::Release);
        None
    }
}
