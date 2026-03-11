use serde::{Deserialize, Serialize};
use sqlx::{Database, FromRow, Postgres};
use std::fmt::Debug;

use crate::cores::{helper::hack::Hack};

/// A trait representing a database entity that maps to a table row.
///
/// This trait is designed to abstract over types that represent rows in a database table
/// and specifies key properties and capabilities required for interaction with the database.
/// It combines the ability to construct an entity from a row, as well as other utility
/// traits to work with the entity safely across threads and contexts.
///
/// # Associated Types
/// - `KeyType`: The type of the primary key for the entity. It must implement `Send`, `Sync`,
///   `Debug`, `ToString`, and have a static lifetime. This type is used to uniquely
///   identify an entity in the database.
///
/// # Constants
/// - `TABLE_NAME`: A string literal representing the name of the table associated with
///   this entity in the database.
/// - `PRIMARY_KEY`: A string literal representing the primary key column of the table
///   associated with this entity.
///
/// # Required Traits
/// The trait requires several other traits to be implemented by the type:
/// - `FromRow<'r>`: Enables the type to be constructed from a database row.
/// - `Unpin`: Indicates that the type does not use self-referencing pointers and can be
///   safely moved.
/// - `Send` and `Sync`: Ensures the type can be used safely across threads.
/// - `Debug`: Provides a string representation of the type, useful for logging and
///   debugging.
/// - `'static`: Ensures the type does not contain non-static references, meaning it can
///   be used in tasks with arbitrary lifetimes.
/// - `Clone`: Allows the type to be cloned, creating a copy of the instance.
///
/// # Example
/// ```rust
/// #[derive(Debug, Clone)]
/// struct User {
///     id: i32,
///     username: String,
/// }
///
/// impl Entity for User {
///     type KeyType = i32;
///     const TABLE_NAME: &'static str = "users";
///     const PRIMARY_KEY: &'static str = "id";
/// }
/// ```
pub trait Entity:
    for<'r> FromRow<'r, <Postgres as Database>::Row> + Unpin + Send + Sync + Debug + 'static + Clone
{
    type KeyType: Send + Sync + Debug + ToString + 'static;
    const TABLE_NAME: &'static str;
    const TABLE_SCHEMA: &'static str = "public";
    const PRIMARY_KEY: &'static str;
    fn record_state(&self) -> RecordState;
    fn is_clean_state(&self) -> bool {
        self.record_state() == RecordState::Clean
    }
    fn is_new_state(&self) -> bool {
        self.record_state() == RecordState::New
    }
    fn is_dirty_state(&self) -> bool {
        self.record_state() == RecordState::Dirty || self.is_clean_dirty_state()
    }
    fn is_clean_dirty_state(&self) -> bool {
        self.record_state() == RecordState::CleanDirty
    }
    fn table() -> String {
        let mut str = String::new();
        let table_schema = Self::table_schema();
        if !table_schema.trim().is_empty() {
            str.push_str(&Hack::escape_table_identifier(table_schema, false));
            str.push('.');
        }
        str.push_str(&Hack::escape_table_identifier(Self::table_name(), false));
        str
    }

    fn table_quoted() -> String {
        format!(r#""{}""#, Self::table())
    }
    fn table_name() -> &'static str;
    fn table_schema() -> &'static str;
    fn primary_key() -> &'static str;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum RecordState {
    Dirty = 1,
    CleanDirty = 2,
    New = 3,
    #[default]
    Clean = 5,
}

pub fn record_dirty_state() -> RecordState {
    RecordState::Dirty
}
pub fn record_new_state() -> RecordState {
    RecordState::New
}
pub fn record_clean_state() -> RecordState {
    RecordState::New
}
