use crate::cores::{assets::{asset_dependencies::{AssetDependencies, Css, Js}, asset_dependency::AssetDependency, asset_manager::AssetManager, attributes::Attributes}, helper::hack::Hack, system::error::ResultError};
use std::{collections::HashMap, sync::Arc};


#[derive(Debug, Clone)]
pub struct Package {
    id: String,
    css: Vec<String>,
    js: Vec<String>,
}

impl Package {
    pub fn new(id: &str, css: Vec<String>, js: Vec<String>) -> Self {
        Self {
            id: id.to_string(),
            css: Hack::unique_set_string(css),
            js: Hack::unique_set_string(js),
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn css(&self) -> &Vec<String> {
        &self.css
    }
    pub fn js(&self) -> &Vec<String> {
        &self.js
    }
    pub fn clone_self(&self) -> Self {
        Self {
            id: self.id.clone(),
            css: self.css.clone(),
            js: self.js.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Assets {
    pub(crate) js: Js,
    pub(crate) css: Css,
    pub(crate) packages: HashMap<String, Arc<Package>>,
}

impl Assets {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get_js(&mut self) -> &Js {
        &self.js
    }
    pub fn get_js_mut(&mut self) -> &mut Js {
        &mut self.js
    }
    pub fn get_css(&mut self) -> &Css {
        &self.css
    }
    pub fn get_css_mut(&mut self) ->&mut Css {
        &mut self.css
    }
    pub fn register_css(
        &mut self,
        id: &str,
        url: &str,
        attributes: Option<Attributes>,
        inherits: Vec<String>
    ) -> ResultError<Arc<AssetDependency>> {
        self.get_css_mut().register_url(id, url, attributes, inherits)
    }
    pub fn register_css_inline(
        &mut self,
        id: &str,
        content: &str,
        attributes: Option<Attributes>,
        inherits: Vec<String>
    ) -> ResultError<Arc<AssetDependency>> {
        self.get_css_mut().register_inline(id, content, attributes, inherits)
    }
    pub fn deregister_css(&mut self, id: &str) -> Option<Arc<AssetDependency>> {
        self.get_css_mut().deregister(id)
    }
    pub fn register_js(
        &mut self,
        id: &str,
        url: &str,
        attributes: Option<Attributes>,
        inherits: Vec<String>
    ) -> ResultError<Arc<AssetDependency>> {
        self.get_js_mut().register_url(id, url, attributes, inherits)
    }
    pub fn register_js_inline(
        &mut self,
        id: &str,
        content: &str,
        attributes: Option<Attributes>,
        inherits: Vec<String>
    ) -> ResultError<Arc<AssetDependency>> {
        self.get_js_mut().register_inline(id, content, attributes, inherits)
    }
    pub fn deregister_js(&mut self, id: &str) -> Option<Arc<AssetDependency>> {
        self.get_js_mut().deregister(id)
    }
    pub fn get_packages(&self) -> &HashMap<String, Arc<Package>> {
        &self.packages
    }
    pub fn get_package(&mut self, package_id: &str) -> Option<Arc<Package>> {
        self.packages.get(package_id).map(|e|e.clone())
    }
    pub fn register_package(&mut self, package: Package) -> Option<Arc<Package>> {
        self.packages.insert(package.id.clone(), Arc::new(package))
    }
    pub fn deregister_package(&mut self, package_id: &str) -> Option<Arc<Package>> {
        self.packages.remove(package_id)
    }
    pub fn new_manager(&self) -> AssetManager {
        AssetManager::create_manager(self)
    }

    /// Doing deep clone of Assets (package keep the Arc cause reuse)
    pub fn clone_self(&self) -> Self {
        Self {
            js: self.js.clone_self(),
            css: self.css.clone_self(),
            packages: self.packages.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        }
    }
}
