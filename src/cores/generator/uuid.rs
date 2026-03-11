use crate::cores::system::error::{Error, ResultError};
use core::convert::AsRef;
use core::fmt::Display;
use core::ops::Deref;
use uuid::Uuid as UuidCrate;

// Regex to validate UUIDs of versions 1-8
// static UUID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
//     Regex::new(
//         r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
//     )
//     .unwrap()
// });

/// Represents different versions of the UUID crate used within the application.
///
/// This enum allows specifying the version of the UUID crate and associates it with
/// a specific crate version (e.g., V4 or V7).
///
/// # Variants
///
/// - `V4(UuidCrate)`
///   Represents UUID version 4 functionality, associated with a specific version
///   of the `UuidCrate`.
///
/// - `V7(UuidCrate)`
///   Represents UUID version 7 functionality, associated with a specific version
///   of the `UuidCrate`.
///
/// # Attributes
///
/// - `UuidCrate`
///   A type that represents the specific version or configuration of the UUID crate.
///
/// # Derives
///
/// - `Debug`: Enables formatting the enum using the `{:?}` formatter.
/// - `Clone`: Allows creating a copy of the enum value.
/// - `Copy`: Allows the enum value to be copied instead of being moved.
///
/// # Example
///
/// ```rust
/// use your_crate::UuidCrateVersion;
///
/// let uuid_v4 = UuidCrateVersion::V4(UuidCrate::default());
/// let uuid_v7 = UuidCrateVersion::V7(UuidCrate::default());
///
/// println!("{:?}", uuid_v4);
/// println!("{:?}", uuid_v7);
/// ```
#[derive(Debug, Clone, Copy)]
pub enum UuidCrateVersion {
    V4(UuidCrate),
    V7(UuidCrate),
}

impl UuidCrateVersion {
    pub fn new_v4() -> Self {
        Self::V4(UuidCrate::new_v4())
    }
    pub fn new_v7() -> Self {
        Self::V7(UuidCrate::now_v7())
    }
    pub fn type_name(&self) -> &'static str {
        match self.type_id() {
            4 => "UUIDv4",
            7 => "UUIDv7",
            _ => "Unknown UUID Version",
        }
    }
    /// Returns the type identifier (`u8`) for the current enum variant.
    ///
    /// # Variants and their corresponding type IDs:
    /// - `Self::V4(_)` - Returns `4`
    /// - `Self::V7(_)` - Returns `7`
    ///
    /// # Examples
    /// ```
    /// let v4 = MyEnum::V4(value);
    /// assert_eq!(v4.type_id(), 4);
    ///
    /// let v7 = MyEnum::V7(value);
    /// assert_eq!(v7.type_id(), 7);
    /// ```
    ///
    /// # Panics
    /// This function does not panic.
    ///
    /// # Note
    /// The function assumes that all enum variants are matched (exhaustive match),
    /// ensuring that the returned value always represents a valid type ID.
    pub fn type_id(&self) -> u8 {
        match self {
            Self::V4(_) => 4,
            Self::V7(_) => 7,
        }
    }

    /// Returns a reference to the inner `UuidCrate` instance.
    ///
    /// This method provides access to the inner `UuidCrate` object stored
    /// within the enum variant. It supports both `V4` and `V7` variants,
    /// returning a reference to the shared inner value.
    ///
    /// # Returns
    /// A reference to the `UuidCrate` object inside the `Self` instance.
    ///
    /// # Examples
    /// ```
    /// match uuid_instance.inner() {
    ///     uuid => println!("Inner UUID: {:?}", uuid),
    /// }
    /// ```
    pub fn inner(&self) -> &UuidCrate {
        match self {
            Self::V4(inner) | Self::V7(inner) => inner,
        }
    }
}

impl Deref for UuidCrateVersion {
    type Target = UuidCrate;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl Display for UuidCrateVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.inner(), f)
    }
}

/// A struct representing a Universally Unique Identifier (UUID).
///
/// The `Uuid` struct is typically used to generate and work with UUIDs,
/// which are 128-bit values designed to uniquely identify information
/// in distributed systems without significant risk of collision.
///
/// UUIDs are widely used in databases, network protocols, and systems
/// that require a unique identifier for objects, sessions, or records.
///
/// # Examples
///
/// ```rust
/// use some_crate::Uuid; // Replace with actual crate/module
///
/// // Example usage of Uuid (depending on full implementation details):
/// let unique_id = Uuid::v4(); // Create a new v4 UUID
/// println!("{:?}", unique_id);
/// ```
///
/// # Note
/// The `Uuid` struct provided here is a placeholder for additional functionality.
/// To generate or parse UUIDs, methods typically need to be implemented (e.g., `new_v4`, `parse_str`).
///
/// # References
/// - [RFC 4122: A Universally Unique Identifier (UUID) URN Namespace](https://www.rfc-editor.org/rfc/rfc4122)
/// - [Wikipedia: Universally unique identifier](https://en.wikipedia.org/wiki/Universally_unique_identifier)
pub struct Uuid;

/// UUID Helper that implements methods to generate UUIDs and validate UUID strings.
impl Uuid {
    /// Check if a string is a valid UUID
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::uuid::Uuid;
    /// let is_valid = Uuid::valid("123e4567-e89b-12d3-a456-426614174000");
    /// println!("Is valid UUID: {}", is_valid);
    /// ```
    pub fn valid<T: AsRef<str>>(u: T) -> bool {
        UuidCrate::parse_str(u.as_ref()).is_ok()
        // UUID_REGEX.is_match(uuid.as_ref())
    }

    /// Parse a UUID from a string
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::uuid::Uuid;
    /// let uuid = Uuid::parse("123e4567-e89b-12d3-a456-426614174000").unwrap();
    /// println!("Parsed UUID: {}", uuid);
    /// ```
    pub fn parse<T: AsRef<str>>(uuid: T) -> ResultError<UuidCrate> {
        UuidCrate::parse_str(uuid.as_ref())
            .map_err(|_| Error::invalid_input(format!("Invalid UUID for: {}", uuid.as_ref())))
    }

    /// Generate a new UUIDv7
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::uuid::Uuid;
    /// let uuid = Uuid::v7();
    /// println!("Generated UUIDv7: {}", uuid);
    /// ```
    pub fn v7() -> UuidCrateVersion {
        UuidCrateVersion::new_v7()
    }

    /// Generate a new UUIDv4
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::uuid::Uuid;
    /// let uuid = Uuid::v4();
    /// println!("Generated UUIDv4: {}", uuid);
    /// ```
    pub fn v4() -> UuidCrateVersion {
        UuidCrateVersion::new_v4()
    }
}
