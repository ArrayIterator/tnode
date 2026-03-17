use std::{ops::Deref, sync::Arc};

use dashmap::DashMap;

use crate::cores::{database::adapter::Driver, helper::hack::Hack, system::error::{Error, ResultError}};

const PG: Driver = Driver::Postgresql;
const SQLITE: Driver = Driver::Sqlite;
const MYSQL: Driver = Driver::MySql;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Collation {
    #[default]
    Default, // default collation is npne, it depends on the database and column type
    UnicodeCaseSensitive, // Unicode collation (e.g., "en_US.UTF-8" with Unicode settings, case-sensitive)
    UnicodeCaseInsensitive, // Unicode collation (e.g., "en_US.UTF-8" with Unicode settings)
    CaseSensitive, // Case-sensitive collation (e.g., "en_US.UTF-8" with case-sensitive settings)
    CaseInsensitive, // Case-insensitive collation (e.g., "en_US.UTF-8" with case-insensitive settings)
    Binary, // Binary collation (byte-by-byte comparison, case-sensitive)
    Custom(String), // Custom collation name (e.g., "en_US", "fr_FR", "und-u-ks-level3-x-icu", etc.)
}

impl Collation {
    pub fn as_name(&self, driver: Driver) -> Option<&str> {
        match self {
            Self::UnicodeCaseSensitive => Some(match driver {
                Driver::Postgresql => "und-u-ks-level3-x-icu",
                Driver::MySql => "utf8mb4_bin", // MySQL biasanya pakai _bin untuk case sensitive
                Driver::Sqlite => "BINARY",
            }),
            Self::UnicodeCaseInsensitive|Self::CaseInsensitive => Some(match driver {
                Driver::Postgresql => "und-u-ks-level2-x-icu",
                Driver::MySql => "utf8mb4_unicode_ci",
                Driver::Sqlite => "NOCASE",
            }),
            Self::Binary|Self::CaseSensitive => Some(match driver {
                Driver::Postgresql => "C",
                Driver::MySql      => "utf8mb4_bin", // Atau "binary" kalau di level kolom raw
                Driver::Sqlite     => "BINARY",
            }),
            Self::Default => None,
            Self::Custom(s) => Some(s),
        }
    }
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }
    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary)
    }
    pub fn is_unicode(&self) -> bool {
        matches!(self, Self::UnicodeCaseSensitive | Self::UnicodeCaseInsensitive)
    }
    pub fn is_case_insensitive(&self) -> bool {
        matches!(self, Self::UnicodeCaseInsensitive | Self::CaseInsensitive)
    }
    pub fn is_case_sensitive(&self) -> bool {
        matches!(self, Self::UnicodeCaseSensitive | Self::CaseSensitive)
    }
    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColumnCategory {
    Character,
    Numeric,
    DateTime,
    Boolean,
    Specialized,
    SpecializedBinary,
    SpecializedBinaryArray,
    NetworkAddress,
    Geometric,
    FullTextSearch,
    BitString,
    XML,
    InternalSystem,
}

impl ColumnCategory {
    pub fn is_text_based(&self) -> bool {
        matches!(self, Self::Character | Self::Specialized | Self::SpecializedBinary | Self::FullTextSearch | Self::XML)
    }
    pub fn is_binary_based(&self) -> bool {
        matches!(self, Self::BitString | Self::SpecializedBinary | Self::SpecializedBinaryArray)
    }
    pub fn is_numeric_based(&self) -> bool {
        matches!(self, Self::Numeric)
    }
     pub fn is_date_time_based(&self) -> bool {
        matches!(self, Self::DateTime)
    }
     pub fn is_boolean_based(&self) -> bool {
        matches!(self, Self::Boolean)
    }
     pub fn is_network_address_based(&self) -> bool {
        matches!(self, Self::NetworkAddress)
    }
     pub fn is_geometric_based(&self) -> bool {
        matches!(self, Self::Geometric)
    }
     pub fn is_internal_system_based(&self) -> bool {
        matches!(self, Self::InternalSystem)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColumnType {
    // Character Types
    /// Variable-length character string with a defined limit.
    VARCHAR,
    /// Fixed-length character string, blank-padded.
    CHAR,
    /// Variable-length character string without a limit.
    TEXT,
    /// For simplicity, we can treat MEDIUM_TEXT and LONG_TEXT as TEXT in PostgreSQL, since PostgreSQL's TEXT type can handle large strings. However, if we want to maintain the distinction for compatibility with MySQL, we can include them as separate types.
    MEDIUMTEXT,
    /// For simplicity, we can treat MEDIUM_TEXT and LONG_TEXT as TEXT in PostgreSQL, since PostgreSQL's TEXT type can handle large strings. However, if we want to maintain the distinction for compatibility with MySQL, we can include them as separate types.
    LONGTEXT,
    /// ENUM types are not natively supported in PostgreSQL, but can be emulated with CHECK constraints or custom types. For simplicity, we won't include a native ENUM type here, but it could be added as a specialized type if needed.
    ENUM, // Emulated ENUM type (store as TEXT with CHECK constraint or custom type in PostgreSQL)
    /// Numeric Types
    /// Signed two-byte integer (range: -32768 to +32767).
    SMALLINT,
    /// Signed four-byte integer (standard integer type).
    INT,
    /// Signed eight-byte integer (standard for large IDs).
    BIGINT,
    /// Exact numeric with user-specified precision and scale.
    DECIMAL,
    /// Exact numeric with user-specified precision (alias for DECIMAL).
    NUMERIC,
    /// Single precision floating-point number (4 bytes).
    REAL,
    /// Double precision floating-point number (8 bytes).
    DOUBLE,

    // Date/Time Types
    /// Calendar date (year, month, day).
    DATE,
    /// Time of day (no time zone).
    TIME,
    /// Time of day (including time zone).
    TIMETZ,
    /// Both date and time (no time zone).
    TIMESTAMP,
    /// Both date and time (including time zone).
    TIMESTAMPTZ,
    /// Time span or duration.
    INTERVAL,

    // Boolean Type
    /// Logical Boolean (true/false/null).
    BOOLEAN,

    // Specialized Types
    /// Universally Unique Identifier (standard 128-bit UUID).
    UUID,
    /// Textual JSON data (stored as a string).
    JSON,

    // Specialized Binary Types
    /// Binary JSON data (decomposed, supports indexing).
    JSONB,
    /// Binary data ("byte array" for files/blobs).
    BYTEA,
    /// Large binary data (alias for BYTEA in PostgreSQL, LONGBLOB in MySQL, BLOB in SQLite).
    BLOB,
    MEDIUMBLOB,
    LONGBLOB,
    // Network Address Types
    /// IPv4 or IPv6 host and network address.
    INET,
    /// IPv4 or IPv6 network address (CIDR notation).
    CIDR,
    /// Media Access Control (MAC) address.
    MACADDR,

    // Geometric Types
    /// Point on a 2D plane.
    POINT,
    /// Infinite line on a 2D plane.
    LINE,
    /// Finite line segment on a 2D plane.
    LSEG,
    /// Rectangular box on a 2D plane.
    BOX,
    /// Geometric path (open or closed).
    PATH,
    /// Closed geometric path (polygon).
    POLYGON,
    /// Circle (center point and radius).
    CIRCLE,

    // Full-Text Search Types
    /// Text search document (preprocessed for searching).
    TSVECTOR,
    /// Text search query (search terms and operators).
    TSQUERY,

    // Bit String Types
    /// Fixed-length bit string.
    BIT,
    /// Variable-length bit string.
    VARBIT,

    // XML Type
    /// Validated XML data.
    XML,

    // Internal/System Types
    /// Object Identifier (PostgreSQL internal system ID).
    OID,
}

impl ColumnType {
    pub fn category(&self) -> ColumnCategory {
        match self {
            Self::VARCHAR | Self::CHAR | Self::TEXT | Self::ENUM | Self::MEDIUMTEXT | Self::LONGTEXT => ColumnCategory::Character,
            Self::SMALLINT | Self::INT | Self::BIGINT | Self::DECIMAL | Self::NUMERIC | Self::REAL | Self::DOUBLE => ColumnCategory::Numeric,
            Self::DATE | Self::TIME | Self::TIMETZ | Self::TIMESTAMP | Self::TIMESTAMPTZ | Self::INTERVAL => ColumnCategory::DateTime,
            Self::BOOLEAN => ColumnCategory::Boolean,
            Self::UUID | Self::JSON => ColumnCategory::Specialized,
            Self::JSONB => ColumnCategory::SpecializedBinary,
            Self::BYTEA | Self::BLOB | Self::MEDIUMBLOB | Self::LONGBLOB => ColumnCategory::SpecializedBinaryArray,
            Self::INET | Self::CIDR | Self::MACADDR => ColumnCategory::NetworkAddress,
            Self::POINT | Self::LINE | Self::LSEG | Self::BOX | Self::PATH | Self::POLYGON | Self::CIRCLE => ColumnCategory::Geometric,
            Self::TSVECTOR | Self::TSQUERY => ColumnCategory::FullTextSearch,
            Self::BIT | Self::VARBIT => ColumnCategory::BitString,
            Self::XML => ColumnCategory::XML,
            Self::OID => ColumnCategory::InternalSystem,
        }
    }
    pub fn is_support_length(&self) -> bool {
        match self {
            Self::VARCHAR | Self::CHAR | Self::BIT | Self::VARBIT => true,
            _ => false,
        }
    }
    pub fn is_support_scale(&self) -> bool {
        matches!(self, Self::DECIMAL | Self::NUMERIC)
    }

    pub fn is_support_collation(&self) -> bool {
        match self {
            Self::VARCHAR | Self::CHAR | Self::TEXT => true,
            _ => false,
        }
    }
    pub fn is_support_auto_increment(&self) -> bool {
        matches!(self, Self::SMALLINT | Self::INT | Self::BIGINT)
    }

    pub fn is_support_indexing(&self) -> bool {
        match self {
            Self::JSONB => true, // JSONB supports indexing in PostgreSQL
            Self::TEXT | Self::VARCHAR | Self::CHAR => true, // Text types can be indexed
            Self::SMALLINT | Self::INT | Self::BIGINT | Self::BOOLEAN => true, // Numeric and boolean types can be indexed
            _ => false,
        }
    }
    pub fn get_default_collation(&self) -> Collation {
        match self {
            Self::VARCHAR | Self::CHAR | Self::TEXT => Collation::UnicodeCaseInsensitive,
            _ => Collation::Default,
        }
    }
    pub fn get_default_length(&self) -> Option<usize> {
        match self {
            Self::VARCHAR => Some(255),
            Self::CHAR => Some(1),
            Self::BIT => Some(1),
            Self::VARBIT => Some(1),
            _ => None,
        }
    }
    pub fn get_default_scale(&self) -> Option<usize> {
        match self {
            Self::DECIMAL | Self::NUMERIC => Some(0), // Default scale is 0 for DECIMAL/NUMERIC
            _ => None,
        }
    }
    pub fn default_scale_of(&self, precision: Option<usize>) -> Option<usize> {
        match self {
            Self::DECIMAL | Self::NUMERIC => {
                if precision.is_some() {
                    Some(0) // If precision is set, scale defaults to 0
                } else {
                    None // Arbitrary precision, no fixed scale
                }
            },
            _ => None,
        }
    }
    /// Returns the maximum allowable scale for PostgreSQL NUMERIC types.
    /// PostgreSQL allows a scale up to 16383, but it cannot exceed the precision.
    pub fn max_scale_of(&self, precision: usize) -> Option<usize> {
        match self {
            Self::DECIMAL | Self::NUMERIC => Some(precision.clamp(0, 16383)), // Scale cannot exceed precision and max is 16383
            _ => None,
        }
    }

    /// Returns the minimum allowable scale.
    pub fn get_min_scale(&self) -> Option<usize> {
        match self {
            Self::DECIMAL | Self::NUMERIC => Some(0),
            _ => None,
        }
    }

    pub fn get_max_length(&self) -> Option<usize> {
        match self {
            Self::VARCHAR => Some(65535),
            Self::CHAR => Some(255),
            Self::BIT => Some(8388608), // 8 million bits
            Self::VARBIT => Some(8388608), // 8 million bits
            _ => None,
        }
    }
    pub fn get_min_length(&self) -> Option<usize> {
        match self {
            Self::VARCHAR | Self::VARBIT => Some(1),
            Self::CHAR | Self::BIT => Some(1),
            _ => None,
        }
    }
    pub fn is_support_default(&self) -> bool {
        // In PostgreSQL, all column types support default values, but the default value must be compatible with the column type. For example, you can set a default value for a JSONB column, but it must be a valid JSONB value.
        true
    }
    pub fn is_support_nullable(&self) -> bool {
        true // All types in PostgreSQL support NULL values
    }
    pub fn is_support_reference(&self) -> bool {
        match self {
            Self::JSONB | Self::BYTEA | Self::XML | Self::JSON | Self::BLOB | Self::MEDIUMBLOB | Self::LONGBLOB => false, // These types do not support foreign key references in PostgreSQL
            _ => true,
        }
    }
    pub fn is_support_unique(&self) -> bool {
        // In PostgreSQL, all column types can be part of a UNIQUE constraint, but the uniqueness is determined by the values stored in the column.
        //  However, certain types like JSONB, BYTEA, and XML may not be suitable for unique constraints due to their nature (e.g., JSONB can store complex structures that may not be easily comparable for uniqueness).
        self.is_support_primary_key()
    }
    pub fn is_support_primary_key(&self) -> bool {
        match self {
            Self::JSONB | Self::BYTEA | Self::XML | Self::JSON | Self::BLOB | Self::MEDIUMBLOB | Self::LONGBLOB => false, // These types do not support foreign key references in PostgreSQL
            _ => true,
        }
    }
    pub fn is_support_index(&self) -> bool {
        match self {
            Self::JSON | Self::XML => false,
            _ => true,
        }
    }
    pub fn is_support_unsigned(&self) -> bool {
        match self {
            Self::SMALLINT | Self::INT | Self::BIGINT => true,
            _ => false,
        }
    }
    pub fn to_sql_type(&self, driver: Driver) -> String {
        match self {
            Self::VARCHAR => "VARCHAR".to_string(),
            Self::CHAR => "CHAR".to_string(),
            Self::TEXT => "TEXT".to_string(),
            Self::MEDIUMTEXT => match driver {
                Driver::Postgresql => "TEXT".to_string(), // PostgreSQL's TEXT can handle medium/long text
                Driver::MySql => "MEDIUMTEXT".to_string(),
                Driver::Sqlite => "TEXT".to_string(),
            },
            Self::LONGTEXT => match driver {
                Driver::Postgresql => "TEXT".to_string(), // PostgreSQL's TEXT can handle medium/long text
                Driver::MySql => "LONGTEXT".to_string(),
                Driver::Sqlite => "TEXT".to_string(),
            },
            Self::SMALLINT => "SMALLINT".to_string(),
            Self::INT => "INT".to_string(),
            Self::BIGINT => "BIGINT".to_string(),
            Self::DECIMAL => "DECIMAL".to_string(),
            Self::NUMERIC => "NUMERIC".to_string(),
            Self::REAL => "REAL".to_string(),
            Self::DOUBLE => match driver {
                Driver::Postgresql => "DOUBLE PRECISION".to_string(),
                _ => "DOUBLE".to_string(),
            },
            Self::DATE => "DATE".to_string(),
            Self::TIME => match driver {
                Driver::Postgresql => "TIME WITHOUT TIME ZONE".to_string(),
                _ => "TIME".to_string(),
            },
            Self::TIMETZ => match driver {
                Driver::Postgresql => "TIME WITH TIME ZONE".to_string(),
                _ => "TIME".to_string(),
            },
            Self::TIMESTAMP => match driver {
                Driver::Postgresql => "TIMESTAMP WITHOUT TIME ZONE".to_string(),
                Driver::MySql => "DATETIME".to_string(), // MySQL DATETIME lebih umum untuk timestamp
                Driver::Sqlite => "DATETIME".to_string(),
            },
            Self::TIMESTAMPTZ => match driver {
                Driver::Postgresql => "TIMESTAMP WITH TIME ZONE".to_string(),
                _ => "DATETIME".to_string(),
            },
            Self::INTERVAL => match driver {
                Driver::Postgresql => "INTERVAL".to_string(),
                _ => "VARCHAR(255)".to_string(),
            },
            Self::BOOLEAN => "BOOLEAN".to_string(), // BOOLEAN in mysql is TINYINT(1), but we can use BOOLEAN for consistency and let the adapter handle it
            Self::UUID => match driver {
                Driver::Postgresql => "UUID".to_string(),
                Driver::MySql => "UUID".to_string(),
                Driver::Sqlite => "TEXT".to_string(),
            },
            Self::JSON => match driver {
                Driver::Sqlite => "TEXT".to_string(),
                _ => "JSON".to_string(),
            },
            Self::JSONB => match driver {
                Driver::Postgresql => "JSONB".to_string(),
                Driver::MySql => "JSON".to_string(),
                Driver::Sqlite => "TEXT".to_string(),
            },
            Self::BYTEA => match driver {
                Driver::Postgresql => "BYTEA".to_string(),
                Driver::MySql => "LONGBLOB".to_string(),
                Driver::Sqlite => "BLOB".to_string(),
            },
            Self::BLOB | Self::MEDIUMBLOB | Self::LONGBLOB => match driver {
                Driver::Postgresql => "BYTEA".to_string(),
                Driver::MySql => match self {
                    Self::BLOB => "BLOB".to_string(),
                    Self::MEDIUMBLOB => "MEDIUMBLOB".to_string(),
                    Self::LONGBLOB => "LONGBLOB".to_string(),
                    _ => unreachable!(),
                },
                Driver::Sqlite => "BLOB".to_string(),
            },
            Self::INET | Self::CIDR | Self::MACADDR => match driver {
                Driver::Postgresql => format!("{:?}", self).to_uppercase(),
                _ => "VARCHAR(50)".to_string(),
            },
            Self::POINT | Self::LINE | Self::LSEG | Self::BOX | Self::PATH | Self::POLYGON | Self::CIRCLE => match driver {
                Driver::Postgresql => format!("{:?}", self).to_uppercase(),
                Driver::MySql => "GEOMETRY".to_string(),
                Driver::Sqlite => "TEXT".to_string(),
            },
            Self::ENUM => match driver {
                Driver::Postgresql => "TEXT".to_string(),
                Driver::MySql => "ENUM".to_string(),
                Driver::Sqlite => "TEXT".to_string(),
            },
            _ => match driver {
                Driver::Postgresql => format!("{:?}", self).to_uppercase(),
                _ => "TEXT".to_string(),
            },
        }
    }

    pub fn to_sql(&self, column: &Column, driver: Driver) -> String {
        let mut sql = self.to_sql_type(driver);
        if self == &ColumnType::ENUM {
            if driver == Driver::Postgresql {
                if let Some(enum_name) = column.enum_name.as_ref() {
                    sql = format!("\"{}\"", enum_name);
                } else {
                    sql = "TEXT".to_string();
                }
                // postgre doesn't have native ENUM type, so we emulate it with TEXT + CHECK constraint. The CHECK constraint will be added in the column definition, not here in the type definition.
                 // sql = " TEXT".to_string(); // todo add identify enum type in postgre, biar bisa pakai native enum dengan CREATE TYPE
            } else if driver == Driver::MySql {
                let enums = Vec::from_iter(column.enum_values.iter().cloned().map(|v| format!("'{}'", Column::escape_literal(&v))));
                sql.push_str(&format!("({})", enums.join(", ")));
            } else {
                 let enums = Vec::from_iter(column.enum_values.iter().cloned().map(|v| format!("'{}'", Column::escape_literal(&v))));
                 sql.push_str(&format!(" CHECK (\"{}\" IN ({}))", column.name, enums.join(", ")));
            }
        } else {
            if self.is_support_scale() {
                if let (Some(p), Some(s)) = (column.length, column.scale) {
                    sql.push_str(&format!("({}, {})", p, s));
                } else if let Some(p) = column.length {
                    sql.push_str(&format!("({})", p));
                }
            } else if self.is_support_length() {
                // Untuk VARCHAR, CHAR, BIT, dll.
                if let Some(length) = column.length {
                    sql.push_str(&format!("({})", length));
                }
            }
        }
        if self.is_support_auto_increment() {
            sql.push_str(match driver {
                Driver::Postgresql => " GENERATED ALWAYS AS IDENTITY",
                Driver::MySql => " AUTO_INCREMENT",
                Driver::Sqlite => " AUTOINCREMENT",
            });
        }
        // 2. Handle Collation (Sesuai diskusi kita sebelumnya, diletakkan inline)
        if self.is_support_collation() && !column.collation.is_default() {
            if let Some(collation_name) = column.collation.as_name(driver) {
                sql.push_str(&format!(" COLLATE \"{}\"", collation_name));
            }
        }

        // 3. Handle NULL / NOT NULL
        if !column.nullable {
            sql.push_str(" NOT NULL");
        }

        // 4. Handle Default / Identity
        if let Some(default_sql) = column.default.to_sql(self.clone(), driver) {
            if column.default.is_identity() {
                sql.push_str(&format!(" {}", default_sql));
            } else {
                sql.push_str(&format!(" DEFAULT {}", default_sql));
            }
        }

        if column.unsigned && self.category().is_numeric_based() {
            if driver == Driver::Postgresql {
                sql.push_str(&format!(" CHECK (\"{}\" >= 0)", column.name));
            } else {
                sql.push_str(" UNSIGNED");
            }
        }

        sql
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum SingleColumnIndexing {
    #[default]
    None,
    PrimaryKey,
    Unique(String), // String = Constraint/Index
    Index(String),  // String = Index
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReferenceAction {
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ColumnDefault {
    /// No default value (effectively NULL if nullable).
    #[default]
    None,
    /// Explicit NULL.
    Null,
    /// Current timestamp (maps to CURRENT_TIMESTAMP or NOW()).
    Now,
    /// Auto-incrementing identity (Postgres 10+ style).
    Identity,
    /// Generates a random UUID (requires 'uuid-ossp' or pgcrypto).
    Uuid,
    /// A literal string/numeric value (The builder should handle quoting).
    Value(String),
    /// A raw SQL expression (e.g., "1 + 5" or "gen_random_uuid()").
    Expression(String),
}

impl ColumnDefault {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    pub fn is_now(&self) -> bool {
        matches!(self, Self::Now)
    }
    pub fn is_identity(&self) -> bool {
        matches!(self, Self::Identity)
    }
    pub fn is_uuid(&self) -> bool {
        matches!(self, Self::Uuid)
    }
    pub fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }
    pub fn is_expression(&self) -> bool {
        matches!(self, Self::Expression(_))
    }

    pub fn is_supported(&self, column_type: ColumnType) -> bool {
        match self {
            Self::None => true, // No default is always supported
            Self::Null => column_type.is_support_nullable(), // NULL default only valid if column supports NULL
            Self::Now => matches!(column_type.category(), ColumnCategory::DateTime), // NOW() only valid for date/time types
            Self::Identity => column_type.is_support_auto_increment(), // Identity only valid for integer types
            Self::Uuid => column_type == ColumnType::UUID, // gen_random_uuid() only valid for UUID type
            Self::Value(_) => column_type.is_support_default(), // Literal values must be compatible with the column type
            Self::Expression(_) => column_type.is_support_default(), // Expressions must be compatible with the column type
        }
    }
    pub fn to_sql(&self, column_type: ColumnType, driver: Driver) -> Option<String> {
        if !self.is_supported(column_type) || self.is_none() {
            return None;
        }
        match self {
            Self::None => None,
            Self::Null => Some("NULL".to_string()),
            Self::Now => match driver {
                Driver::Sqlite => Some("CURRENT_TIMESTAMP".to_string()),
                _ => Some("now()".to_string()), // Postgres & MySQL support now()
            },
            Self::Identity => match driver {
                Driver::Postgresql => Some("GENERATED ALWAYS AS IDENTITY".to_string()),
                Driver::MySql => None, // MySQL AUTO_INCREMENT handled in the column definition, not default value
                Driver::Sqlite => None, // SQLite AUTOINCREMENT juga ditangani di definisi kolom, not default value
            },
            Self::Uuid => match driver {
                Driver::Postgresql => Some("gen_random_uuid()".to_string()),
                Driver::MySql => Some("(UUID())".to_string()),
                Driver::Sqlite => Some("(lower(hex(randomblob(16))))".to_string()),
            },
            Self::Value(val) => Some(format!("'{}'", Column::escape_literal(val))),
            Self::Expression(expr) => {
                match driver {
                    Driver::MySql | Driver::Sqlite => Some(format!("({})", expr)),
                    _ => Some(expr.clone()),
                }
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    name: String,
    nullable: bool,
    unsigned: bool,        // Postgres mapping: CHECK (col >= 0)
    column_type: ColumnType,
    collation: Collation,
    index: SingleColumnIndexing,
    auto_increment: bool,
    default: ColumnDefault,
    comment: Option<String>,
    length: Option<usize>,      // VARCHAR(length)
    scale: Option<usize>,       // DECIMAL(length, scale) -> s
    enum_values: Vec<String>, // For ENUM types, store possible values (PostgreSQL enum emulation)
    enum_name: Option<String>, // Optional name for ENUM type (useful for PostgreSQL native enum emulation)
    driver: Driver,
}
// From

impl From<Column> for String {
    fn from(column: Column) -> Self {
        column.name.clone()
    }
}

impl From<&Column> for String {
    fn from(column: &Column) -> Self {
        column.name.clone()
    }
}

impl From<Column> for ColumnType {
    fn from(column: Column) -> Self {
        column.column_type.clone()
    }
}

impl From<&Column> for ColumnType {
    fn from(column: &Column) -> Self {
        column.column_type.clone()
    }
}

impl From<Column> for Collation {
    fn from(column: Column) -> Self {
        column.collation.clone()
    }
}

impl From<&Column> for Collation {
    fn from(column: &Column) -> Self {
        column.collation.clone()
    }
}

impl From<Column> for ColumnDefault {
    fn from(column: Column) -> Self {
        column.default.clone()
    }
}

impl From<&Column> for ColumnDefault {
    fn from(column: &Column) -> Self {
        column.default.clone()
    }
}

impl From<Column> for SingleColumnIndexing {
    fn from(column: Column) -> Self {
        column.index.clone()
    }
}

impl From<&Column> for SingleColumnIndexing {
    fn from(column: &Column) -> Self {
        column.index.clone()
    }
}

impl Deref for Column {
    type Target = ColumnType;

    fn deref(&self) -> &Self::Target {
        &self.column_type
    }
}

// Creator
impl Column {
    pub fn new<T: Into<String>>(
        name: T,
        column_type: ColumnType,
        driver: Driver,
    ) -> Self {
        Self {
            name: name.into(),
            nullable: if column_type.is_support_nullable() { true } else { false },
            unsigned: false,
            column_type: column_type.clone(),
            collation: column_type.get_default_collation(),
            auto_increment: false,
            index: SingleColumnIndexing::default(),
            default: ColumnDefault::default(),
            comment: None,
            length: column_type.get_default_length(),
            scale: column_type.get_default_scale(),
            enum_values: Vec::new(),
            enum_name: None,
            driver,
        }
    }
}

// IS
impl Column {
    pub fn escape_literal(value: &str) -> String {
        // Simple escaping for single quotes in SQL literals
        value.replace("'", "''")
    }

    pub fn is_primary_key(&self) -> bool {
        matches!(self.index, SingleColumnIndexing::PrimaryKey)
    }
     pub fn is_unique(&self) -> bool {
        matches!(self.index, SingleColumnIndexing::Unique(_))
    }
     pub fn is_indexed(&self) -> bool {
        matches!(self.index, SingleColumnIndexing::PrimaryKey | SingleColumnIndexing::Unique(_) | SingleColumnIndexing::Index(_))
    }
    pub fn is_valid_column_name(&self) -> bool {
        if self.name.is_empty() || self.name.len() > 63 {
            return false; // Postgres default limit is 63 chars
        }

        let mut chars = self.name.chars();
        let first = chars.next().unwrap();

        // Must start with letter or underscore
        if !(first.is_ascii_alphabetic() || first == '_') {
            return false;
        }

        // Followed by letters, digits, or underscores
        chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }
}

// Getter
impl Column {
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_type(&self) -> &ColumnType {
        &self.column_type
    }
    pub fn get_collation(&self) -> &Collation {
        &self.collation
    }
    pub fn get_default(&self) -> &ColumnDefault {
        &self.default
    }
    pub fn get_comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_length(&self) -> Option<usize> {
        self.length
    }
    pub fn get_scale(&self) -> Option<usize> {
        self.scale
    }
    pub fn get_index(&self) -> &SingleColumnIndexing {
        &self.index
    }
}

// set / add / remove
impl Column {
    pub fn set_default(&mut self, default: ColumnDefault) -> &mut Self {
        if default.is_supported(self.column_type.clone()) {
            self.default = default;
        }
        self
    }
    pub fn set_collation(&mut self, collation: Collation) -> &mut Self {
        if self.column_type.is_support_collation() {
            self.collation = collation;
        }
        self
    }
    pub fn set_length(&mut self, length: usize) -> &mut Self {
        if self.column_type.is_support_length() {
            let max_length = self.column_type.get_max_length().unwrap_or(u64::MAX as usize);
            let min_length = self.column_type.get_min_length().unwrap_or(0);
            self.length = Some(length.clamp(min_length, max_length));
        }
        self
    }
    pub fn set_scale(&mut self, scale: usize) -> &mut Self {
        if self.column_type.is_support_scale() {
            let max_scale = self.column_type.max_scale_of(self.length.unwrap_or(0)).unwrap_or(0);
            let min_scale = self.column_type.get_min_scale().unwrap_or(0);
            self.scale = Some(scale.clamp(min_scale, max_scale));
        }
        self
    }
     pub fn set_index(&mut self, index: SingleColumnIndexing) -> &mut Self {
        if index == SingleColumnIndexing::PrimaryKey && !self.column_type.is_support_primary_key() {
            return self; // Cannot set primary key on unsupported types
        }
        if matches!(index, SingleColumnIndexing::Unique(_) | SingleColumnIndexing::Index(_)) && !self.column_type.is_support_index() {
            return self; // Cannot set unique/index on unsupported types
        }
        self.index = index;
        self
    }
    pub fn set_comment<T: Into<String>>(&mut self, comment: T) -> &mut Self {
        self.comment = Some(comment.into());
        self
    }
    pub fn set_nullable(&mut self, nullable: bool) -> &mut Self {
        if !nullable && !self.column_type.is_support_nullable() {
            return self; // Cannot set NOT NULL on unsupported types
        }
        self.nullable = nullable;
        self
    }

    pub fn set_unsigned(&mut self, unsigned: bool) -> &mut Self {
        if !unsigned && !self.column_type.is_support_unsigned() {
            return self; // Cannot set unsigned on unsupported types
        }
        self.unsigned = unsigned;
        self
    }

    pub fn set_auto_increment(&mut self, auto_increment: bool) -> &mut Self {
        if !auto_increment && !self.column_type.is_support_auto_increment() {
            return self; // Cannot set auto-increment on unsupported types
        }
        self.auto_increment = auto_increment;
        self
    }
}

// todo: builder pattern for column creation and modification, to avoid mutable borrows and allow chaining
// column sql
impl Column {
    pub fn to_sql(&self, driver: Driver) -> String {
        let mut sql = format!("{} {}", Hack::escape_table_identifier_quote(&self.name), self.column_type.to_sql(self, driver));
        match &self.index {
            SingleColumnIndexing::PrimaryKey => sql.push_str(" PRIMARY KEY"),
            SingleColumnIndexing::Unique(name) => sql.push_str(&format!(" UNIQUE CONSTRAINT {}", name)),
            SingleColumnIndexing::Index(name) => sql.push_str(&format!(" INDEX {}", name)),
            _ => {}
        }
        if let Some(default_sql) = self.default.to_sql(self.column_type.clone(), driver) {
            sql.push_str(&format!(" DEFAULT {}", default_sql));
        }
        if let Some(comment) = &self.comment {
            if driver == Driver::Postgresql {
                // For PostgreSQL, comments are added with a separate COMMENT ON statement, so we won't include it inline here.
            } else {
                sql.push_str(&format!(" COMMENT '{}'", Column::escape_literal(comment)));
            }
        }
        sql
    }
}

#[derive(Debug, Clone)]
pub struct Index {
    pub name: String,
    pub columns: Vec<Column>, // List of column names in the index
    pub unique: bool,
    pub comment: Option<String>,
}
impl Index {
    pub fn new<T: Into<String>>(name: T, columns: Vec<Column>, unique: bool) -> Self {
        Self {
            name: name.into(),
            columns,
            unique,
            comment: None,
        }
    }
    pub fn set_comment<T: Into<String>>(&mut self, comment: T) -> &mut Self {
        self.comment = Some(comment.into());
        self
    }
    pub fn get_comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn set_columns(&mut self, columns: Vec<Column>) -> &mut Self {
        self.columns = columns;
        self
    }
     pub fn add_column(&mut self, column: Column) -> &mut Self {
         self.columns.push(column);
         self
    }
     pub fn remove_column<T: Into<String>>(&mut self, name: T) -> &mut Self {
         let name = name.into();
         self.columns.retain(|c| c.name != name);
         self
    }
     pub fn get_columns(&self) -> &Vec<Column> {
        &self.columns
    }
    pub fn to_create_sql(&self, table_name: &str, driver: Driver, skip_exists : bool) -> String {
        let columns_sql = self.columns.iter().map(|c| Hack::escape_table_identifier_quote(&c.name)).collect::<Vec<String>>().join(", ");
        let unique_str = if self.unique { "UNIQUE " } else { "" };
        format!("CREATE {}INDEX{} {} ON {} ({})",
            unique_str,
            if skip_exists {
                " IF NOT EXISTS"
            } else {
                ""
            },
            Hack::escape_table_identifier_quote(&self.name),
            Hack::escape_table_identifier_quote(table_name),
            columns_sql
        )
    }
    pub fn to_modify_sql(&self, table_name: &str, driver: Driver) -> String {
        // Note: Modifying indexes is not standardized across databases. Some databases require dropping and recreating the index.
        // For simplicity, we'll generate a DROP and CREATE statement here.
        let drop_sql = format!("DROP INDEX IF EXISTS {} ON {}", Hack::escape_table_identifier_quote(&self.name), Hack::escape_table_identifier_quote(table_name));
        let create_sql = self.to_create_sql(table_name, driver, false);
        format!("{};\n{}", drop_sql, create_sql)
    }
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub name: String,          // FK Name Constraint (eg: "fk_user_id")
    pub table: String,         // Target Table
    pub source: Column,        // Source Column
    pub target: Column,        // Target Column
    pub on_update: ReferenceAction,
    pub on_delete: ReferenceAction,
}

pub struct Table {
    name: String,
    columns: DashMap<String, Arc<Column>>,
    references: Arc<DashMap<String, Arc<Reference>>>,
    indexes: DashMap<String, Arc<Index>>,
    comment: Option<String>,
    collation: Collation,
}

impl Table {
    pub fn new<T: Into<String>>(name: T) -> Self {
        Self {
            name: name.into(),
            columns: DashMap::new(),
            references: Arc::new(DashMap::new()),
            comment: None,
            collation: Collation::default(),
            indexes: DashMap::new(),
        }
    }

    pub fn create_reference<T: Into<String>>(name: T, table: T, source: Column, target: Column, on_update: ReferenceAction, on_delete: ReferenceAction) -> ResultError<Reference> where Self: Sized {
        if !source.is_support_reference() {
            return Err(Error::unsupported(format!("Column '{}' does not support foreign key", source.name)));
        }
        if !source.is_support_reference() {
            return Err(Error::unsupported(format!("Column '{}' does not support foreign key", source.name)));
        }
        if source.column_type != target.column_type {
            return Err(Error::unsupported(format!("Column type mismatch between source '{}' and target '{}'", source.column_type.to_sql(&source, Driver::Postgresql), target.column_type.to_sql(&target, Driver::Postgresql))));
        }
        Ok(Reference {
            name: name.into(),
            table: table.into(),
            source,
            target,
            on_update,
            on_delete,
        })
    }

    pub fn insert_reference<T: Into<String>>(&self, name: T, table: T, source: Column, target: Column, on_update: ReferenceAction, on_delete: ReferenceAction) -> ResultError<Arc<Reference>> {
        let reference = Arc::new(Self::create_reference(name, table, source, target, on_update, on_delete)?);
        self.references.insert(reference.name.clone(), reference.clone());
        Ok(reference)
    }
    pub fn clear_references(&self) -> &Self {
        self.references.clear();
        self
    }
    pub fn add_reference(&self, reference: Reference) -> &Self {
        self.references.insert(reference.name.clone(), Arc::new(reference));
        self
    }
    pub fn remove_reference<T: Into<String>>(&self, name: T) -> &Self {
        self.references.remove(&name.into());
        self
    }
    pub fn get_references(&self) -> Vec<Arc<Reference>> {
        self.references.iter().map(|e| e.value().clone()).collect::<Vec<Arc<Reference>>>()
    }
    pub fn get_reference<T: Into<String>>(&self, name: T) -> Option<Arc<Reference>> {
        self.references.get(&name.into()).map(|e| e.value().clone())
    }
    pub fn has_reference<T: Into<String>>(&self, name: T) -> bool {
        self.references.contains_key(&name.into())
    }
    pub fn set_comment<T: Into<String>>(&mut self, comment: T) -> &mut Self {
        self.comment = Some(comment.into());
        self
    }
    pub fn get_comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn add_column(&self, column: Column) -> &Self {
        self.columns.insert(column.name.clone(), Arc::new(column));
        self
    }
    pub fn remove_column<T: Into<String>>(&self, name: T) -> &Self {
        self.columns.remove(&name.into());
        self
    }
    pub fn get_column<T: Into<String>>(&self, name: T) -> Option<Arc<Column>> {
        self.columns.get(&name.into()).map(|e| e.value().clone())
    }
    pub fn get_columns(&self) -> Vec<Arc<Column>> {
        self.columns.iter().map(|e| e.value().clone()).collect::<Vec<Arc<Column>>>()
    }
    pub fn set_collation(&mut self, collation: Collation) -> &mut Self {
        self.collation = collation;
        self
    }
    pub fn get_collation(&self) -> &Collation {
        &self.collation
    }
    pub fn has_column<T: Into<String>>(&self, name: T) -> bool {
        self.columns.contains_key(&name.into())
    }
    pub fn has_index<T: Into<String>>(&self, name: T) -> bool {
        self.indexes.contains_key(&name.into())
    }
     pub fn get_index<T: Into<String>>(&self, name: T) -> Option<Arc<Index>> {
        self.indexes.get(&name.into()).map(|e| e.value().clone())
    }
     pub fn get_indexes(&self) -> Vec<Arc<Index>> {
        self.indexes.iter().map(|e| e.value().clone()).collect::<Vec<Arc<Index>>>()
    }
    pub fn add_index(&self, index: Index) -> &Self {
        self.indexes.insert(index.name.clone(), Arc::new(index));
        self
    }
    pub fn remove_index<T: Into<String>>(&self, name: T) -> &Self {
        self.indexes.remove(&name.into());
        self
    }
    pub fn to_create_table_sql(&self, driver: Driver, skip_exists: bool) -> String {
        let columns_sql = self.columns.iter().map(|c| c.to_sql(driver)).collect::<Vec<String>>().join(",\n");
        let mut str = format!(
            "CREATE{}TABLE {} ({})",
            if skip_exists { " IF NOT EXISTS " } else { " " },
            Hack::escape_table_identifier_quote(&self.name),
            columns_sql
        );
        if let Some(comment) = &self.comment {
            if driver == Driver::Postgresql {
                str.push_str(&format!("; COMMENT ON TABLE {} IS '{}'", self.name, Column::escape_literal(comment)));
            } else {
                str.push_str(&format!(" COMMENT='{}'", Column::escape_literal(comment)));
            }
        }
        // add colation for table
        if !self.collation.is_default() {
            if let Some(collation_name) = self.collation.as_name(driver) {
                str.push_str(&format!(" COLLATE \"{}\"", collation_name));
            }
        }

        str.push_str("; ");
        if driver != Driver::Postgresql {
            let comments_column = Vec::from_iter(self.columns.iter().filter_map(|c| c.get_comment().map(|comment| format!("COMMENT ON COLUMN {}.{} IS '{}'", self.name, c.name, Column::escape_literal(comment)))));
            if !comments_column.is_empty() {
                str.push_str("; ");
                str.push_str(&comments_column.join("; "));
            }
        }
        /// check if has foreign
        if self.references.len() > 0 {
            let foreign_keys_sql = self.references.iter().map(|entry| {
                let reference = entry.value();
                format!(
                    "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}({}) ON UPDATE {} ON DELETE {};",
                    self.name,
                    &Hack::escape_table_identifier_quote(&reference.name),
                    &Hack::escape_table_identifier_quote(&reference.source.name),
                    &Hack::escape_table_identifier_quote(&reference.table),
                    &Hack::escape_table_identifier_quote(&reference.target.name),
                    match reference.on_update {
                        ReferenceAction::NoAction => "NO ACTION",
                        ReferenceAction::Restrict => "RESTRICT",
                        ReferenceAction::Cascade => "CASCADE",
                        ReferenceAction::SetNull => "SET NULL",
                        ReferenceAction::SetDefault => match driver {
                            Driver::Postgresql => "SET DEFAULT",
                            _ => "SET NULL", // MySQL and SQLite do not support SET DEFAULT for foreign keys, so we use SET NULL instead
                        },
                    },
                    match reference.on_delete {
                        ReferenceAction::NoAction => "NO ACTION",
                        ReferenceAction::Restrict => "RESTRICT",
                        ReferenceAction::Cascade => "CASCADE",
                        ReferenceAction::SetNull => "SET NULL",
                        ReferenceAction::SetDefault => match driver {
                            Driver::Postgresql => "SET DEFAULT",
                            _ => "SET NULL", // MySQL and SQLite do not support SET DEFAULT for foreign keys, so we use SET NULL instead
                        },
                    }
                )
            }).collect::<Vec<String>>().join("\n");
            str.push_str(&foreign_keys_sql);
        }
        if self.indexes.len() > 0 {
            let indexes_sql = self.indexes.iter().map(|entry| entry.value().to_create_sql(&self.name, driver, skip_exists)).collect::<Vec<String>>().join("\n");
            str.push_str(&indexes_sql);
        }
        str
    }
}
