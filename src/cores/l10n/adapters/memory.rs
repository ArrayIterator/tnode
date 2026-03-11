use crate::cores::i18n::country_language::CountryLanguageData;
use crate::cores::l10n::translation_trait::{TranslationAdapter, TranslationData};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MemoryAdapter {
    translation_lists: HashMap<String, TranslationData>,
    language_data: CountryLanguageData,
}

impl TranslationAdapter for MemoryAdapter {
    fn new(language_data: CountryLanguageData) -> Self {
        Self {
            translation_lists: HashMap::new(),
            language_data: language_data.clone(),
        }
    }

    fn get_translation_lists(&self) -> HashMap<String, TranslationData> {
        self.translation_lists.clone()
    }

    fn get_lists_mut(&mut self) -> &mut HashMap<String, TranslationData> {
        &mut self.translation_lists
    }

    fn get_language_data(&self) -> &CountryLanguageData {
        &self.language_data
    }
}
