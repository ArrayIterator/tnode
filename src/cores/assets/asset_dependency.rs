use crate::cores::assets::attributes::Attributes;
use std::{fmt::{Debug, Display}, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetType {
    Style,
    Script,
}

impl AssetType {
    pub fn as_name(&self) -> &'static str {
        match self {
            Self::Style => "Style",
            Self::Script => "Script",
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Style => "style",
            Self::Script => "script",
        }
    }
    pub fn tag_name_inline(&self) -> &'static str {
        self.as_str()
    }
    pub fn tag_name(&self) -> &'static str {
        match self {
            Self::Style => "link",
            Self::Script => "script",
        }
    }
    pub fn is_self_closing(&self) -> bool {
        match self {
            Self::Style => true,
            Self::Script => false,
        }
    }
    pub fn support_inline(&self) -> bool {
        match self {
            AssetType::Style => true,
            AssetType::Script => true,
        }
    }
    pub fn configure_attribute(&self, content: &str, source_type: &SourceType, attributes: &Attributes) -> Attributes {
        let mut attributes = attributes.clone();
        if source_type.is_inline() {
            match self {
                AssetType::Style => {
                    attributes.remove("rel");
                    attributes.remove("href");
                },
                AssetType::Script => {
                    attributes.remove("src");
                },
            }
        } else {
            match self {
                AssetType::Style => {
                    attributes.attr("rel", "stylesheet").attr("href", &content);
                },
                AssetType::Script => {
                    attributes.attr("src", &content);
                }
            }
        }
        attributes
    }

    pub fn render(&self, content: &str, source_type: &SourceType, attributes: &Attributes) -> String {
        if source_type.is_inline() && !self.support_inline() {
            return "".to_string();
        }
        let mut attributes = self.configure_attribute(content, source_type, attributes);
        let attributes_str = if attributes.to_attributes_string().is_empty() {
            "".to_string()
        } else {
            format!(" {}", attributes.to_attributes_string())
        };
        if source_type.is_inline() {
            format!(
                "<{tag}{attributes}>{content}</{tag}>",
                tag = self.tag_name_inline(),
                attributes = attributes_str,
                content = content
            )
        } else {
            if self.is_self_closing() {
                format!(
                    "<{tag}{attributes}>",
                    tag = self.tag_name(),
                    attributes = attributes_str
                )
            } else {
                format!(
                    "<{tag}{attributes}></{tag}>",
                    tag = self.tag_name(),
                    attributes = attributes_str
                )
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceType {
    Url,
    Inline,
}
impl SourceType {
    pub fn is_inline(&self) -> bool {
        match self {
            SourceType::Url => false,
            SourceType::Inline => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssetDependency {
    pub id: String,
    pub inherits: Vec<String>,
    pub attributes: Arc<Attributes>,
    pub source: String,
    pub asset_type: AssetType,
    pub source_type: SourceType,
}

impl AssetDependency {
    pub fn new(
        id: &str,
        inherits: Vec<String>,
        attributes: Arc<Attributes>,
        source: String,
        asset_type: AssetType,
        source_type: SourceType,
    ) -> Self {
        Self {
            id: id.to_string(),
            inherits,
            attributes,
            source,
            asset_type,
            source_type,
        }
    }
    pub fn get_id(&self) -> String {
        self.id.clone()
    }
    pub fn get_inherits(&self) -> Vec<String> {
        self.inherits.clone()
    }
    pub fn get_attributes(&self) -> Arc<Attributes> {
        self.attributes.clone()
    }
    pub fn get_source(&self) -> String {
        self.source.clone()
    }
    pub fn get_asset_type(&self) -> &AssetType {
        &self.asset_type
    }
    pub fn get_source_type(&self) -> &SourceType {
        &self.source_type
    }
    pub fn render(&self) -> String {
        self.asset_type.render(&self.source, &self.source_type, &self.attributes)
    }
    pub fn clone_box(&self) -> Arc<AssetDependency> {
        Arc::new(self.clone())
    }
}

impl Display for AssetDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
