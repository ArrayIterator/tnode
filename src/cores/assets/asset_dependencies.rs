use crate::cores::{assets::{asset_dependency::{AssetDependency, AssetType, SourceType}, attributes::Attributes}, system::error::ResultError};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

pub trait AssetDependencies : Debug + Send + Sync + 'static {
    fn get_type(&self) -> AssetType;
    fn get_dependencies(&self) -> Vec<Arc<AssetDependency>>;
    fn is_registered(&self, id: &str) -> bool;
    fn register(&mut self, asset: Arc<AssetDependency>) -> ResultError<()>;
    fn deregister(&mut self, id: &str) -> Option<Arc<AssetDependency>>;
    fn deep_copy_dependencies(&self) -> Vec<Arc<AssetDependency>> {
        self.get_dependencies().iter().map(|d| d.clone_box()).collect()
    }
    fn get(&self, id: &str) -> Option<Arc<AssetDependency>> {
        self.get_dependencies().iter().find(|d| d.get_id() == id).cloned()
    }
    fn render_for(&self, id: &str) -> Option<Vec<Arc<AssetDependency>>> {
        if let Some(asset) = self.get(id) {
            let mut rendered = HashMap::new();
            let asset = asset.clone();
            for dependency_id in asset.clone().get_inherits().into_iter() {
                if let Some(dependency) = self.get(&dependency_id) {
                    rendered.insert(dependency_id.to_string(), dependency);
                }
            }
            rendered.insert(asset.get_id(), asset.clone());
            let rendered = rendered.into_values().map(|e|e.clone()).collect();
            Some(rendered)
        } else {
            None
        }
    }
    fn register_url(&mut self, id: &str, url: &str, attributes: Option<Attributes>, inherits: Vec<String>) -> ResultError<Arc<AssetDependency>> {
        let attributes = Arc::new(attributes.unwrap_or_default());
        let asset = AssetDependency::new(
            id,
            inherits,
            attributes,
            url.to_string(),
            self.get_type(),
            SourceType::Url
        );
        let asset = Arc::new(asset);
        self.register(asset.clone())?;
        Ok(asset)
    }

    fn register_inline(&mut self, id: &str, content: &str, attributes: Option<Attributes>, inherits: Vec<String>) -> ResultError<Arc<AssetDependency>> {
        if !self.get_type().support_inline() {
            return Err(crate::cores::system::error::Error::unsupported(format!("Type {} does not support inline asset", self.get_type().as_str())))
        }
        let attributes = Arc::new(attributes.unwrap_or_default());
        let asset = AssetDependency::new(
            id,
            inherits,
            attributes,
            content.to_string(),
            self.get_type(),
            SourceType::Inline
        );
        let asset = Arc::new(asset);
        self.register(asset.clone())?;
        Ok(asset)
    }
}

macro_rules! impl_asset_deps {
    ($identity:ident, $asset_type:expr) => {
        #[derive(Debug, Clone, Default)]
        pub struct $identity {
            pub(crate) dependencies: HashMap<String, Arc<AssetDependency>>,
        }

        impl $identity {
            pub fn clone_self(&self) -> Self {
                let mut new_deps = HashMap::new();
                for (id, asset) in &self.dependencies {
                    new_deps.insert(id.clone(), asset.clone_box());
                }
                Self { dependencies: new_deps }
            }
        }

        impl AssetDependencies for $identity {
            fn is_registered(&self, id: &str) -> bool {
                self.dependencies.contains_key(id)
            }
            fn get_dependencies(&self) -> Vec<Arc<AssetDependency>> {
                self.dependencies.values().cloned().collect()
            }
            fn deregister(&mut self, id: &str) -> Option<Arc<AssetDependency>> {
                self.dependencies.remove(id)
            }
            fn get_type(&self) -> AssetType {
                $asset_type
            }
            fn register(&mut self, asset: Arc<AssetDependency>) -> ResultError<()> {
                let expected = self.get_type();
                if asset.get_asset_type() != &expected {
                    return Err(crate::cores::system::error::Error::invalid_input(format!(
                        "Asset type must be {} but {} given",
                        expected.as_str(),
                        asset.get_asset_type().as_str()))
                    );
                }
                let id = asset.get_id();
                self.dependencies.insert(id, asset);
                Ok(())
            }
        }
    };
}

impl_asset_deps!(Css, AssetType::Style);
impl_asset_deps!(Js, AssetType::Script);
