use crate::cores::system::error::{Error, ResultError};

/// A trait for serializing objects into JSON string or JSON value representations.
///
/// The `ToJson` trait provides methods to create JSON representations of implementing
/// objects, with flexibility to include only public fields or both public and private fields
/// in the output. This trait is designed to work with the `serde_json` crate.
///
/// # Example Implementations
///
/// Structs implementing the `ToJson` trait should define the logic for converting
/// an object into a format suitable for serialization using the provided methods:
///
/// ```rust
/// use serde_json::{json, Value};
/// use my_crate::ToJson;
///
/// struct MyStruct {
///     pub field1: String,
///     private_field: usize,
/// }
///
/// impl ToJson for MyStruct {
///     fn to_json(&self, public: bool) -> Result<serde_json::Value, Error> {
///         if public {
///             Ok(json!({
///                 "field1": self.field1
///             }))
///         } else {
///             Ok(json!({
///                 "field1": self.field1,
///                 "private_field": self.private_field
///             }))
///         }
///     }
///
///     fn to_json_string(&self, public: bool) -> Result<String, Error> {
///         serde_json::to_string(&self.to_json(public)?).map_err(|e| Error::parse_error(e))
///     }
/// }
///
/// let my_object = MyStruct {
///     field1: String::from("value1"),
///     private_field: 42,
/// };
///
/// let public_json = my_object.to_json(true).unwrap();
/// println!("Public JSON: {}", public_json);
///
/// let json_str = my_object.to_json_string(false).unwrap();
/// println!("Private JSON String: {}", json_str);
pub trait ToJson {
    /// Converts the current object into a JSON string representation.
    ///
    /// # Parameters
    /// - `public`: A boolean that determines the visibility or scope of the fields to be serialized.
    ///   If `true`, only public fields or data are included in the serialized output. If `false`,
    ///   a more detailed representation including private fields may be included, depending on the
    ///   implementation of the `to_json` method.
    ///
    /// # Returns
    /// - `Ok(String)`: A JSON string representation of the object if serialization is successful.
    /// - `Err(Error)`: An error wrapped in a custom `Error` type if serialization fails, either due
    ///   to issues in creating the intermediate JSON object or during the string serialization process.
    ///
    /// # Errors
    /// - This function returns an error if:
    ///   - The intermediate JSON object creation (`self.to_json(public)`) fails.
    ///   - The JSON serialization process (`serde_json::to_string`) encounters a failure.
    ///
    /// # Example
    /// ```rust
    /// let my_object = MyStruct::new();
    /// match my_object.to_json_string(true) {
    ///     Ok(json_str) => println!("Serialized JSON: {}", json_str),
    ///     Err(e) => eprintln!("Error serializing object: {:?}", e),
    /// }
    /// ```
    fn to_json_string(&self, public: bool) -> ResultError<String> {
        serde_json::to_string(&self.to_json(public)?).map_err(|e| Error::parse_error(e))
    }

    /// Converts the object into a JSON representation.
    ///
    /// # Parameters
    /// - `public` (bool): Determines whether to include only public fields or both public and private
    ///   fields in the JSON output.
    ///   - If `true`, only public fields are included.
    ///   - If `false`, both public and private fields are included.
    ///
    /// # Returns
    /// - `Result<serde_json::Value, Error>`: On success, returns the serialized JSON representation
    ///   of the object as `serde_json::Value`. On failure, returns an `Error` indicating what went wrong.
    ///
    /// # Errors
    /// This function may return an error in the following cases:
    /// - Serialization fails due to incompatible data types.
    /// - An underlying issue with `serde_json` prevents proper encoding.
    ///
    /// # Examples
    /// ```rust
    /// let my_object = MyStruct::new();
    /// let json_public = my_object.to_json(true)?;
    /// let json_private = my_object.to_json(false)?;
    /// println!("Public JSON: {}", json_public);
    /// println!("Private JSON: {}", json_private);
    /// ```
    fn to_json(&self, public: bool) -> ResultError<serde_json::Value>;
}
