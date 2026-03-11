use crate::cores::system::error::{Error, ResultError};
use crate::cores::theme::theme::{TemplateRequirements, Theme, DEFAULT_REQUIRED_TEMPLATES};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

pub trait TemplateRequirementsRule: Debug + Send + Sync + 'static {
    fn get_required_template_requirements(&self) -> Vec<String>;
    fn satisfy_requirements(&self, template: &TemplateRequirements) -> bool {
        let (_, unsatisfied) = template
            .required_templates_satisfied_with(self.get_required_template_requirements());
        unsatisfied.is_empty()
    }
    fn satisfied(&self, template: &TemplateRequirements) -> ResultError<()> {
        if self.satisfy_requirements(template) {
            Ok(())
        } else {
            Err(Error::invalid_input("Template requirements not satisfied."))
        }
    }
}

#[derive(Debug)]
pub struct DefaultTemplateRequirementsRule {
    required_templates: Vec<String>,
}

impl DefaultTemplateRequirementsRule {
    pub fn from_template_requirements(requirements: &TemplateRequirements) -> Self {
        Self {
            required_templates: requirements.required_templates().clone(),
        }
    }

    pub fn from_theme(theme: &Theme) -> Self {
        Self::from_template_requirements(theme.requirements())
    }
}

impl TemplateRequirementsRule for DefaultTemplateRequirementsRule {
    fn get_required_template_requirements(&self) -> Vec<String> {
        self.required_templates.clone()
    }
}

impl Default for DefaultTemplateRequirementsRule {
    fn default() -> Self {
        let required_templates = DEFAULT_REQUIRED_TEMPLATES.iter().map(|s| s.to_string()).collect();
        Self {
            required_templates
        }
    }
}

#[derive(Debug)]
pub struct Themes {
    themes: HashMap<String, Arc<Theme>>,
    rule: Arc<dyn TemplateRequirementsRule>,
    default: Option<String>,
}

impl Themes {
    pub fn new() -> Self {
        Self {
            themes: HashMap::new(),
            rule: Arc::new(DefaultTemplateRequirementsRule::default()),
            default: None,
        }
    }

    pub fn set_default_theme(&mut self, theme: Theme) {
        let slug = theme.slug().to_string();
        self.default = Some(slug.clone());
        self.themes.insert(slug, Arc::new(theme));
    }
    pub fn set_default_theme_by_slug<S: AsRef<str>>(&mut self, slug: S) -> bool {
        let slug = slug.as_ref().to_string();
        if self.themes.contains_key(&slug) {
            self.default = Some(slug.clone());
            return true;
        }
        false
    }

    pub fn set_template_requirements_rule<R: TemplateRequirementsRule>(&mut self, rule: R) {
        self.rule = Arc::new(rule);
    }

    pub fn get_template_requirements_rule(&self) -> Arc<dyn TemplateRequirementsRule> {
        self.rule.clone()
    }

    pub fn from_default(default: Theme) -> Self {
        let slug = default.slug().to_string();
        let mut map = HashMap::new();
        let default = Arc::new(default);
        map.insert(slug.clone(), default.clone());
        Self {
            themes: map,
            default: Some(slug),
            rule: Arc::new(DefaultTemplateRequirementsRule::from_theme(&default)),
        }
    }
    pub fn has<T: AsRef<str>>(&self, slug: T) -> bool {
        let slug = slug.as_ref();
        self.themes.contains_key(slug)
    }

    pub fn add(&mut self, theme: Theme) -> bool {
        let slug = theme.slug().to_string();
        if let Some(_) = self.themes.insert(slug, Arc::new(theme)) {
            true
        } else {
            false
        }
    }
    pub fn get<S: AsRef<str>>(&self, slug: S) -> Option<&Arc<Theme>> {
        let slug = slug.as_ref();
        self.themes.get(slug)
    }

    pub fn list(&self) -> &HashMap<String, Arc<Theme>> {
        &self.themes
    }

    pub fn remove<S: AsRef<str>>(&mut self, slug: S) -> Option<Arc<Theme>> {
        let slug = slug.as_ref();
        if let Some(def) = &self.default
            && def == slug
        {
            return None;
        }
        self.themes.remove(slug)
    }
    pub fn default(&self) -> Option<&Arc<Theme>> {
        self.default.as_ref().and_then(|s| self.themes.get(s))
    }
    pub fn render<S: Serialize, Slug: AsRef<str>>(
        &self,
        slug: Slug,
        template: &str,
        data: &S,
    ) -> ResultError<String> {
        let slug = slug.as_ref();
        if let Some(theme) = self.get(slug) {
            return theme.render(template, data);
        }
        Err(Error::invalid_input(format!("Theme {} not found.", slug)))
    }
}

impl Default for Themes {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for Themes {
    type Item = (String, Arc<Theme>);
    type IntoIter = std::collections::hash_map::IntoIter<String, Arc<Theme>>;
    fn into_iter(self) -> Self::IntoIter {
        self.themes.into_iter()
    }
}
