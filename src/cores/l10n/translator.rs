use crate::cores::i18n::country_language::CountryLanguage;
use crate::cores::l10n::plural::{Plural, ToPluralCount};
use crate::cores::l10n::translation::Translation;
use std::collections::HashMap;
use std::string::ToString;

#[derive(Debug)]
pub struct Translator {
    translations: HashMap<String, Box<Translation>>,
}

pub const NO_TRANSLATE: &str = "en";

impl Default for Translator {
    fn default() -> Self {
        Self::new()
    }
}

impl Translator {
    pub fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            translations: HashMap::new(),
        }
    }

    pub fn set_translation(&mut self, translation: Translation) {
        self.add_translation(Box::new(translation));
    }

    pub fn add_translation(&mut self, translation: Box<Translation>) {
        let code = translation.get_code();
        if code.is_empty() || NO_TRANSLATE.eq(code.as_str()) {
            return;
        }
        self.translations
            .insert(code, translation);
    }

    pub fn remove_translation<R: AsRef<str>>(&mut self, code: R) -> Option<Box<Translation>> {
        self.translations.remove(&code.as_ref().to_string())
    }

    pub fn translate<Locale, Key, Domain, Context>(
        &self,
        locale: Locale,
        key: Key,
        domain: Domain,
        context: Context,
    ) -> String
    where
        Locale: AsRef<str>,
        Key: AsRef<str>,
        Domain: AsRef<str>,
        Context: AsRef<str>,
    {
        let locale = CountryLanguage::normalize_code_2(locale.as_ref());
        if locale.len() == 2 && !NO_TRANSLATE.eq(locale.as_str()) {
            let translation = self.translations.get(&locale);
            if let Some(translation) = translation {
                return translation.translate(key, domain, context);
            }
        }
        key.as_ref().to_string()
    }

    pub fn translate_plural<Locale, Singular, PluralString, Count, Domain, Context>(
        &self,
        locale: Locale,
        singular: Singular,
        plural: PluralString,
        count: Count,
        domain: Domain,
        context: Context,
    ) -> String
    where
        Locale: AsRef<str>,
        Singular: AsRef<str>,
        PluralString: AsRef<str>,
        Count: ToPluralCount + Copy,
        Domain: AsRef<str>,
        Context: AsRef<str>,
    {
        let locale = CountryLanguage::normalize_code_2(locale);
        if locale.len() == 2 && !NO_TRANSLATE.eq(locale.as_str()) {
            let translation = self.translations.get(&locale);
            if let Some(translation) = translation {
                return translation.translate_plural(singular, plural, count, domain, context);
            }
        }
        let index = if let Some(locale) = CountryLanguage::get(&locale) {
            Plural::resolve(&locale.code, count)
        } else {
            if count.to_f64() == 1.0 {
                0
            } else {
                1
            }
        };
        if index == 0 {
            singular.as_ref().to_string()
        } else {
            plural.as_ref().to_string()
        }
    }
}
