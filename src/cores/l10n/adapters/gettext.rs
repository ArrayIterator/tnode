use crate::cores::helper::file_info::FileInfo;
use crate::cores::i18n::country_language::CountryLanguageData;
use crate::cores::l10n::translation_trait::{TranslationAdapter, TranslationData};
use crate::cores::system::error::{Error, ResultError};
use crate::translation_key_join;
use polib::catalog::Catalog;
use polib::po_file::{parse_from_reader_with_option, POParseOptions};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Gettext {
    language: CountryLanguageData,
    translation_lists: HashMap<String, TranslationData>,
}

impl Gettext {
    pub fn new(language: CountryLanguageData) -> Self {
        Self {
            language,
            translation_lists: HashMap::new(),
        }
    }

    /// Parses a catalog from the given file path.
    ///
    /// This function validates the provided file path by ensuring the file:
    /// - Exists
    /// - Is a regular file (not a directory or other type of file)
    /// - Is readable
    ///
    /// If any of these conditions are not met, an appropriate `Error` is returned.
    /// Upon successful validation, the function attempts to open the file and parse
    /// it into a `Catalog` object.
    ///
    /// # Arguments
    ///
    /// * `path` - A `PathBuf` representing the path to the file that contains the catalog.
    ///
    /// # Returns
    ///
    /// * `Ok(Catalog)` - If the file is successfully validated, opened, and parsed
    ///   into a `Catalog`.
    /// * `Err(Error)` - If any of the following conditions occur:
    ///     - The file does not exist.
    ///     - The file is not a regular file.
    ///     - The file is not readable.
    ///     - An error occurs while opening or parsing the file.
    ///
    /// # Errors
    ///
    /// - Returns `ErrorKind::NotFound` if the file does not exist.
    /// - Returns `ErrorKind::InvalidInput` if the file is not a regular file or is not readable.
    /// - Returns `ErrorKind::Other` if an error occurs during file opening or parsing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::Path;
    /// use my_crate::{Catalog, parse_catalog};
    ///
    /// let path = Path::new("example_catalog.json");
    /// match parse_catalog(path) {
    ///     Ok(catalog) => {
    ///         println!("Catalog parsed successfully: {:?}", catalog);
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to parse catalog: {}", e);
    ///     }
    /// }
    /// ```
    pub fn parse_catalog(path: &Path) -> ResultError<Catalog> {
        let path: FileInfo = FileInfo::new(path);
        if !path.is_exists() {
            return Err(Error::file_not_found(format!(
                "File {} does not exist",
                path
            )));
        }
        if !path.is_file() {
            return Err(Error::invalid_input(format!("File {} is not a file", path)));
        }
        if !path.is_readable() {
            return Err(Error::invalid_input(format!("File {} is not a file", path)));
        }
        let binding = path
            .extension()
            .unwrap_or_else(|| "".to_string())
            .to_lowercase();
        let file_type = binding.as_str();
        let mut f = File::open(path.as_path()).map_err(Error::from)?;
        let parse_options: POParseOptions;
        if file_type != "po" && file_type != "mo" {
            let mut buffer = [0u8; 4];
            if f.read_exact(&mut buffer).is_ok() {
                parse_options = Gettext::get_options_from_bytes(&buffer);
            } else {
                parse_options = POParseOptions::default();
            }
            f.seek(SeekFrom::Start(0)).ok();
        } else {
            parse_options = if file_type == "mo" {
                POParseOptions {
                    message_body_only: true,
                    translated_only: true,
                    unsafe_utf8_decode: false,
                }
            } else {
                POParseOptions::default()
            };
        }
        parse_from_reader_with_option(f, &parse_options).map_err(|e| Error::parse_error(e))
    }

    pub fn parse_catalog_from_bytes(bytes: &[u8]) -> ResultError<Catalog> {
        let parse_options = Self::get_options_from_bytes(bytes);
        let reader = Cursor::new(bytes);
        parse_from_reader_with_option(reader, &parse_options).map_err(|e| Error::parse_error(e))
    }

    pub fn parse_catalog_from_string<T: AsRef<str>>(content: T) -> ResultError<Catalog> {
        Self::parse_catalog_from_bytes(content.as_ref().as_bytes())
    }

    fn get_options_from_bytes(bytes: &[u8]) -> POParseOptions {
        let mut is_mo = false;
        if bytes.len() >= 4 {
            let buffer: [u8; 4] = bytes[0..4].try_into().unwrap_or([0u8; 4]);

            let magic_le = u32::from_le_bytes(buffer);
            let magic_be = u32::from_be_bytes(buffer);

            if magic_le == 0x950412de || magic_be == 0x950412de {
                is_mo = true;
            }
        }
        if is_mo {
            POParseOptions {
                message_body_only: true,
                translated_only: true,
                unsafe_utf8_decode: false,
            }
        } else {
            POParseOptions::default()
        }
    }

    pub fn add_from_catalog<T: AsRef<str>>(
        &mut self,
        catalog: Catalog,
        domain: T,
    ) -> ResultError<()> {
        let domain_str = domain.as_ref();
        let mut staging = HashMap::new();
        for message in catalog.messages() {
            let singular = message
                .msgstr()
                .map_err(|e| Error::parse_error(format!("Error parsing translation: {}", e)))?
                .to_string();
            if singular.is_empty() {
                continue;
            }
            let id = message.msgid();
            let context = message.msgctxt().unwrap_or("");
            let mut plural_id: Option<String> = None;
            let mut plural_translations: Vec<String> = vec![];
            if let Ok(msgid_plural) = message.msgid_plural() {
                plural_id = Some(msgid_plural.to_string());
                plural_translations = message
                    .msgstr_plural()
                    .map_err(|e| Error::parse_error(format!("Error parsing translation: {}", e)))?
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            }
            let key = translation_key_join!(id, context);
            staging.insert(
                key,
                TranslationData::new(
                    id,
                    plural_id,
                    domain_str,
                    context,
                    singular,
                    plural_translations,
                ),
            );
        }
        self.translation_lists.extend(staging);
        Ok(())
    }
}

impl TranslationAdapter for Gettext {
    fn new(language_data: CountryLanguageData) -> Self {
        Self::new(language_data)
    }

    fn get_translation_lists(&self) -> HashMap<String, TranslationData> {
        self.translation_lists.clone()
    }

    fn get_lists_mut(&mut self) -> &mut HashMap<String, TranslationData> {
        &mut self.translation_lists
    }

    fn get_language_data(&self) -> &CountryLanguageData {
        &self.language
    }
}
