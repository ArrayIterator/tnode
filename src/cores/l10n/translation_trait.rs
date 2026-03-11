use crate::cores::i18n::country_language::CountryLanguageData;
use crate::cores::l10n::plural::Plural;
use std::any::TypeId;
use std::collections::HashMap;
use std::fmt::Debug;

/// Macro to join translation or string with null character as separator
/// # Example
/// ```rust
/// use crate::cores::l10n::translation::join_translation;
/// let joined : String = null_join!("Hello", "World", "!");
/// assert_eq!(joined, "Hello\0World\0!");
/// ```
#[macro_export]
macro_rules! translation_key_join {
    ( $( $x:expr ),* ) => {
        {
            let mut s = String::new();
            $(
                if !s.is_empty() {
                    s.push('\0');
                }
                s.push_str($x.as_ref());
            )*
            s
        }
    };
}

#[derive(Debug, Clone)]
pub struct TranslationData {
    // Translation key
    id: String,
    // Translation plural id
    plural_id: Option<String>,
    // Translation domain
    domain: String,
    // Translation context
    context: String,
    // Singular translation
    singular: String,
    // Plural translations
    plural: Vec<String>,
}

impl TranslationData {
    pub fn new<Id, PluralId, Domain, Singular, Ctx, Plural>(
        id: Id,
        plural_id: Option<PluralId>,
        domain: Domain,
        context: Ctx,
        singular: Singular,
        plural: Vec<Plural>,
    ) -> Self
    where
        Id: AsRef<str>,
        PluralId: AsRef<str>,
        Domain: AsRef<str>,
        Singular: AsRef<str>,
        Ctx: AsRef<str>,
        Plural: AsRef<str>,
    {
        Self {
            id: id.as_ref().to_string(),
            plural_id: plural_id.map(|s| s.as_ref().to_string()),
            domain: domain.as_ref().to_string(),
            context: context.as_ref().to_string(),
            singular: singular.as_ref().to_string(),
            plural: plural.into_iter().map(|s| s.as_ref().to_string()).collect(),
        }
    }
}

pub trait TranslationClone {
    fn clone_box(&self) -> Box<dyn TranslationAdapter>;
}

impl<T> TranslationClone for T
where
    T: 'static + TranslationAdapter + Clone,
{
    fn clone_box(&self) -> Box<dyn TranslationAdapter> {
        Box::new(self.clone())
    }
}

pub trait TranslationAdapter: TranslationClone + Debug + Send + Sync + 'static {
    fn get_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// Create a new translation adapter with country language data
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country_language::CountryLanguageData;
    /// use crate::cores::l10n::translation::TranslationAdapter;
    /// struct MyAdapter;
    /// impl<Str: AsRef<str>> TranslationAdapter<Str> for MyAdapter {
    ///     fn new(language_code: CountryLanguageData) -> Self {
    ///         MyAdapter
    ///     }
    ///     // implement other methods...
    /// }
    /// ```
    fn new(language_data: CountryLanguageData) -> Self
    where
        Self: Sized;

    /// Retrieves a collection of translation lists.
    ///
    /// This method returns a `HashMap` where each key is a `String` representing
    /// a specific language or category, and the corresponding value is
    /// a `TranslationData` object containing the translations associated with it.
    ///
    /// # Returns
    ///
    /// A `HashMap<String, TranslationData>` where:
    /// - The `String` keys denote the category or language.
    /// - The `TranslationData` values store the actual translations or additional metadata.
    ///
    /// # Example
    ///
    /// ```rust
    /// let translation_lists = my_object.get_translation_lists();
    /// for (language, data) in &translation_lists {
    ///     println!("Language/Category: {}", language);
    ///     // Perform operations with `data`
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// - Ensure that the returned `HashMap` is not empty before performing any operations.
    /// - The structure and content of the `TranslationData` will depend on its implementation.
    ///
    /// # Errors
    ///
    /// This function assumes that the calling instance (`self`) maintains and correctly
    /// populates its internal data structures; hence, no explicit errors are returned.
    fn get_translation_lists(&self) -> HashMap<String, TranslationData>;

    /// Retrieves a mutable reference to the `HashMap` containing `TranslationData`.
    ///
    /// # Returns
    /// A mutable reference to the `HashMap` where:
    /// - The keys are `String` values used as identifiers.
    /// - The values are of type `TranslationData`, representing translation-related data.
    ///
    /// # Usage
    /// This function allows modification of the internal `HashMap` structure,
    /// including adding, updating, or removing elements.
    ///
    /// # Example
    /// ```rust
    /// let mut my_struct = MyStruct::new();
    /// let lists = my_struct.get_lists_mut();
    /// lists.insert(String::from("key"), TranslationData::new());
    /// ```
    ///
    /// # Notes
    /// - Make sure that the caller has mutable ownership of the containing structure,
    ///   as this function requires a mutable reference to `self`.
    /// - Modifying the `HashMap` through this method will directly affect the internal state of the struct.
    fn get_lists_mut(&mut self) -> &mut HashMap<String, TranslationData>;

    /// Get country language data
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// use crate::cores::l10n::translation::Translation;
    /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(lang_data) = CountryLanguage::get("US").await {
    ///         let translation = Translation::from_language(lang_data.clone());
    ///         let lang_info = translation.get_language_data();
    ///         println!("Language Code: {}", lang_info.code);
    ///     }
    /// }
    /// ```
    fn get_language_data(&self) -> &CountryLanguageData;

    /// Get translation data by key, domain, and context
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// use crate::cores::l10n::translation::Translation;
    /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(lang_data) = CountryLanguage::get("US").await {
    ///         let translation = Translation::from_language(lang_data.clone());
    ///         if let Some(data) = translation.get("Hello", "default", "") {
    ///             println!("Translation Data: {:?}", data);
    ///         }
    ///     }
    /// }
    /// ```
    fn get(&self, id: &str, domain: &str, context: &str) -> Option<TranslationData> {
        let key = translation_key_join!(id, context, domain);
        self.get_translation_lists().get(&key).cloned()
    }

    /// Retrieves the singular form of a localized string based on the provided key, domain, and context.
    ///
    /// # Type Parameters
    /// - `T`: A type that implements the `AsRef<str>` trait, allowing conversion to a string slice.
    ///
    /// # Parameters
    /// - `key`: The key identifying the localized string. It is used to look up the desired string.
    /// - `domain`: The domain or category under which the localized string is grouped.
    /// - `context`: Additional identifying information to disambiguate contexts where the same key might
    ///   have different meanings.
    ///
    /// # Returns
    /// - `Option<String>`: An `Option` containing the singular form of the localized string if found, or
    ///   `None` if no matching string is found for the given key, domain, and context.
    ///
    /// # Examples
    /// ```
    /// let localization = Adapter::new();
    /// if let Some(singular) = localization.singular("hello", "messages", "greeting") {
    ///     println!("Localized singular form: {}", singular);
    /// } else {
    ///     println!("Localization not found.");
    /// }
    /// ```
    fn singular(&self, id: &str, domain: &str, context: &str) -> Option<String> {
        let key = id.as_ref();
        let domain = domain.as_ref();
        let context = context.as_ref();
        if let Some(data) = self.get(key, domain, context) {
            Some(data.singular)
        } else {
            None
        }
    }

    /// Translates a given singular and plural form of a string based on the provided count, domain,
    /// and context, while taking into account the current language's pluralization rules.
    ///
    /// # Type Parameters
    /// - `T`: A type that implements the `AsRef<str>` trait, used to work with string-like values.
    /// - `C`: A type that implements the `ToPluralCount` trait, used to determine the pluralization count.
    ///
    /// # Parameters
    /// - `singular`: The singular form of the string to be translated.
    /// - `plural`: The plural form of the string to be translated.
    /// - `count`: A value representing the count used to determine the pluralization form.
    /// - `domain`: The domain for the translation, allowing for grouping or scoping translations.
    /// - `context`: Additional contextual information for resolving the correct translation.
    ///
    /// # Returns
    /// A `String` containing the appropriate translation based on the count, language data,
    /// and available translations. If a translation is available in the system's data, it will
    /// return the corresponding pluralized string. If no translation is found, it falls back to
    /// the singular or plural form provided, depending on the count.
    ///
    /// # Internal Behavior
    /// - Resolves the pluralization index using a language's specific pluralization rules by invoking
    ///   the `Plural::resolve` method.
    /// - Searches for a translation in the system's data using the singular key, domain, and context.
    /// - If a translation exists and the resolved plural index is within bounds, the corresponding
    ///   pluralized string is returned.
    /// - If no translation is found or the plural index is out of bounds, it falls back to the
    ///   provided `singular` or `plural` form based on the count.
    ///
    /// # Example
    /// ```rust
    /// let translator = Adapter::new();
    /// let result = translator.plural("apple", "apples", 2, "fruits", "shopping");
    /// assert_eq!(result, "apples");
    /// ```
    fn plural(
        &self,
        singular: &str,
        plural: &str,
        count: f64,
        domain: &str,
        context: &str,
    ) -> String {
        // translate the plural
        let index = Plural::resolve(self.get_language_data().code.as_str(), count);
        if let Some(data) = self.get(singular, domain, context) {
            if index < data.plural.len() {
                data.plural[index].clone()
            } else {
                plural.to_string()
            }
        } else {
            if index == 0 {
                singular.to_string()
            } else {
                plural.to_string()
            }
        }
    }

    /// Check if the translation data is addable
    ///
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country_language::CountryLanguageData;
    /// use crate::cores::l10n::translation::TranslationAdapter;
    /// struct MyAdapter;
    /// impl<Str: AsRef<str>> TranslationAdapter<Str> for MyAdapter {
    ///     fn new(language_code: CountryLanguageData) -> Self {
    ///         MyAdapter
    ///     }
    ///     fn addable() -> bool {
    ///         true
    ///     }
    ///     // implement other methods...
    /// }
    /// ```
    fn addable(&self) -> bool
    where
        Self: Sized,
    {
        true
    }

    /// Removes a translation entry identified by the given key, domain, and context.
    ///
    /// # Generic Parameters
    /// - `T`: A type that implements the `AsRef<str>` trait, allowing conversion to a string slice.
    ///
    /// # Parameters
    /// - `key`: The key identifying the translation entry to remove. This can represent the unique identifier or text key.
    /// - `domain`: The domain or category associated with the translation entry.
    /// - `context`: Additional context related to the translation entry, used to differentiate keys with identical text but different semantics.
    ///
    /// # Returns
    /// - `Option<TranslationData>`:
    ///   - Returns `Some(TranslationData)` containing the removed translation data if the specified key exists.
    ///   - Returns `None` if the key does not exist in the translation data.
    ///
    /// # Behavior
    /// - Combines the `key`, `domain`, and `context` into a unique composite key using the macro `translation_null_join!`.
    /// - Searches for the matching composite key in the internal storage and removes the associated translation entry.
    /// - Mutates the underlying storage by removing the specified item if present.
    ///
    /// # Example
    /// ```rust
    /// let mut translations = TranslationManager::new();
    /// translations.add("hello", "greetings", "formal", TranslationData::new("Hello"));
    /// assert!(translations.remove("hello", "greetings", "formal").is_some());
    /// assert!(translations.remove("hello", "greetings", "formal").is_none());
    /// ```
    ///
    /// # Note
    /// - The internal implementation relies on `self.get_lists_mut()` to access and modify underlying storage.
    /// - Ensure that the keys are properly joined using the provided macro to avoid mismatches.
    ///
    /// # Macro Requirement
    /// - The `translation_null_join!` macro must be defined and must handle construction of unique composite keys specific to the data storage format.
    fn remove(&mut self, id: &str, domain: &str, context: &str) -> Option<TranslationData> {
        let key = translation_key_join!(id, context, domain);
        self.get_lists_mut().remove(&key)
    }

    /// Adds a translation entry to the current collection.
    ///
    /// This method allows adding a translation with specifications including the ID, optional plural ID, domain, context,
    /// singular text, and optional plural forms. The translation is represented by a `TranslationData` object and is
    /// inserted into an internal list using a key composed of the `id` and `context`.
    ///
    /// # Type Parameters
    /// - `Str`: A type that implements the `AsRef<str>` trait, allowing flexibility in the input string types.
    ///
    /// # Arguments
    /// - `id`: A unique string identifier for the translation entry.
    /// - `plural_id`: An optional unique string identifier for the plural form of the translation entry.
    /// - `domain`: The domain to which the translation belongs, typically used for categorization.
    /// - `context`: An additional string to provide context for the translation, aiding disambiguation.
    /// - `singular`: The singular form of the translation string.
    /// - `plural`: A vector containing the plural forms of the translation string.
    ///
    /// # Returns
    /// - `true` if the translation is successfully added.
    /// - `false` if adding the translation is not allowed, determined by a call to `Self::addable()`.
    ///
    /// # Behavior
    /// - This method starts by checking if translations can be added using the `Self::addable()` method.
    /// - If `Self::addable()` returns `false`, the method immediately exits, returning `false`.
    /// - If `Self::addable()` returns `true`, it creates a key by joining the `id` and `context` values using
    ///   the `translation_null_join!` macro.
    /// - It then updates the internal list of translations with the new `TranslationData` object.
    /// - Returns `true` to indicate that the operation was successful.
    ///
    /// # Example
    /// ```rust
    /// let mut translator = Translator::new();
    /// let success = translator.add(
    ///     "greeting",
    ///     None,
    ///     "messages",
    ///     "user_interface",
    ///     "Hello",
    ///     vec!["Hello", "Hello everyone"]
    /// );
    /// assert!(success);
    /// ```
    ///
    /// In this example, a translation with the ID "greeting" and singular form "Hello" is added to a domain
    /// "messages" with additional context "user_interface". Since the operation is valid, it returns `true`.
    ///
    fn add(
        &mut self,
        id: &str,
        plural_id: Option<&str>,
        domain: &str,
        context: &str,
        singular: &str,
        plural: Vec<&str>,
    ) -> bool
    where
        Self: Sized,
    {
        if !self.addable() {
            return false;
        }

        let key_joined = translation_key_join!(id, context);
        self.get_lists_mut().insert(
            key_joined,
            TranslationData::new(id, plural_id, domain, context, singular, plural),
        );
        true
    }
}
