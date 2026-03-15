use std::{fmt::Display, ops::Deref, sync::Arc};

use dashmap::DashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Collation {
    UnicodeCaseSensitive, // Unicode collation (e.g., "en_US.UTF-8" with Unicode settings, case-sensitive)
    UnicodeCaseInsensitive, // Unicode collation (e.g., "en_US.UTF-8" with Unicode settings)
    CaseSensitive, // Case-sensitive collation (e.g., "en_US.UTF-8" with case-sensitive settings)
    CaseInsensitive, // Case-insensitive collation (e.g., "en_US.UTF-8" with case-insensitive settings)
    Binary, // Binary collation (byte-by-byte comparison, case-sensitive)
    Default, // Use database default collation (e.g., "en_US.UTF-8" or "C" depending on the database configuration)
    Custom(String), // Custom collation name (e.g., "en_US", "fr_FR", "und-u-ks-level3-x-icu", etc.)
}

impl Collation {
    pub fn to_str(&self) -> &str {
        match self {
            Self::UnicodeCaseSensitive => "und-u-ks-level3-x-icu",
            Self::UnicodeCaseInsensitive|Self::CaseInsensitive => "und-u-ks-level2-x-icu",
            Self::Binary|Self::CaseSensitive => "C",
            Self::Default => "default",
            Self::Custom(s) => &s,
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

impl Display for Collation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
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

    // Numeric Types
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
            Self::VARCHAR | Self::CHAR | Self::TEXT => ColumnCategory::Character,
            Self::SMALLINT | Self::INT | Self::BIGINT | Self::DECIMAL | Self::NUMERIC | Self::REAL | Self::DOUBLE => ColumnCategory::Numeric,
            Self::DATE | Self::TIME | Self::TIMETZ | Self::TIMESTAMP | Self::TIMESTAMPTZ | Self::INTERVAL => ColumnCategory::DateTime,
            Self::BOOLEAN => ColumnCategory::Boolean,
            Self::UUID | Self::JSON => ColumnCategory::Specialized,
            Self::JSONB => ColumnCategory::SpecializedBinary,
            Self::BYTEA => ColumnCategory::SpecializedBinaryArray,
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
            Self::JSONB | Self::BYTEA | Self::XML => false, // These types do not support foreign key references in PostgreSQL
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
            Self::JSONB | Self::BYTEA | Self::XML => false, // These types do not support primary key constraints in PostgreSQL
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

    pub fn to_sql_type(&self) -> String {
        match self {
            Self::VARCHAR => "VARCHAR".to_string(),
            Self::CHAR => "CHAR".to_string(),
            Self::TEXT => "TEXT".to_string(),
            Self::SMALLINT => "SMALLINT".to_string(),
            Self::INT => "INT".to_string(),
            Self::BIGINT => "BIGINT".to_string(),
            Self::DECIMAL => "DECIMAL".to_string(),
            Self::NUMERIC => "NUMERIC".to_string(),
            Self::REAL => "REAL".to_string(),
            Self::DOUBLE => "DOUBLE PRECISION".to_string(),
            Self::DATE => "DATE".to_string(),
            Self::TIME => "TIME WITHOUT TIME ZONE".to_string(),
            Self::TIMETZ => "TIME WITH TIME ZONE".to_string(),
            Self::TIMESTAMP => "TIMESTAMP WITHOUT TIME ZONE".to_string(),
            Self::TIMESTAMPTZ => "TIMESTAMP WITH TIME ZONE".to_string(),
            Self::INTERVAL => "INTERVAL".to_string(),
            Self::BOOLEAN => "BOOLEAN".to_string(),
            Self::UUID => "UUID".to_string(),
            Self::JSON => "JSON".to_string(),
            Self::JSONB => "JSONB".to_string(),
            Self::BYTEA => "BYTEA".to_string(),
            Self::INET => "INET".to_string(),
            Self::CIDR => "CIDR".to_string(),
            Self::MACADDR => "MACADDR".to_string(),
            Self::POINT => "POINT".to_string(),
            Self::LINE => "LINE".to_string(),
            Self::LSEG => "LSEG".to_string(),
            Self::BOX => "BOX".to_string(),
            Self::PATH => "PATH".to_string(),
            Self::POLYGON => "POLYGON".to_string(),
            Self::CIRCLE => "CIRCLE".to_string(),
            Self::TSVECTOR => "TSVECTOR".to_string(),
            Self::TSQUERY => "TSQUERY".to_string(),
            Self::BIT => "BIT".to_string(),
            Self::VARBIT => "VARBIT".to_string(),
            Self::XML => "XML".to_string(),
            Self::OID => "OID".to_string(),
        }
    }
    pub fn to_sql(&self, column: &Column) -> String {
        let mut sql = self.to_sql_type();

        // 1. Handle Precision & Scale secara bersamaan agar tidak duplikat kurung
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

        // 2. Handle Collation (Sesuai diskusi kita sebelumnya, diletakkan inline)
        if self.is_support_collation() && !column.collation.is_default() {
            sql.push_str(&format!(" COLLATE \"{}\"", column.collation.to_str()));
        }

        // 3. Handle NULL / NOT NULL
        if !column.nullable {
            sql.push_str(" NOT NULL");
        }

        // 4. Handle Default / Identity
        if let Some(default_sql) = column.default.to_sql(self.clone()) {
            if column.default.is_identity() {
                sql.push_str(&format!(" {}", default_sql));
            } else {
                sql.push_str(&format!(" DEFAULT {}", default_sql));
            }
        }

        if column.unsigned && self.category().is_numeric_based() {
            sql.push_str(&format!(" CHECK (\"{}\" >= 0)", column.name));
        }

        sql
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Indexing {
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

    pub fn to_sql(&self, column_type: ColumnType) -> Option<String> {
        if !self.is_supported(column_type) || self.is_none() {
            return None; // No default value
        }
        match self {
            Self::None => None,
            Self::Null => Some("NULL".to_string()),
            Self::Now => Some("CURRENT_TIMESTAMP".to_string()),
            Self::Identity => Some("GENERATED ALWAYS AS IDENTITY".to_string()),
            Self::Uuid => Some("gen_random_uuid()".to_string()), // Requires 'uuid-ossp' or pgcrypto extension
            Self::Value(val) => Some(format!("'{}'", Column::escape_literal(val))), // The builder should handle quoting/escaping as needed
            Self::Expression(expr) => Some(expr.clone()), // The builder should ensure this is a valid SQL expression
        }
    }
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub name: String,          // FK Name Constraint (eg: "fk_user_id")
    pub table: String,         // Target Table
    pub column: String,        // Target Column
    pub on_update: ReferenceAction,
    pub on_delete: ReferenceAction,
}

#[derive(Debug, Clone)]
pub struct Column {
    name: String,
    nullable: bool,
    unsigned: bool,        // Postgres mapping: CHECK (col >= 0)
    column_type: ColumnType,
    collation: Collation,
    index: Indexing,
    auto_increment: bool,
    default: ColumnDefault,
    comment: Option<String>,
    length: Option<usize>,      // VARCHAR(length)
    scale: Option<usize>,       // DECIMAL(length, scale) -> s
    references: Arc<DashMap<String, Arc<Reference>>>,
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

impl From<Column> for Indexing {
    fn from(column: Column) -> Self {
        column.index.clone()
    }
}

impl From<&Column> for Indexing {
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
    pub fn new<T: Into<String>>(name: T, column_type: ColumnType) -> Self {
        Self {
            name: name.into(),
            nullable: if column_type.is_support_nullable() { true } else { false },
            unsigned: false,
            column_type: column_type.clone(),
            collation: column_type.get_default_collation(),
            auto_increment: false,
            index: Indexing::default(),
            default: ColumnDefault::default(),
            comment: None,
            length: column_type.get_default_length(),
            scale: column_type.get_default_scale(),
            references: Arc::new(DashMap::new()),
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
        matches!(self.index, Indexing::PrimaryKey)
    }
     pub fn is_unique(&self) -> bool {
        matches!(self.index, Indexing::Unique(_))
    }
     pub fn is_indexed(&self) -> bool {
        matches!(self.index, Indexing::PrimaryKey | Indexing::Unique(_) | Indexing::Index(_))
    }
    pub fn is_has_foreign_key(&self) -> bool {
        self.column_type.is_support_reference() && !self.references.is_empty()
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
    pub fn get_index(&self) -> &Indexing {
        &self.index
    }
    pub fn get_references(&self) -> Vec<Arc<Reference>> {
        self.references.iter().map(|e| e.value().clone()).collect::<Vec<Arc<Reference>>>()
    }
    pub fn get_reference<T: Into<String>>(&self, name: T) -> Option<Arc<Reference>> {
        self.references.get(&name.into()).map(|e| e.value().clone())
    }
}

// set / add / remove
impl Column {
    pub fn clear_references(&self) -> &Self {
        self.references.clear();
        self
    }
    pub fn add_reference(&self, reference: Reference) -> &Self {
        if self.column_type.is_support_reference() {
            self.references.insert(reference.name.clone(), Arc::new(reference));
        }
        self
    }
    pub fn remove_reference<T: Into<String>>(&self, name: T) -> &Self {
        self.references.remove(&name.into());
        self
    }
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
     pub fn set_index(&mut self, index: Indexing) -> &mut Self {
        if index == Indexing::PrimaryKey && !self.column_type.is_support_primary_key() {
            return self; // Cannot set primary key on unsupported types
        }
        if matches!(index, Indexing::Unique(_) | Indexing::Index(_)) && !self.column_type.is_support_index() {
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
    pub fn to_foreign_key_sql(&self) -> Vec<String> {
        self.references.iter().map(|entry| {
            let reference = entry.value();
            format!(
                "FOREIGN KEY ({}) REFERENCES {}({}) ON UPDATE {} ON DELETE {}",
                self.name,
                reference.table,
                reference.column,
                match reference.on_update {
                    ReferenceAction::NoAction => "NO ACTION",
                    ReferenceAction::Restrict => "RESTRICT",
                    ReferenceAction::Cascade => "CASCADE",
                    ReferenceAction::SetNull => "SET NULL",
                    ReferenceAction::SetDefault => "SET DEFAULT",
                },
                match reference.on_delete {
                    ReferenceAction::NoAction => "NO ACTION",
                    ReferenceAction::Restrict => "RESTRICT",
                    ReferenceAction::Cascade => "CASCADE",
                    ReferenceAction::SetNull => "SET NULL",
                    ReferenceAction::SetDefault => "SET DEFAULT",
                }
            )
        }).collect()
    }

    pub fn to_comment_sql(&self, table_name: &str) -> Option<String> {
        self.comment.as_ref().map(|comment| {
            format!(
                "COMMENT ON COLUMN {}.{} IS '{}'",
                table_name,
                self.name,
                Column::escape_literal(comment)
            )
        })
    }

    pub fn to_sql(&self) -> String {
        let mut sql = format!("{} {}", self.name, self.column_type.to_sql(self));
        match &self.index {
            Indexing::PrimaryKey => sql.push_str(" PRIMARY KEY"),
            Indexing::Unique(name) => sql.push_str(&format!(" UNIQUE CONSTRAINT {}", name)),
            Indexing::Index(name) => sql.push_str(&format!(" INDEX {}", name)),
            _ => {}
        }
        if let Some(default_sql) = self.default.to_sql(self.column_type.clone()) {
            sql.push_str(&format!(" DEFAULT {}", default_sql));
        }
        if let Some(comment) = &self.comment {
            sql.push_str(&format!(" COMMENT '{}'", Column::escape_literal(comment)));
        }
        sql
    }
}
