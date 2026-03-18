use crate::cores::auth::password::Password;
use crate::cores::auth::totp::TotpCharLength;
use crate::cores::base::snapshot::Snapshot;
use crate::cores::base::to_json::ToJson;
use crate::cores::base::user::{UserBase, Util};
use crate::cores::database::connection::DbType;
use crate::cores::database::entity::{Entity, RecordState, record_dirty_state};
use crate::cores::generator::random::Random;
use crate::cores::system::error::{Error, ResultError};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, Pool, Row};
use std::fmt::{Debug, Display};
use std::sync::{Arc, LazyLock, OnceLock};
use std::time::Instant;

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

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    #[serde(skip)]
    pub table_name: String,
    #[serde(skip)]
    pub table_schema: String,
    #[serde(default)]
    pub(crate) id: i64,
    #[serde(default)]
    pub(crate) username: String,
    #[serde(default)]
    pub(crate) email: String,
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) password: String,
    #[serde(default)]
    pub(crate) role: String,
    #[serde(default)]
    pub(crate) first_name: String,
    #[serde(default)]
    pub(crate) last_name: Option<String>,
    #[serde(default)]
    pub(crate) auth_key: Option<String>,
    #[serde(default)]
    pub(crate) secret_key: String,
    #[serde(default)]
    pub(crate) reason: Option<String>,
    #[serde(default)]
    pub(crate) verified_at: Option<i64>,
    #[serde(default)]
    pub(crate) created_at: i64,
    #[serde(default)]
    pub(crate) updated_at: i64,
    #[serde(default)]
    pub(crate) deleted_at: Option<i64>,
    #[serde(default)]
    pub(crate) banned_at: Option<i64>,
    #[sqlx(skip)]
    #[serde(skip, default = "record_dirty_state")]
    __state: RecordState,
    #[sqlx(skip)]
    #[serde(skip)]
    __snapshot: Arc<OnceLock<Self>>,
    #[sqlx(skip)]
    #[serde(skip)]
    __cached: Option<Instant>,
}

impl Default for User {
    fn default() -> Self {
        let timestamp = chrono::Utc::now().timestamp();
        Self {
            table_name: "users".to_string(),
            table_schema: "public".to_string(),
            id: 0,
            username: Default::default(),
            email: Default::default(),
            status: UserStatus::Active.to_string_status(),
            password: Default::default(),
            role: "user".to_string(),
            first_name: "".to_string(),
            last_name: None,
            auth_key: None,
            secret_key: Self::generate_secret_key(),
            reason: None,
            verified_at: None,
            created_at: timestamp,
            updated_at: timestamp,
            deleted_at: None,
            banned_at: None,
            __state: RecordState::New,
            __snapshot: Default::default(),
            __cached: None,
        }
    }
}

impl User {
    pub fn new<Username: AsRef<str>, Email: AsRef<str>, Pass: AsRef<str>>(
        username: Username,
        email: Email,
        password: Pass,
    ) -> ResultError<Self> {
        let username = Util::filter_username(username)?;
        let email = Util::filter_email(email)?;
        let password = Password::hash(password)?;
        let mut default_self = Self::default();
        default_self.password = password;
        default_self.email = email;
        default_self.username = username;
        Ok(default_self)
    }
    pub fn generate_secret_key() -> String {
        Random::hex(64)
    }
    pub fn generate_totp_key() -> String {
        TotpCharLength::Default.generate()
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
    pub fn set_first_name<T: AsRef<str>>(&mut self, first_name: T) -> ResultError<()> {
        let first_name = first_name.as_ref().trim().to_string();
        if first_name.is_empty() {
            return Err(Error::invalid_length(
                "First name can not be empty or contain whitespace only",
            ));
        }
        self.stack_state(self.first_name.clone(), first_name.clone());
        self.first_name = first_name;
        Ok(())
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

    pub fn is_cached(&self) -> bool {
        if let Some(cached) = self.__cached {
            let valid_cached = cached.elapsed() < EXPIRATION_DURATION;
            if !valid_cached {
                USER_CACHE.remove(self.id_u64());
            }
            return valid_cached;
        }
        false
    }

    pub fn get_cached_at(&self) -> Option<Instant> {
        if let Some(cached) = self.__cached {
            let valid_cached = cached.elapsed() < EXPIRATION_DURATION;
            if !valid_cached {
                USER_CACHE.remove(self.id_u64());
            }
            return Some(cached);
        }
        None
    }

    pub async fn delete<C: Into<Pool<DbType>>>(&mut self, conn: C) -> Result<(), sqlx::Error> {
        if self.is_deleted() {
            return Ok(());
        }
        let now = chrono::Utc::now().timestamp();
        let str = format!(
            "UPDATE {} SET deleted_at=$1, updated_at=$2 WHERE id=$3",
            Self::default().table_quoted()
        );
        sqlx::query(&str)
            .bind(now)
            .bind(now)
            .bind(self.id)
            .execute(&conn.into())
            .await?;
        self.deleted_at = Some(now);
        USER_CACHE.remove(self.id_u64());
        Ok(())
    }

    pub async fn find_fresh_by_username<C: Into<Pool<DbType>>, T: AsRef<str>>(
        conn: C,
        username: T,
    ) -> Result<Self, sqlx::Error> {
        let username = username.as_ref().trim().to_lowercase();
        if username.is_empty() {
            return Err(sqlx::Error::InvalidArgument(
                "Username can not be empty or contain whitespace only".to_string(),
            ));
        }
        let user = sqlx::query_as::<DbType, Self>(&format!(
            "SELECT * FROM {} WHERE LOWER(username)=$1",
            Self::default().table_quoted()
        ))
        .bind(&username)
        .fetch_one(&conn.into())
        .await?;
        USER_CACHE.insert_ref(&user);
        Ok(user)
    }

    pub async fn find_by_username<C: Into<Pool<DbType>>, T: AsRef<str>>(
        conn: C,
        username: T,
    ) -> Result<Self, sqlx::Error> {
        let username = username.as_ref().trim().to_lowercase();
        if let Some(user) = USER_CACHE.find_by_username(&username) {
            return Ok(user);
        }
        Self::find_fresh_by_username(conn, username).await
    }

    pub async fn find_by_fresh_email<C: Into<Pool<DbType>>, T: AsRef<str>>(
        conn: C,
        email: T,
    ) -> Result<Self, sqlx::Error> {
        let email = email.as_ref().trim().to_lowercase();
        if email.is_empty() {
            return Err(sqlx::Error::InvalidArgument(
                "Email can not be empty or contain whitespace only".to_string(),
            ));
        }
        let second_mail = match Util::filter_email(&email) {
            Ok(e) => e,
            Err(_) => email.clone(),
        };
        let user = sqlx::query_as::<DbType, Self>(&format!(
            "SELECT * FROM {} WHERE email=$1 OR LOWER(email)=$1",
            Self::default().table_quoted()
        ))
        .bind(&email)
        .bind(&second_mail)
        .fetch_one(&conn.into())
        .await?;
        USER_CACHE.insert_ref(&user);
        Ok(user)
    }

    pub async fn find_by_email<C: Into<Pool<DbType>>, T: AsRef<str>>(
        conn: C,
        email: T,
    ) -> Result<Self, sqlx::Error> {
        let email = email.as_ref().trim().to_lowercase();
        if let Some(user) = USER_CACHE.find_by_email(&email) {
            return Ok(user);
        }
        Self::find_by_fresh_email(conn, email).await
    }

    pub async fn find_fresh_by_id<C: Into<Pool<DbType>>>(
        conn: C,
        id: u64,
    ) -> Result<Self, sqlx::Error> {
        if id <= 0 {
            return Err(sqlx::Error::InvalidArgument(format!(
                "Id should be greater than zero and should not being {}",
                id
            )));
        }
        let user = sqlx::query_as::<DbType, Self>(&format!(
            "SELECT * FROM {} WHERE id=$1",
            Self::default().table_quoted()
        ))
        .bind(id as i64)
        .fetch_one(&conn.into())
        .await?;
        USER_CACHE.insert_ref(&user);
        Ok(user)
    }

    pub async fn find_by_id<C: Into<Pool<DbType>>>(conn: C, id: u64) -> Result<Self, sqlx::Error> {
        if id <= 0 {
            return Err(sqlx::Error::InvalidArgument(format!(
                "Id should be greater than zero and should not being {}",
                id
            )));
        }
        if let Some(user) = USER_CACHE.find(id) {
            return Ok(user);
        }
        Self::find_fresh_by_id(conn, id).await
    }

    pub async fn save<I: Into<Pool<DbType>>>(
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
        if self.password.is_empty() {
            return Err(sqlx::Error::InvalidArgument(
                "Password can not be empty".to_string(),
            ));
        }
        if self.first_name.trim().is_empty() {
            return Err(sqlx::Error::InvalidArgument(
                "First name can not be empty or contain whitespace only".to_string(),
            ));
        }
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
                Self::default().table_quoted(),
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
                Self::default().table_quoted()
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
            USER_CACHE.insert_ref(&snapshot);
            Ok(Some(snapshot))
        }
    }
}

impl Entity for User {
    type KeyType = i64;
    fn record_state(&self) -> RecordState {
        self.__state.clone()
    }

    fn table_name(&self) -> &str {
        &self.table_name
    }

    fn table_schema(&self) -> &str {
        &self.table_schema
    }

    fn primary_key(&self) -> &str {
        "id"
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

#[derive(Debug)]
struct UserCache {
    users: DashMap<u64, (Instant, Arc<User>)>,
    email_to_id: DashMap<String, u64>,
    username_to_id: DashMap<String, u64>,
    capcity: usize,
}

impl UserCache {
    fn new() -> Self {
        Self {
            users: DashMap::with_capacity(CACHE_CAPACITY),
            capcity: CACHE_CAPACITY,
            email_to_id: DashMap::with_capacity(CACHE_CAPACITY),
            username_to_id: DashMap::with_capacity(CACHE_CAPACITY),
        }
    }
    fn remove(&self, user_id: u64) -> Option<Arc<User>> {
        self.users.remove(&user_id).map(|(_, (_, user))| user)
    }

    fn find(&self, id: u64) -> Option<User> {
        if id == 0 {
            return None;
        }
        if let Some((instant, user)) = self.users.get(&id).map(|e| e.value().clone()) {
            if instant.elapsed() < EXPIRATION_DURATION {
                let mut user = user.as_ref().clone();
                user.__cached = Some(instant);
                return Some(user);
            }
            let email = user.email().to_lowercase();
            let username = user.username().to_lowercase();
            self.users.remove(&id);
            self.email_to_id.remove(&email);
            self.username_to_id.remove(&username);
        }
        None
    }
    fn find_by_email<T: AsRef<str>>(&self, email: T) -> Option<User> {
        let email = email.as_ref().to_lowercase();
        if let Some(id) = self.email_to_id.get(&email).map(|e| *e.value()) {
            return self.find(id);
        }
        None
    }
    fn find_by_username<T: AsRef<str>>(&self, username: T) -> Option<User> {
        let username = username.as_ref().to_lowercase();
        if let Some(id) = self.username_to_id.get(&username).map(|e| *e.value()) {
            return self.find(id);
        }
        None
    }
    fn insert(&self, user: User) {
        self.insert_ref(&user);
    }
    fn insert_ref(&self, user: &User) {
        let mut user = user.clone();
        let now = Instant::now();
        user.__cached = Some(now);
        let user = Arc::new(user);
        let id = user.id_u64();
        if self.users.len() >= CACHE_CAPACITY {
            // clean up
            self.users.retain(|_, (inst, u)| {
                let is_fresh = now.duration_since(*inst) < EXPIRATION_DURATION;
                if !is_fresh {
                    self.email_to_id.remove(&u.email().to_lowercase());
                    self.username_to_id.remove(&u.username().to_lowercase());
                }
                is_fresh
            });
            let current_len = self.users.len();
            if current_len >= CACHE_CAPACITY {
                let to_remove_idx = CLAIM_CAPACITY.min(current_len - 1);
                let mut entries: Vec<_> = self
                    .users
                    .iter()
                    .map(|e| {
                        let v = e.value();
                        let u = v.clone().1;
                        (
                            *e.key(),
                            v.0,
                            u.email().to_lowercase(),
                            u.username().to_lowercase(),
                        )
                    })
                    .collect();
                entries.select_nth_unstable_by_key(to_remove_idx, |(_, inst, ..)| *inst);
                for (key, _, email, username) in entries.into_iter().take(to_remove_idx + 1) {
                    self.users.remove(&key);
                    self.email_to_id.remove(&email);
                    self.username_to_id.remove(&username);
                }
            }
        }
        self.users.insert(id, (Instant::now(), user.clone()));
        self.email_to_id.insert(user.email().to_lowercase(), id);
        self.username_to_id
            .insert(user.username().to_lowercase(), id);
    }
}

const CLAIM_CAPACITY: usize = 100;
const CACHE_CAPACITY: usize = 8192;
const EXPIRATION_DURATION: std::time::Duration = std::time::Duration::from_mins(15); // 15 minutes
static USER_CACHE: LazyLock<UserCache> = LazyLock::new(|| UserCache::new());
