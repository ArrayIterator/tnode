use crate::app::models::user::User;
use crate::cores::{auth::session::Session, database::connection::Connection};
use log::trace;
use tokio::sync::OnceCell;
use std::sync::{Arc, atomic::AtomicBool};

#[derive(Debug, Clone)]
pub struct UserSession {
    session: Arc<Session>,
    user: Arc<OnceCell<Option<Arc<User>>>>,
    queried: Arc<AtomicBool>,
    connection: Arc<Connection>,
}

impl UserSession {
    pub fn new(session: Session, connection: Arc<Connection>) -> Self {
        Self {
            session: Arc::new(session),
            user: Arc::new(OnceCell::new()),
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
        self.user.get_or_init(|| async {
            let payload = self.session.get_payload();
            let user_id = payload.user_id();
            trace!("Querying database for user: {}", user_id);
            match User::find_by_id(&*self.connection, user_id).await {
                Ok(u) => Some(Arc::new(u)),
                Err(_) => None,
            }
        }).await.clone()
    }
}
