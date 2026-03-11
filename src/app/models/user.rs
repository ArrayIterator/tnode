use crate::cores::auth::password::Password;
use crate::cores::auth::session_tokenizer::{SessionPayload, SessionTokenizer};
use crate::cores::base::snapshot::Snapshot;
use crate::cores::base::to_json::ToJson;
use crate::cores::base::user::{UserBase, Util};
use crate::cores::database::connection::ConnectionPool;
use crate::cores::database::entity::{record_dirty_state, Entity, RecordState};
use crate::cores::generator::random::Random;
use crate::cores::system::error::{Error, ResultError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, Postgres, Row};
use std::fmt::{Debug, Display};
use std::sync::{Arc, OnceLock};

pub const MIN_USERNAME_LENGTH: usize = 3;
pub const MAX_USERNAME_LENGTH: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    #[default]
    Active,
    Pending,
    Banned,
    Deleted,
    Suspended,
    Custom(String),
}

impl UserStatus {
    pub fn to_string_status(&self) -> String {
        match self {
            UserStatus::Active => "active".to_string(),
            UserStatus::Pending => "pending".to_string(),
            UserStatus::Banned => "banned".to_string(),
            UserStatus::Deleted => "deleted".to_string(),
            UserStatus::Suspended => "suspended".to_string(),
            UserStatus::Custom(e) => e.clone().to_lowercase(),
        }
    }
}

impl Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_status())
    }
}

impl Into<String> for UserStatus {
    fn into(self) -> String {
        self.to_string().to_lowercase()
    }
}
impl<T: AsRef<str>> From<T> for UserStatus {
    fn from(value: T) -> Self {
        match value.as_ref() {
            "active" => Self::Active,
            "pending" => Self::Pending,
            "banned" => Self::Banned,
            "deleted" => Self::Deleted,
            "suspended" => Self::Suspended,
            other => Self::Custom(other.to_string()),
        }
    }
}
fn default_zero() -> i64 {
    0
}
fn default_user() -> String {
    "user".to_string()
}
fn default_secret_key() -> String {
    Random::hex(64)
}
fn default_now() -> i64 {
    chrono::Utc::now().timestamp()
}
fn default_none_string() -> Option<String> {
    None
}
fn default_none_i64() -> Option<i64> {
    None
}
fn default_user_status() -> String {
    UserStatus::Active.to_string_status()
}
fn default_empty_string() -> String {
    "".to_string()
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    #[serde(default = "default_zero")]
    pub(crate) id: i64,
    pub(crate) username: String,
    pub(crate) email: String,
    #[serde(default = "default_user_status")]
    pub(crate) status: String,
    pub(crate) password: String,
    #[serde(default = "default_user")]
    pub(crate) role: String,
    #[serde(default = "default_empty_string")]
    pub(crate) first_name: String,
    #[serde(default = "default_none_string")]
    pub(crate) last_name: Option<String>,
    #[serde(default = "default_none_string")]
    pub(crate) auth_key: Option<String>,
    #[serde(default = "default_secret_key")]
    pub(crate) secret_key: String,
    #[serde(default = "default_none_string")]
    pub(crate) reason: Option<String>,
    #[serde(default = "default_none_i64")]
    pub(crate) verified_at: Option<i64>,
    #[serde(default = "default_now")]
    pub(crate) created_at: i64,
    #[serde(default = "default_now")]
    pub(crate) updated_at: i64,
    #[serde(default = "default_none_i64")]
    pub(crate) deleted_at: Option<i64>,
    #[serde(default = "default_none_i64")]
    pub(crate) banned_at: Option<i64>,
    #[sqlx(skip)]
    #[serde(skip, default = "record_dirty_state")]
    __state: RecordState,
    #[sqlx(skip)]
    #[serde(skip)]
    __snapshot: Arc<OnceLock<Self>>,
}

impl User {
    pub fn new<Username: AsRef<str>, Email: AsRef<str>, Pass: AsRef<str>>(
        username: Username,
        email: Email,
        password: Pass,
    ) -> ResultError<Self> {
        let timestamp = chrono::Utc::now().timestamp();
        let username = Util::filter_username(username)?;
        let email = Util::filter_email(email)?;
        let password = Password::hash(password)?;
        Ok(Self {
            id: default_zero(),
            username,
            email,
            status: default_user_status(),
            password,
            role: default_user(),
            first_name: "".to_string(),
            last_name: None,
            auth_key: None,
            secret_key: default_secret_key(),
            reason: None,
            verified_at: None,
            created_at: timestamp,
            updated_at: timestamp,
            deleted_at: None,
            banned_at: None,
            __state: RecordState::New,
            __snapshot: Arc::new(OnceLock::new()),
        })
    }

    pub fn generate_token(&self, tokenizer: &SessionTokenizer) -> ResultError<SessionPayload> {
        tokenizer.generate_with(self)
    }

    fn stack_state<T: Send + Sync + 'static + PartialEq>(&mut self, current: T, new: T) {
        if current == new {
            return;
        }
        if !self.is_clean_state() {
            return;
        }
        self.__snapshot.get_or_init(|| {
            let mut snapshot = self.clone();
            snapshot.__snapshot = Arc::new(OnceLock::new());
            snapshot
        });
        self.__state = RecordState::CleanDirty;
    }

    pub fn set_username<T: AsRef<str>>(&mut self, username: T) -> ResultError<()> {
        let username = Util::filter_username(username)?;
        let old_username = self.username.to_lowercase();
        self.stack_state(username.clone(), old_username.clone());
        self.username = username;
        Ok(())
    }
    pub fn set_password<T: AsRef<str>>(&mut self, password: T) -> ResultError<()> {
        self.set_password_hashed(Password::hash(password)?)
    }
    pub fn set_password_hashed<T: AsRef<str>>(&mut self, password_hashed: T) -> ResultError<()> {
        let password_hashed = Password::parse(&password_hashed)?.full;
        self.stack_state(password_hashed.clone(), self.password.clone());
        self.password = password_hashed;
        Ok(())
    }
    pub fn set_email<T: AsRef<str>>(&mut self, email: T) -> ResultError<()> {
        let email = Util::filter_email(email)?;
        self.stack_state(email.clone(), self.email.clone());
        self.email = email;
        Ok(())
    }
    pub fn auth_key(&self) -> Option<&str> {
        self.auth_key.as_deref()
    }
    pub fn set_auth_key(&mut self, auth_key: Option<String>) {
        self.stack_state(self.auth_key.clone(), auth_key.clone());
        self.auth_key = auth_key.clone()
    }
    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }
    pub fn set_secret_key<T: AsRef<str>>(&mut self, secret_key: T) {
        let secret_key = secret_key.as_ref().to_string();
        self.stack_state(self.secret_key.clone(), secret_key.clone());
        self.secret_key = secret_key;
    }
    pub fn status(&self) -> UserStatus {
        if self.is_deleted() {
            // make the real status
            UserStatus::Deleted
        } else {
            UserStatus::from(self.status.clone())
        }
    }
    pub fn set_status<T: Into<UserStatus>>(&mut self, status: T) {
        let status = status.into().to_string_status();
        self.stack_state(self.status.clone(), status.clone());
        self.status = status;
    }
    pub fn role(&self) -> &str {
        &self.role
    }
    pub fn set_role<T: AsRef<str>>(&mut self, role: T) {
        let role = role.as_ref().trim().to_lowercase();
        self.stack_state(self.role.clone(), role.clone());
        self.role = role;
    }
    pub fn first_name(&self) -> &str {
        &self.first_name
    }
    pub fn set_first_name<T: AsRef<str>>(&mut self, first_name: T) {
        let first_name = first_name.as_ref().trim().to_string();
        self.stack_state(self.first_name.clone(), first_name.clone());
        self.first_name = first_name;
    }
    pub fn last_name(&self) -> Option<&str> {
        self.last_name.as_deref()
    }
    pub fn set_last_name<T: AsRef<str>>(&mut self, last_name: Option<T>) {
        let last_name = last_name.map(|l| l.as_ref().trim().to_string());
        self.stack_state(self.last_name.clone(), last_name.clone());
        self.last_name = last_name;
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    pub fn set_reason<T: AsRef<str>>(&mut self, reason: Option<T>) {
        let reason = reason.map(|r| r.as_ref().trim().to_string());
        self.stack_state(self.reason.clone(), reason.clone());
        self.reason = reason;
    }

    pub fn verified_at(&self) -> Option<i64> {
        self.verified_at
    }

    pub fn set_verified_at(&mut self, verified_at: Option<i64>) {
        self.stack_state(self.verified_at.clone(), verified_at.clone());
        self.verified_at = verified_at;
    }

    pub fn created_at(&self) -> i64 {
        self.created_at
    }
    pub fn updated_at(&self) -> i64 {
        self.updated_at
    }
    pub fn deleted_at(&self) -> Option<i64> {
        self.deleted_at
    }
    pub fn set_deleted_at(&mut self, deleted_at: Option<i64>) {
        self.stack_state(self.deleted_at.clone(), deleted_at.clone());
        self.deleted_at = deleted_at;
    }
    pub fn banned_at(&self) -> Option<i64> {
        self.banned_at
    }
    pub fn is_deleted(&self) -> bool {
        self.status() == UserStatus::Deleted || self.deleted_at.is_some()
    }
    pub fn is_active(&self) -> bool {
        !self.is_deleted() && self.status() == UserStatus::Active
    }
    pub fn is_banned(&self) -> bool {
        !self.is_deleted() && self.status() == UserStatus::Banned
    }
    pub fn is_suspended(&self) -> bool {
        !self.is_deleted() && self.status() == UserStatus::Suspended
    }
    pub fn is_pending(&self) -> bool {
        !self.is_deleted() && self.status() == UserStatus::Pending
    }

    pub fn id_u64(&self) -> u64 {
        self.id.max(0) as u64
    }

    pub fn id_usize(&self) -> usize {
        self.id.max(0) as usize
    }

    //noinspection DuplicatedCode
    pub fn diff(&self) -> Vec<(&'static str, Value)> {
        let mut changes = Vec::new();
        let Some(_original) = self.__snapshot.get() else {
            return changes;
        };

        macro_rules! check_diff {
            ($field:ident, $col_name:expr) => {
                if self.$field != _original.$field {
                    if let Ok(val) = serde_json::to_value(&self.$field) {
                        changes.push(($col_name, val));
                    }
                }
            };
        }

        check_diff!(username, "username");
        check_diff!(email, "email");
        check_diff!(status, "status");
        check_diff!(password, "password");
        check_diff!(role, "role");
        check_diff!(first_name, "first_name");
        check_diff!(last_name, "last_name");
        check_diff!(auth_key, "auth_key");
        check_diff!(secret_key, "secret_key");
        check_diff!(reason, "reason");
        check_diff!(verified_at, "verified_at");
        check_diff!(deleted_at, "deleted_at");
        check_diff!(banned_at, "banned_at");
        changes
    }

    pub async fn find_by_username<C: Into<ConnectionPool>, T: AsRef<str>>(
        conn: C,
        username: T,
    ) -> Result<Self, sqlx::Error> {
        let username = username.as_ref().trim().to_lowercase();
        if username.is_empty() {
            return Err(sqlx::Error::InvalidArgument(
                "Username can not be empty or contain whitespace only".to_string(),
            ));
        }
        Ok(sqlx::query_as::<Postgres, Self>(&format!(
            "SELECT * FROM {} WHERE LOWER(username)=$1",
            Self::table_quoted()
        ))
        .bind(&username)
        .fetch_one(&conn.into())
        .await?)
    }
    pub async fn find_by_email<C: Into<ConnectionPool>, T: AsRef<str>>(
        conn: C,
        email: T,
    ) -> Result<Self, sqlx::Error> {
        let email = email.as_ref().trim().to_lowercase();
        if email.is_empty() {
            return Err(sqlx::Error::InvalidArgument(
                "Email can not be empty or contain whitespace only".to_string(),
            ));
        }
        Ok(sqlx::query_as::<Postgres, Self>(&format!(
            "SELECT * FROM {} WHERE email=$1",
            Self::table_quoted()
        ))
        .bind(&email)
        .fetch_one(&conn.into())
        .await?)
    }

    pub async fn find<C: Into<ConnectionPool>>(conn: C, id: u64) -> Result<Self, sqlx::Error> {
        if id <= 0 {
            return Err(sqlx::Error::InvalidArgument(format!(
                "Id should be greater than zero and should not being {}",
                id
            )));
        }
        Ok(sqlx::query_as::<Postgres, Self>(&format!(
            "SELECT * FROM {} WHERE id=$1",
            Self::table_quoted()
        ))
        .bind(id as i64)
        .fetch_one(&conn.into())
        .await?)
    }

    pub async fn save<I: Into<ConnectionPool>>(
        &mut self,
        conn: I,
    ) -> Result<Option<Self>, sqlx::Error> {
        if self.id < 0 || self.is_clean_state() {
            return Ok(None);
        }
        self.username = Util::filter_username(&self.username)
            .map_err(|e| sqlx::Error::InvalidArgument(e.message))?;
        self.email =
            Util::filter_email(&self.email).map_err(|e| sqlx::Error::InvalidArgument(e.message))?;
        let pool = &conn.into();
        if self.is_clean_dirty_state() {
            let changes = self.diff();
            if changes.is_empty() {
                return Ok(None);
            }
            let mut vec = Vec::new();
            let mut inc = 0;
            for (i, _) in &changes {
                inc += 1;
                vec.push(format!("{}=${}", i, inc));
            }
            let updated_at_idx = changes.len() + 1;
            let id_idx = updated_at_idx + 1;
            let v = vec.join(",");
            let str = format!(
                "UPDATE {} SET {}, updated_at=${} WHERE id=${} RETURNING id",
                Self::table_quoted(),
                v,
                updated_at_idx,
                id_idx
            );
            let mut s = sqlx::query(&str);
            for (i, _) in changes {
                s = match i {
                    "id" => s.bind(self.id),
                    "username" => s.bind(self.username()),
                    "password" => s.bind(self.password()),
                    "email" => s.bind(self.email()),
                    "status" => s.bind(self.status().to_string_status()),
                    "role" => s.bind(self.role()),
                    "first_name" => s.bind(self.first_name()),
                    "last_name" => s.bind(self.last_name()),
                    "auth_key" => s.bind(self.auth_key()),
                    "secret_key" => s.bind(self.secret_key()),
                    "verified_at" => s.bind(self.verified_at()),
                    "created_at" => s.bind(self.created_at()),
                    "reason" => s.bind(self.reason()),
                    "deleted_at" => s.bind(self.deleted_at()),
                    "banned_at" => s.bind(self.banned_at()),
                    _ => s,
                }
            }
            let now = chrono::Utc::now().timestamp();
            let r = s.bind(now).bind(self.id).fetch_one(pool).await?;
            let uid: i64 = r.get(0);
            if uid == self.id {
                let mut snapshot = self.clone();
                snapshot.updated_at = now;
                snapshot.__state = RecordState::Clean;
                snapshot.__snapshot = Arc::new(OnceLock::new());
                return Ok(Some(snapshot));
            }
            Ok(None)
        } else {
            let now = chrono::Utc::now().timestamp();
            let str = format!(
                r#"
                INSERT INTO {} (username, password, email, status, role, first_name, last_name, auth_key, secret_key, verified_at, created_at, reason, deleted_at, banned_at, updated_at)
                VALUES           ($1,       $2,       $3,    $4,     $5,   $6,         $7,        $8,       $9,         $10,         $11,        $12,    $13,        $14,       $15)
                RETURNING id
                "#,
                Self::table_quoted()
            );
            let res = sqlx::query(&str)
                .bind(self.username())
                .bind(self.password())
                .bind(self.email())
                .bind(self.status().to_string_status())
                .bind(self.role())
                .bind(self.first_name())
                .bind(self.last_name())
                .bind(self.auth_key())
                .bind(self.secret_key())
                .bind(self.verified_at())
                .bind(now)
                .bind(self.reason())
                .bind(self.deleted_at())
                .bind(self.banned_at())
                .bind(now)
                .fetch_one(pool)
                .await?;
            let id: i64 = res.try_get(0)?;
            let mut snapshot = self.clone();
            snapshot.__snapshot = Arc::new(OnceLock::new());
            snapshot.__state = RecordState::Clean;
            snapshot.id = id;
            snapshot.created_at = now;
            snapshot.updated_at = now;
            Ok(Some(snapshot))
        }
    }
}

impl Entity for User {
    type KeyType = i64;
    const TABLE_NAME: &'static str = "users";
    const PRIMARY_KEY: &'static str = "id";
    fn record_state(&self) -> RecordState {
        self.__state.clone()
    }
    fn table_name() -> &'static str {
        Self::TABLE_NAME
    }
    fn table_schema() -> &'static str {
        Self::TABLE_SCHEMA
    }
    fn primary_key() -> &'static str {
        Self::PRIMARY_KEY
    }
}

impl Snapshot for User {
    fn get_snapshot(&self) -> Option<Self> {
        self.__snapshot.get().map(|e| e.clone())
    }
}

impl UserBase for User {
    fn id(&self) -> i64 {
        self.id
    }

    fn username(&self) -> &str {
        &self.username
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn password(&self) -> &str {
        &self.password
    }
}

impl ToJson for User {
    fn to_json(&self, public: bool) -> ResultError<Value> {
        let mut val = serde_json::to_value(self).map_err(|e| Error::parse_error(e))?;
        if public {
            if let Some(obj) = val.as_object_mut() {
                obj.insert("password".to_string(), Value::String("*".repeat(60)));
            }
        }
        Ok(val)
    }
}

impl Into<u64> for User {
    fn into(self) -> u64 {
        self.id_u64()
    }
}
