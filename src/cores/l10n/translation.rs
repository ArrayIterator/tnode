use crate::cores::i18n::country_language::{CountryLanguage, CountryLanguageData};
use crate::cores::l10n::plural::{Plural, ToPluralCount};
use crate::cores::l10n::translation_trait::TranslationAdapter;

#[derive(Debug)]
pub struct Translation {
    // Country language data
    language_data: CountryLanguageData,
    adapters: Vec<Box<dyn TranslationAdapter>>,
}

/// Translation Adapter Trait implementation
impl Translation
where
    Self: Sized,
{
    // Get translation data from country language data
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// use crate::cores::l10n::translation::Translation;
    /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(lang_data) = CountryLanguage::get("US") {
    ///         let translation = Translation::from_language(lang_data.clone());
    ///         println!("Language Code: {}", translation.language_data.code);
    ///     }
    /// }
    /// ```
    pub fn from_language(language_data: CountryLanguageData) -> Self {
        Self {
            language_data: language_data.clone(),
            adapters: vec![],
        }
    }

    /// Get translation data by language name (case-insensitive)
    /// # Example
    /// ```rust
    /// use crate::cores::l10n::translation::Translation;
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(translation) = Translation::from_code("en") {
    ///         println!("Language Code: {}", translation.language_data.code);
    ///     }
    /// }
    /// ```
    pub fn from_locale<T: AsRef<str>>(locale: T) -> Option<Self> {
        let results = CountryLanguage::find_by_locale(locale);
        if !results.is_empty() {
            return Some(Self::from_language((*results[0]).clone()));
        }
        None
    }

    /// Retrieves the locale code associated with the current language settings.
    ///
    /// # Returns
    ///
    /// A `String` representing the locale code. The locale code provides
    /// a standardized identifier for the language and regional settings,
    /// typically following the format of a language code and optional
    /// region code (e.g., "en-US", "fr-FR").
    ///
    /// # Examples
    ///
    /// ```rust
    /// let settings = ApplicationSettings::new();
    /// let locale = settings.get_locale();
    /// println!("Current locale: {}", locale);
    /// ```
    ///
    /// # Notes
    ///
    /// The `locale_code` function is expected to be implemented in the `language_data`
    /// object, which provides the necessary locale information.
    ///
    pub fn get_locale(&self) -> String {
        self.language_data.locale_code()
    }

    /// Retrieves the code string associated with the language data.
    ///
    /// This method accesses the internal `language_data` field of the struct
    /// and calls its `code` method to obtain the code. The resulting
    /// value is then converted to a `String` and returned to the caller.
    ///
    /// # Returns
    /// * `String` - A string representation of the code.
    ///
    /// # Example
    /// ```rust
    /// let result = instance.get_code();
    /// println!("Code: {}", result);
    /// ```
    pub fn get_code(&self) -> String {
        self.language_data.code()
    }

    /// Retrieves a reference to the `CountryLanguageData` associated with the instance.
    ///
    /// # Returns
    /// A shared reference to the `CountryLanguageData` stored in the instance.
    ///
    /// # Example
    /// ```rust
    /// let country = Country::new();
    /// let language_data = country.get_language_data();
    /// println!("{:?}", language_data);
    /// ```
    ///
    /// # Notes
    /// - This method provides a read-only reference, ensuring the underlying data cannot
    ///   be modified through the returned reference.
    /// - The `CountryLanguageData` contains relevant information about country-specific
    ///   languages.
    ///
    /// # Safety
    /// This method does not perform any unsafe operations.
    ///
    /// # See Also
    /// - [`CountryLanguageData`]: Contains structured data about a country's languages.
    pub fn get_language_data(&self) -> &CountryLanguageData {
        &self.language_data
    }
    /// Get all translation adapters
    /// # Example
    /// ```rust
    /// use crate::cores::l10n::translation::Translation;
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(lang_data) = CountryLanguage::get("US") {
    ///         let translation = Translation::from_language(lang_data.clone());
    ///         let adapters = translation.get_adapters();
    ///         println!("Adapters count: {}", adapters.len());
    ///     }
    /// }
    /// ```
    pub fn get_adapters(&self) -> &Vec<Box<dyn TranslationAdapter>> {
        &self.adapters
    }

    /// Add a translation adapter
    /// # Example
    /// ```rust
    /// use crate::cores::l10n::translation::{Translation, MemoryAdapter};
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(lang_data) = CountryLanguage::get("US") {
    ///         let mut translation = Translation::from_language(lang_data.clone());
    ///         let adapter = MemoryAdapter::new(lang_data.clone());
    ///         translation.add_adapter(adapter);
    ///         println!("Adapters count: {}", translation.get_adapters().len());
    ///     }
    /// }
    /// ```
    pub fn add_adapter(&mut self, adapter: Box<dyn TranslationAdapter>) {
        let new_id = adapter.get_type_id();
        for existing_adapter in &self.adapters {
            if existing_adapter.get_type_id() == new_id {
                return;
            }
        }
        self.adapters.push(adapter);
    }

    /// Remove a translation adapter
    /// # Example
    /// ```rust
    /// use crate::cores::l10n::translation::{Translation, MemoryAdapter};
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(lang_data) = CountryLanguage::get("US") {
    ///         let mut translation = Translation::from_language(lang_data.clone());
    ///         let adapter = MemoryAdapter::new(lang_data.clone());
    ///         translation.add_adapter(adapter);
    ///         translation.remove_adapter(adapter);
    ///         println!("Adapters count after removal: {}", translation.adapters.len());
    ///     }
    /// }
    /// ```
    pub fn remove_adapter(&mut self, adapter: Box<dyn TranslationAdapter>) {
        let target_id = adapter.get_type_id();
        self.adapters.retain(|a| a.get_type_id() != target_id);
    }

    /// Set translation adapters
    /// # Example
    /// ```rust
    /// use crate::cores::l10n::translation::{Translation, MemoryAdapter};
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(lang_data) = CountryLanguage::get("US") {
    ///         let mut translation = Translation::from_language(lang_data.clone());
    ///         let adapter = MemoryAdapter::new(lang_data.clone());
    ///         translation.set_adapters(vec![adapter]);
    ///         println!("Adapters count after set: {}", translation.adapters.len());
    ///     }
    /// }
    /// ```
    pub fn set_adapters(&mut self, adapters: Vec<Box<dyn TranslationAdapter>>) {
        self.adapters = adapters;
    }

    /// Clear all translation adapters
    /// # Example
    /// ```rust
    /// use crate::cores::l10n::translation::{Translation, MemoryAdapter};
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(lang_data) = CountryLanguage::get("US") {
    ///         let mut translation = Translation::from_language(lang_data.clone());
    ///         let adapter = MemoryAdapter::new(lang_data.clone());
    ///         translation.add_adapter(adapter);
    ///         println!("Adapters count before clear: {}", translation.adapters.len());
    ///         translation.clear_adapters();
    ///         println!("Adapters count after clear: {}", translation.adapters.len());
    ///     }
    /// }
    /// ```
    pub fn clear_adapters(&mut self) {
        self.adapters.clear();
    }

    /// Translate a singular string using available adapters
    /// # Example
    /// ```rust
    /// use crate::cores::l10n::translation::{Translation, MemoryAdapter};
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(lang_data) = CountryLanguage::get("US") {
    ///         let mut translation = Translation::from_language(lang_data.clone());
    ///         let mut adapter = MemoryAdapter::new(lang_data.clone());
    ///         adapter.add_translation("Hello", "default", "", "Hola", vec!["Holaa"]);
    ///         translation.add_adapter(adapter);
    ///         let translated = translation.translate("Hello", "default", "");
    ///         println!("Translated: {}", translated);
    ///     }
    /// }
    /// ```
    pub fn translate<K, D, C>(&self, key: K, domain: D, context: C) -> String
    where
        K: AsRef<str>,
        D: AsRef<str>,
        C: AsRef<str>,
    {
        let key = key.as_ref();
        let domain = domain.as_ref();
        let context = context.as_ref();
        for adapter in &self.adapters {
            if let Some(translated) = adapter.singular(key, domain, context) {
                return translated;
            }
        }

        // make a copy of the key and use it as the default translation
        key.to_string()
    }

    /// Translate a plural string using available adapters
    /// # Example
    /// ```rust
    /// use crate::cores::l10n::translation::{Translation, MemoryAdapter};
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(lang_data) = CountryLanguage::get("US") {
    ///         let mut translation = Translation::from_language(lang_data.clone());
    ///         let mut adapter = MemoryAdapter::new(lang_data.clone());
    ///         adapter.add_translation("apple", "default", "", "manzana", vec!["manzanas"]);
    ///         translation.add_adapter(adapter);
    ///         let translated = translation.translate_plural("apple", "apples", 3, "default", "");
    ///         println!("Translated: {}", translated);
    ///     }
    /// }
    /// ```
    pub fn translate_plural<Singular, PluralString, Count, Domain, Context>(
        &self,
        singular: Singular,
        plural: PluralString,
        count: Count,
        domain: Domain,
        context: Context,
    ) -> String
    where
        Singular: AsRef<str>,
        PluralString: AsRef<str>,
        Count: ToPluralCount + Copy,
        Domain: AsRef<str>,
        Context: AsRef<str>,
    {
        let singular_ref = singular.as_ref();
        let plural_ref = plural.as_ref();
        let domain_ref = domain.as_ref();
        let context_ref = context.as_ref();
        for adapter in &self.adapters {
            let translated = adapter.plural(
                singular_ref,
                plural_ref,
                count.to_f64(),
                domain_ref,
                context_ref,
            );
            if translated != singular_ref && translated != plural_ref {
                return translated;
            }
        }

        // Fallback to default pluralization
        let index = Plural::resolve(self.language_data.code.as_str(), count);
        if index == 0 {
            singular_ref.to_string()
        } else {
            plural_ref.to_string()
        }
    }
}
