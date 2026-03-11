use std::collections::HashMap;

use handlebars::html_escape;


#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

pub const INVALID_ATTRIBUTE_KEY_CHARS: &[char] = &[' ', '"', '\'', '=', '<', '>', '/'];
pub const LOWERCASE_ATTRIBUTE_LIST: &[&str] = &[
    // --- Global Attributes ---
    "id", "class", "style", "title", "lang", "dir", "tabindex", "accesskey",
    "hidden", "draggable", "contenteditable", "spellcheck", "translate",
    "role", "data-", "aria-", "contextmenu", "slot",

    // --- Links & Navigation ---
    "href", "target", "rel", "download", "hreflang", "type", "referrerpolicy",
    "ping", "media",

    // --- Images & Multimedia ---
    "src", "srcset", "sizes", "alt", "width", "height", "loading", "decoding",
    "crossorigin", "usemap", "ismap", "poster", "preload", "autoplay",
    "loop", "muted", "controls", "playsinline", "default", "kind", "srclang",

    // --- Forms & Inputs ---
    "name", "value", "type", "placeholder", "disabled", "readonly", "required",
    "checked", "selected", "multiple", "autofocus", "autocomplete", "form",
    "formaction", "formenctype", "formmethod", "formnovalidate", "formtarget",
    "min", "max", "step", "maxlength", "minlength", "pattern", "size", "list",
    "accept", "capture", "dirname", "rows", "cols", "wrap",

    // --- Table Attributes ---
    "span", "colspan", "rowspan", "headers", "scope",

    // --- Metadata & Scripting ---
    "charset", "content", "http-equiv", "name", "property", "async", "defer",
    "nomodule", "integrity", "nonce", "sandbox", "allow", "allowfullscreen",

    // --- Performance & Optimization ---
    "as", "imagesrcset", "imagesizes", "fetchpriority",

    // --- Others (SVG/Deprecated/Legacy but still active) ---
    "align", "valign", "border", "bgcolor", "cellspacing", "cellpadding"
];

impl Attribute {
    pub fn new<N: Into<String>, V: Into<String>>(name: N, value: V) -> Self {
        let name_str = name.into().trim().to_string();
        let mut clean_name = String::new();

        Self {
            name: Self::normalize_attribute_key(name_str),
            value: value.into(),
        }
    }
    pub fn normalize_attribute_key<T: AsRef<str>>(key: T) -> String {
        let key_str = key.as_ref().trim().to_string();
        let mut clean_key = String::new();

        for c in key_str.chars() {
            if !INVALID_ATTRIBUTE_KEY_CHARS.contains(&c) {
                clean_key.push(c);
            }
        }

        let lower_key = clean_key.to_lowercase();
        if lower_key.starts_with("data-") {
            format!("data-{}", &clean_key[5..])
        } else if lower_key.starts_with("aria-") {
            format!("aria-{}", &clean_key[5..])
        } else if LOWERCASE_ATTRIBUTE_LIST.contains(&lower_key.as_str()) {
            lower_key
        } else {
            clean_key
        }
    }

    pub fn to_attribute_string(&self) -> Option<String> {
        if self.name.is_empty() {
            return None;
        }
        if self.value.is_empty() {
            // Boolean attribute (e.g., disabled, required)
            return Some(self.name.clone());
        }
        let escaped_value = html_escape(&self.value);
        let escaped_key = html_escape(&self.name);
        Some(format!("{}=\"{}\"", escaped_key, escaped_value))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Attributes {
    pub attributes: HashMap<String, Attribute>,
}

impl Attributes {
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }
    pub fn attr<N: AsRef<str>, V: AsRef<str>>(&mut self, name: N, value: V) -> &mut Self {
        let attribute = Attribute::new(name.as_ref(), value.as_ref());
        self.add(attribute)
    }
    pub fn add(&mut self, attribute: Attribute) -> &mut Self {
        self.attributes.insert(attribute.name.clone(), attribute);
        self
    }
    pub fn get(&self, name: &str) -> Option<&Attribute> {
        self.attributes.get(name)
    }
    pub fn remove(&mut self, name: &str) -> Option<Attribute> {
        self.attributes.remove(name)
    }
    pub fn all(&self) -> Vec<&Attribute> {
        self.attributes.values().collect()
    }
    pub fn clear(&mut self) {
        self.attributes.clear();
    }
    pub fn has<T: AsRef<str>>(&self, name: T) -> bool {
        self.attributes.contains_key(name.as_ref())
    }
    pub fn to_attributes_string(&self) -> String {
        if self.attributes.is_empty() {
            return String::new();
        }
        self.attributes
            .values()
            .filter_map(|attr| attr.to_attribute_string())
            .collect::<Vec<_>>()
            .join(" ")
    }
    pub fn to_attributes_string_exclude(&self, exclusion: Vec<String>) -> String {
        if self.attributes.is_empty() {
            return String::new();
        }
        let exlusion_lists = exclusion.iter().map(|s| Attribute::normalize_attribute_key(s)).collect::<Vec<_>>();
        self.attributes
            .values()
            .filter(|attr| !exlusion_lists.contains(&attr.name))
            .filter_map(|attr| attr.to_attribute_string())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl IntoIterator for Attributes {
    type Item = Attribute;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.attributes.into_values().collect::<Vec<_>>().into_iter()
    }
}
