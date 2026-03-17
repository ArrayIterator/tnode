use crate::app::models::user::User;
use crate::cores::base::snapshot::Snapshot;
use crate::cores::base::to_json::ToJson;
use crate::cores::base::user::UserBase;
use crate::cores::database::connection::DbType;
use crate::cores::database::entity::{record_dirty_state, Entity, RecordState};
use crate::cores::system::error::{Error, ResultError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, Pool, Row};
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserMeta {
    #[serde(skip)]
    pub table_name: String,
    #[serde(skip)]
    pub table_schema: String,
    #[serde(default)]
    pub(crate) meta_id: i64,
    #[serde(default)]
    pub(crate) user_id: i64,
    #[serde(default)]
    pub(crate) meta_name: String,
    #[serde(default)]
    pub(crate) meta_value: Option<String>,
    #[sqlx(skip)]
    #[serde(skip, default = "record_dirty_state")]
    __state: RecordState,
    #[sqlx(skip)]
    #[serde(skip)]
    __snapshot: Arc<OnceLock<Self>>,
}

impl Default for UserMeta {
    fn default() -> Self {
        Self {
            table_name: "user_meta".to_string(),
            table_schema: "public".to_string(),
            meta_id: 0,
            user_id: 0,
            meta_name: String::new(),
            meta_value: None,
            __state: RecordState::New,
            __snapshot: Default::default(),
        }
    }
}

impl Entity for UserMeta {
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
        "meta_id"
    }
}

impl UserMeta {
    fn stack_state<T: Send + Sync + 'static + PartialEq>(&mut self, current: T, new: T) {
        if current == new || !self.is_clean_state() {
            return;
        }
        self.__snapshot.get_or_init(|| {
            let mut snapshot = self.clone();
            snapshot.__snapshot = Arc::new(OnceLock::new());
            snapshot
        });
        self.__state = RecordState::CleanDirty;
    }
    pub fn resolve_uid(user: User) -> ResultError<i64> {
        if user.is_new_state() {
            return Err(Error::invalid_state(
                "User argument state should not as new",
            ));
        }
        let uid: i64;
        if !user.is_clean_state() {
            if let Some(snapshot) = user.get_snapshot() {
                uid = snapshot.id;
            } else {
                return Err(Error::invalid_state(format!(
                    "Invalid state {:?} with user id: {}",
                    user.record_state(),
                    user.id
                )));
            }
        } else {
            uid = user.id;
        }
        if uid <= 0 {
            return Err(Error::invalid_data(format!("User id {} is invalid", uid)));
        }
        Ok(uid)
    }

    pub fn new<T: AsRef<str>>(user: User, meta_name: T, meta_value: Option<String>) -> ResultError<Self> {
        let mut y = Self::default();
        y.meta_name = meta_name.as_ref().to_string();
        y.meta_value = meta_value.clone();
        y.user_id = Self::resolve_uid(user.clone())?;
        Ok(y)
    }
    pub fn new_unchecked<T: AsRef<str>>(uid: u64, meta_name: T, meta_value:  Option<String>) -> Self {
        let mut y = Self::default();
        y.meta_name = meta_name.as_ref().to_string();
        y.meta_value = meta_value.clone();
        y.user_id = uid as i64;
        y
    }

    pub fn set_user_id(&mut self, user: User) -> ResultError<()> {
        let uid = Self::resolve_uid(user)?;
        self.stack_state(self.user_id, uid);
        self.user_id = uid;
        Ok(())
    }
    pub fn set_user_id_unchecked(&mut self, uid: u64) {
        self.stack_state(self.user_id, uid as i64);
        self.user_id = uid as i64;
    }
    pub fn get_user_id(&self) -> i64 {
        self.user_id
    }
    pub fn get_meta_id(&self) -> i64 {
        self.meta_id
    }
    pub fn set_meta_name<T: AsRef<str>>(&mut self, meta_name: T) {
        let meta_name = meta_name.as_ref().to_string();
        self.stack_state(self.meta_name.clone(), meta_name.clone());
        self.meta_name = meta_name;
    }
    pub fn get_meta_name(&self) -> &str {
        &self.meta_name
    }
    pub fn set_meta_value(&mut self, meta_value:  Option<String>) {
        self.stack_state(self.meta_value.clone(), meta_value.clone());
        self.meta_value = meta_value;
    }
    pub fn get_meta_value(&self) ->  Option<String> {
        self.meta_value.clone()
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
        check_diff!(user_id, "user_id");
        check_diff!(meta_name, "meta_name");
        check_diff!(meta_value, "meta_value");
        changes
    }
    pub async fn find<I: Into<Pool<DbType>>, N: AsRef<str>>(
        conn: I,
        user: User,
        meta_name: N,
    ) -> Result<UserMeta, sqlx::Error> {
        if user.id <= 0 {
            return Err(sqlx::Error::InvalidArgument(format!(
                "User id {} is invalid",
                user.id
            )));
        }
        let name = meta_name.as_ref();
        Ok(sqlx::query_as::<DbType, Self>(&format!(
            r#"
SELECT * FROM {}
 WHERE
    user_id = $1 AND meta_name = $2
LIMIT 1
"#,
            Self::default().table_quoted()
        ))
            .bind(user.id())
            .bind(name.to_string())
            .fetch_one(&conn.into())
            .await?)
    }

    pub async fn save<I: Into<Pool<DbType>>>(
        &mut self,
        conn: I,
    ) -> Result<Option<Self>, sqlx::Error> {
        if self.user_id <= 0
            || self.is_clean_state()
            || (self.is_clean_dirty_state() && self.diff().is_empty())
        {
            return Ok(None);
        }
        if self.meta_name.trim().is_empty() {
            return Err(sqlx::Error::InvalidArgument("Meta name could not be empty or whitespace only".to_string()))
        }
        let table_name = Self::default().table_quoted();
        let pool = &conn.into();
        let original = self.get_snapshot();
        if self.meta_id > 0 {
            let target_id = original.as_ref().map(|s| s.meta_id).unwrap_or(self.meta_id);
            let exists = sqlx::query(&format!(
                "SELECT 1 FROM {} WHERE meta_id = $1",
                table_name
            ))
                .bind(target_id)
                .fetch_optional(pool)
                .await?;

            if exists.is_some() {
                let res = sqlx::query(&format!(
                    r#"
                UPDATE {}
                SET
                    user_id = $1,
                    meta_name = $2,
                    meta_value = $3
                WHERE
                    meta_id = $4
                RETURNING meta_id
                "#,
                    table_name
                ))
                    .bind(self.user_id)
                    .bind(&self.meta_name)
                    .bind(&self.meta_value)
                    .bind(target_id)
                    .fetch_optional(pool)
                    .await?;
                if let Some(row) = res {
                    let meta_id: i64 = row.get(0);
                    let mut snapshot = self.clone();
                    snapshot.__state = RecordState::Clean;
                    snapshot.__snapshot = Arc::new(OnceLock::new());
                    snapshot.meta_id = meta_id;
                    return Ok(Some(snapshot));
                }
            }
        }

        let res = sqlx::query(&format!(
            r#"
        INSERT INTO {} (user_id, meta_name, meta_value)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, meta_name)
        DO UPDATE SET
            meta_value = EXCLUDED.meta_value
        RETURNING meta_id
        "#,
            table_name
        ))
            .bind(self.get_user_id())
            .bind(&self.get_meta_name())
            .bind(&self.get_meta_value())
            .fetch_one(pool)
            .await?;
        let meta_id: i64 = res.try_get(0)?;
        let mut snapshot = self.clone();
        snapshot.__state = RecordState::Clean;
        snapshot.__snapshot = Arc::new(OnceLock::new());
        snapshot.meta_id = meta_id;
        Ok(Some(snapshot))
    }
}

impl Into<u64> for UserMeta {
    fn into(self) -> u64 {
        self.get_meta_id() as u64
    }
}

impl Snapshot for UserMeta {
    fn get_snapshot(&self) -> Option<Self> {
        self.__snapshot.get().map(|e| e.clone())
    }
}

impl ToJson for UserMeta {
    fn to_json(&self, _: bool) -> ResultError<Value> {
        let mut snapshot = self.clone();
        snapshot.__snapshot = Arc::new(OnceLock::new());
        serde_json::to_value(snapshot).map_err(|e| Error::parse_error(e))
    }
}
