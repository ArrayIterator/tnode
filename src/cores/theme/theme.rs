use crate::cores::helper::hack::Hack;
use crate::cores::libs::handlebar::Handlebar;
use crate::cores::system::error::{Error, ResultError};
use crate::cores::theme::theme_uri::ThemeUri;
use handlebars::{
    Context, Handlebars, Helper, Output, RenderContext, RenderError, RenderErrorReason,
};
use rust_embed::EmbeddedFile;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const DEFAULT_REQUIRED_TEMPLATES: &[&str] = &["layout", "error", "404"];

const MAX_TEMPLATE_SIZE: u64 = 1024 * 1024 * 5;
const DEFAULT_TEMPLATE_SIZE: u64 = 1024 * 512;
const DEFAULT_LANGUAGE_SIZE: u64 = 1024 * 1024;
const MIN_TEMPLATE_SIZE: u64 = 1024;

fn handle_bar_register(handlebar: &mut Handlebar, uri: &ThemeUri) {
    let uri_internal = uri.clone();
    handlebar.register_helper(
        "asset_url",
        Box::new(
            move |h: &Helper,
                  _: &Handlebars,
                  _: &Context,
                  _: &mut RenderContext,
                  out: &mut dyn Output|
                  -> Result<(), RenderError> {
                let path = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
                out.write(&uri_internal.assets_url(path)).map_err(|e| {
                    RenderErrorReason::Other(format!("Failed to write asset url: {}", e))
                })?;
                Ok(())
            },
        ),
    );
}

fn normalize_directories(directories: &Directories) -> Directories {
    let mut normalized = directories.clone();
    let default_directories = Directories::default();
    if normalized.assets.trim().is_empty() {
        normalized.assets = default_directories.assets.clone();
    }
    if normalized.partials.trim().is_empty() {
        normalized.partials = default_directories.partials.clone();
    }
    if normalized.language.trim().is_empty() {
        normalized.language = default_directories.language.clone();
    }
    normalized.assets = format!(
        "/{}",
        directories.assets.trim_matches(|c| c == '/' || c == '\\')
    );
    normalized.partials = format!(
        "/{}",
        directories.partials.trim_matches(|c| c == '/' || c == '\\')
    );
    normalized.language = format!(
        "/{}",
        directories.language.trim_matches(|c| c == '/' || c == '\\')
    );
    normalized
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeConfig {
    pub name: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub screenshot: Option<Screenshot>,
    pub version: String,
    pub author: Author,
    pub directories: Directories,
    pub templates: Option<Templates>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Screenshot {
    Url(String),
    Path(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Author {
    pub name: String,
    pub email: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Directories {
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub assets: String,
    #[serde(default)]
    pub partials: String,
}

impl Default for Directories {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            assets: "assets".to_string(),
            partials: "partials".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Templates {
    #[serde(default)]
    pub layout: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub not_found: String,
    #[serde(flatten)]
    __flatten: HashMap<String, String>,
}

impl Default for Templates {
    fn default() -> Self {
        Self {
            layout: "layout".to_string(),
            error: "error".to_string(),
            not_found: "404".to_string(),
            __flatten: HashMap::new(),
        }
    }
}

impl IntoIterator for Templates {
    type Item = (String, String);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let mut items: Vec<(String, String)> = [
            ("layout", self.layout),
            ("error", self.error),
            ("not_found", self.not_found),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        items.extend(self.__flatten.into_iter().map(|(k, v)| (k, v)));

        items.into_iter()
    }
}

impl Templates {
    pub fn get<T: AsRef<str>>(&self, name: T) -> Option<&str> {
        let name = name.as_ref();
        match name {
            "layout" => Some(&self.layout),
            "error" => Some(&self.error),
            "not_found" => Some(&self.not_found),
            other => self.__flatten.get(other).map(|s| s.as_str()),
        }
    }

    pub fn all(&self) -> HashMap<String, String> {
        HashMap::from_iter(self.clone().into_iter())
    }

    pub fn clean_extensions(&mut self) {
        fn trim_hbs(s: &mut String) {
            if s.ends_with(".hbs") {
                s.truncate(s.len() - 4);
            }
        }
        trim_hbs(&mut self.layout);
        trim_hbs(&mut self.error);
        trim_hbs(&mut self.not_found);
        for value in self.__flatten.values_mut() {
            if value.ends_with(".hbs") {
                value.truncate(value.len() - 4);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum RequirementType {
    FileSystem,
    Embed,
}

#[derive(Clone)]
pub struct EmbedFile(EmbeddedFile);

impl Debug for EmbedFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddedFile")
            .field("size", &self.0.data.len())
            .field("hash", &self.0.metadata.sha256_hash())
            .finish()
    }
}

impl From<EmbeddedFile> for EmbedFile {
    fn from(file: EmbeddedFile) -> Self {
        Self(file)
    }
}
impl From<&EmbedFile> for EmbeddedFile {
    fn from(file: &EmbedFile) -> Self {
        file.0.clone()
    }
}

#[derive(Debug, Clone)]
pub struct EmbedCollector {
    files: HashMap<Cow<'static, str>, EmbedFile>,
}

impl EmbedCollector {
    pub fn new<E: rust_embed::RustEmbed + 'static>() -> Self {
        let mut files = HashMap::new();
        for path in E::iter() {
            if let Some(file) = E::get(&path) {
                files.insert(path, EmbedFile::from(file));
            }
        }
        Self { files }
    }
    pub fn get(&self, name: &str) -> Option<&EmbedFile> {
        self.files.get(name)
    }
    pub fn exists(&self, name: &str) -> bool {
        self.files.contains_key(name)
    }
}

impl Default for EmbedCollector {
    fn default() -> Self {
        Self {
            files: HashMap::new(),
        }
    }
}

impl<'a> IntoIterator for &'a EmbedCollector {
    type Item = (&'a Cow<'static, str>, &'a EmbedFile);
    type IntoIter = std::collections::hash_map::Iter<'a, Cow<'static, str>, EmbedFile>;

    fn into_iter(self) -> Self::IntoIter {
        self.files.iter()
    }
}

#[derive(Debug, Clone)]
pub struct TemplateRequirements {
    requirement_type: RequirementType,
    slug: String,
    theme_dir: Option<PathBuf>,
    required: Vec<String>,
    asset_path: String,
    partial_path: String,
    language_path: String,
    // key as path name, and value as full-path
    partial_file_map: HashMap<String, String>,
    // key as path name, and value as full-path
    template_file_map: HashMap<String, String>,
    // key as path name, and value as full-path
    asset_file_map: HashMap<String, String>,
    // key as lang code name, and value as full-path
    language_file_map: HashMap<String, String>,
    embedded_files: Option<EmbedCollector>,
    templates: Templates,
}

impl TemplateRequirements {
    fn reconfigure_init(
        required: &Option<Vec<String>>,
        config_base_name: Option<&str>,
    ) -> ResultError<(ThemeConfig, Templates, Vec<String>, String, String, String)> {
        let config_file = match config_base_name {
            None => {
                let mut current_path = None;
                for path in ["theme.yaml", "theme.yml"] {
                    let path = Path::new(path);
                    if !path.exists() {
                        continue;
                    }
                    current_path = Some(path.to_path_buf());
                    break;
                }
                current_path
            }
            Some(e) => Path::new(e).to_path_buf().into(),
        };
        if let None = config_file {
            return Err(Error::file_not_found(
                "Theme configuration file not found.".to_string(),
            ));
        }
        let config_file = config_file.unwrap();
        let extension = config_file
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or("");
        if extension != "yaml" && extension != "yml" {
            return Err(Error::invalid_input(format!(
                "Extension of theme configuration file must be yaml or yml, but got {}.",
                extension
            )));
        }

        let mut config =
            serde_yaml::from_str::<ThemeConfig>(&fs::read_to_string(&config_file).map_err(
                |e| Error::parse_error(format!("Error parsing metadata theme file: {}", e)),
            )?)
            .map_err(|e| Error::parse_error(format!("Error parsing metadata theme file: {}", e)))?;
        let mut theme_cfg = config.clone();
        let mut directories = theme_cfg.directories;
        let mut name = theme_cfg.name.trim().to_string();
        directories = normalize_directories(&directories);
        // do validation
        let mut templates = theme_cfg.templates.unwrap_or_default();
        templates.clean_extensions();
        let required = required.clone().unwrap_or_else(|| {
            DEFAULT_REQUIRED_TEMPLATES
                .iter()
                .map(|s| s.to_string())
                .collect()
        });
        let asset_path = directories.assets.clone();
        let partial_path = directories.partials.clone();
        let language_path = directories.language.clone();
        let asset_path = Hack::clean_unique_path_trim(asset_path).to_string();
        let partial_path = Hack::clean_unique_path_trim(partial_path).to_string();
        let language_path = Hack::clean_unique_path_trim(language_path).to_string();
        directories.assets = asset_path.clone();
        directories.partials = partial_path.clone();
        directories.language = language_path.clone();
        config.directories = directories;
        config.templates = Some(templates.clone());
        Ok((
            config,
            templates,
            required,
            asset_path,
            partial_path,
            language_path,
        ))
    }

    fn read_files(
        root: &Path,
        dir: &Path,
        nested: bool,
        extensions: &Vec<&str>,
        max_size: u64,
    ) -> ResultError<HashMap<String, String>> {
        let has_extension = extensions.len() > 0;
        let mut files: HashMap<String, String> = HashMap::new();
        for entry in fs::read_dir(dir).map_err(|e| Error::from_error(e))? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if nested {
                    let nested_files = Self::read_files(root, &path, nested, extensions, max_size)?;
                    for (name, file) in nested_files {
                        files.insert(name, file);
                    }
                }
                continue;
            }
            // discard the file that starts with dot
            if path.starts_with(".") {
                continue;
            }
            let meta = path.metadata().map_err(|e| Error::from_io_error(e))?;
            if !meta.is_file() {
                continue;
            }
            let size = meta.len();
            if size > max_size {
                return Err(Error::overflow(format!(
                    "Template file '{}' is too large try maximum is: {} and current file size is: {}",
                    path.display(),
                    max_size,
                    size
                )));
            }
            let relative_path = path
                .strip_prefix(root)
                .map_err(|e| Error::invalid_input(e))?;
            let path_str = path.to_string_lossy().replace("\\", "/");
            let clean_name = relative_path.to_string_lossy().replace("\\", "/");
            if has_extension {
                if let Some(extension) = path.extension()
                    && let Some(ext) = extension.to_str()
                {
                    if !extensions.contains(&ext) {
                        continue;
                    }
                    let clean_name = clean_name
                        .strip_suffix(&ext)
                        .unwrap_or(&clean_name)
                        .to_string();
                    files.insert(clean_name, path_str);
                }
                continue;
            }
            files.insert(clean_name, path_str);
        }
        Ok(files)
    }

    fn validate_paths(
        theme_dir: &Path,
        templates: &Templates,
        required_files: Vec<String>,
    ) -> ResultError<()> {
        for name in &required_files {
            if let Some(template_name) = templates.get(name) {
                let file_path = theme_dir.join(format!("{}.hbs", template_name));
                if !file_path.exists() {
                    return Err(Error::file_not_found(format!(
                        "Template file for {} ({}.hbs) not found in {:?}",
                        name, template_name, theme_dir
                    )));
                }
            } else {
                return Err(Error::invalid_input(format!(
                    "Template name '{}' is required but not found in configuration.",
                    name
                )));
            }
        }
        Ok(())
    }

    //noinspection DuplicatedCode
    pub fn from_file(
        theme_dir: &Path,
        required: Option<Vec<String>>,
        config_base_name: Option<&str>,
        max_size: Option<u64>,
    ) -> ResultError<Self> {
        let slug = match theme_dir.file_name() {
            Some(e) => e
                .to_str()
                .ok_or_else(|| Error::invalid_input("Invalid theme directory name.".to_string()))?,
            None => {
                return Err(Error::invalid_input(
                    "Invalid theme directory name.".to_string(),
                ));
            }
        };
        if !theme_dir.exists() {
            return Err(Error::file_not_found(format!(
                "Theme directory '{}' does not exist.",
                theme_dir.display()
            )));
        }
        if !theme_dir.is_dir() {
            return Err(Error::not_a_directory(format!(
                "Theme directory '{}' is not a directory.",
                theme_dir.display()
            )));
        }
        let mut max_size = max_size.unwrap_or(DEFAULT_TEMPLATE_SIZE);
        if max_size > MAX_TEMPLATE_SIZE {
            max_size = MAX_TEMPLATE_SIZE;
        } else if max_size < MIN_TEMPLATE_SIZE {
            max_size = MIN_TEMPLATE_SIZE;
        }
        let (config, templates, required, asset_path, partial_path, language_path) =
            Self::reconfigure_init(&required, config_base_name)?;
        let partial_dir = theme_dir.join(partial_path.clone());
        let asset_dir = theme_dir.join(asset_path.clone());
        let language_dir = theme_dir.join(language_path.clone());
        let partial_file_map =
            Self::read_files(&partial_dir, &partial_dir, true, &vec!["hbs"], max_size)
                .unwrap_or_default();
        let mut template_file_map = Self::read_files(
            &theme_dir.as_ref(),
            &theme_dir.as_ref(),
            false,
            &vec!["hbs"],
            max_size,
        )
        .unwrap_or_default();
        let asset_file_map =
            Self::read_files(&asset_dir, &asset_dir, true, &vec![], max_size).unwrap_or_default();
        let language_file_map = Self::read_files(
            &language_dir,
            &language_dir,
            true,
            &vec!["mo"],
            DEFAULT_LANGUAGE_SIZE,
        )
        .unwrap_or_default()
        .iter()
        .map(|(k, v)| (k.to_lowercase().to_string(), v.to_string()))
        .collect::<HashMap<_, _>>();
        for (name, path) in templates.clone().into_iter() {
            let path = if path.ends_with(".hbs") {
                format!("{}.hbs", path)
            } else {
                path
            };
            // insert or update
            template_file_map.insert(
                name.clone(),
                theme_dir.join(path).to_string_lossy().to_string(),
            );
        }
        Self::validate_paths(theme_dir.as_ref(), &templates, required.clone())?;
        Ok(Self {
            required,
            theme_dir: Some(theme_dir.to_path_buf()),
            asset_path,
            partial_path,
            language_path,
            partial_file_map,
            template_file_map,
            language_file_map,
            asset_file_map,
            embedded_files: None,
            requirement_type: RequirementType::FileSystem,
            templates: templates.clone(),
            slug: slug.to_string(),
        })
    }

    fn validate_embed<E: rust_embed::RustEmbed + 'static>(
        required: &Vec<String>,
        templates: &Templates,
    ) -> ResultError<()> {
        for name in required {
            if let Some(template_name) = templates.get(name) {
                let file_path = E::get(format!("{}.hbs", template_name).as_str());
                if file_path.is_none() {
                    return Err(Error::file_not_found(format!(
                        "Template file for {} ({}.hbs) not found in embedded assets.",
                        name, template_name
                    )));
                }
            } else {
                return Err(Error::invalid_input(format!(
                    "Template name '{}' is required but not found in configuration.",
                    name
                )));
            }
        }
        Ok(())
    }

    //noinspection DuplicatedCode
    pub fn from_embed<E: rust_embed::RustEmbed + 'static>(
        slug: &str,
        required: Option<Vec<String>>,
        config_base_name: Option<&str>,
    ) -> ResultError<Self> {
        let (config, templates, required, asset_path, partial_path, language_path) =
            Self::reconfigure_init(&required, config_base_name)?;

        let mut template_file_map = HashMap::new();
        let mut partial_file_map = HashMap::new();
        let mut asset_file_map = HashMap::new();
        let mut language_file_map = HashMap::new();
        let asset_prefix = format!("{}/", asset_path);
        let partial_prefix = format!("{}/", partial_path);
        let language_prefix = format!("{}/", language_path);
        let embedded_files = EmbedCollector::new::<E>();
        for (full_path, _file) in &embedded_files {
            if let Some(asset_name) = full_path.strip_prefix(&asset_prefix) {
                asset_file_map.insert(asset_name.to_string(), full_path.to_string());
                continue;
            }
            if let Some(language_name) = full_path.strip_prefix(&language_prefix) {
                if language_name.ends_with(".mo") {
                    language_file_map.insert(language_name.to_lowercase().to_string(), full_path.to_string());
                    continue;
                }
                continue;
            }
            if full_path.ends_with(".hbs") {
                let name_without_ext = full_path.strip_suffix(".hbs").unwrap_or(&full_path);
                if name_without_ext.starts_with(&partial_prefix) {
                    partial_file_map.insert(
                        name_without_ext
                            .strip_prefix(&partial_prefix)
                            .unwrap_or(&name_without_ext)
                            .to_string(),
                        full_path.to_string(),
                    );
                    continue;
                }
                if let Some(template_name) = templates.get(name_without_ext) {
                    template_file_map.insert(template_name.to_string(), full_path.to_string());
                }
                if !name_without_ext.contains('/') {
                    template_file_map.insert(name_without_ext.to_string(), full_path.to_string());
                }
                continue;
            }
        }
        templates.clone().into_iter().for_each(|(name, path)| {
            let path = if path.ends_with(".hbs") {
                format!("{}.hbs", path)
            } else {
                path
            };
            template_file_map.insert(name.clone(), path);
        });

        Self::validate_embed::<E>(&required, &templates)?;

        Ok(Self {
            required,
            asset_path,
            partial_path,
            language_path,
            partial_file_map,
            template_file_map,
            language_file_map,
            asset_file_map,
            embedded_files: Some(embedded_files),
            requirement_type: RequirementType::Embed,
            templates: templates.clone(),
            theme_dir: None,
            slug: slug.to_string(),
        })
    }

    pub fn get_template_file(&self, name: &str) -> ResultError<Cow<'_, [u8]>> {
        let mut name = Hack::clean_unique_path_trim(name);
        if name.ends_with(".hbs") {
            name.truncate(name.len() - 4);
        }
        let path = &self
            .template_file_map()
            .get(&name)
            .ok_or_else(|| Error::invalid_input(format!("Template file '{}' not found.", name)))?;
        match self.requirement_type() {
            RequirementType::FileSystem => {
                let content = fs::read(path).map_err(|e| Error::from_io_error(e))?;
                Ok(Cow::Owned(content))
            }
            RequirementType::Embed => {
                if let Some(embed) = &self.embedded_files {
                    let file = embed.get(path).ok_or_else(|| {
                        Error::invalid_input(format!("Template file '{}' not found.", name))
                    })?;
                    Ok(Cow::Borrowed(file.0.data.as_ref()))
                } else {
                    Err(Error::invalid_input(format!(
                        "Template file '{}' not found.",
                        name
                    )))
                }
            }
        }
    }

    pub fn get_partial_file(&self, name: &str) -> ResultError<Cow<'_, [u8]>> {
        let mut name = Hack::clean_unique_path_trim(name);
        if name.ends_with(".hbs") {
            name.truncate(name.len() - 4);
        }
        let path = self
            .partial_file_map()
            .get(&name)
            .ok_or_else(|| Error::invalid_input(format!("Partial file '{}' not found.", name)))?;
        match self.requirement_type() {
            RequirementType::FileSystem => {
                let content = fs::read(path).map_err(|e| Error::from_io_error(e))?;
                Ok(Cow::Owned(content))
            }
            RequirementType::Embed => {
                if let Some(embed) = &self.embedded_files {
                    let file = embed.get(path).ok_or_else(|| {
                        Error::invalid_input(format!("Partial file '{}' not found.", name))
                    })?;
                    Ok(Cow::Borrowed(file.0.data.as_ref()))
                } else {
                    Err(Error::invalid_input(format!(
                        "Partial file '{}' not found.",
                        name
                    )))
                }
            }
        }
    }
    pub fn get_asset_file(&self, name: &str) -> ResultError<Cow<'_, [u8]>> {
        let name = Hack::clean_unique_path_trim(name);
        let path = self
            .asset_file_map()
            .get(&name)
            .ok_or_else(|| Error::invalid_input(format!("Asset file '{}' not found.", name)))?;
        match self.requirement_type() {
            RequirementType::FileSystem => {
                let content = fs::read(path).map_err(|e| Error::from_io_error(e))?;
                Ok(Cow::Owned(content))
            }
            RequirementType::Embed => {
                if let Some(embed) = self.embedded_files() {
                    let file = embed.get(path).ok_or_else(|| {
                        Error::invalid_input(format!("Asset file '{}' not found.", name))
                    })?;
                    Ok(Cow::Borrowed(file.0.data.as_ref()))
                } else {
                    Err(Error::invalid_input(format!(
                        "Asset file '{}' not found.",
                        name
                    )))
                }
            }
        }
    }

    pub fn get_language_file(&self, name: &str) -> ResultError<Cow<'_, [u8]>> {
        let name = Hack::clean_unique_path_trim(name);
        let path = self
            .language_file_map()
            .get(&name)
            .ok_or_else(|| Error::invalid_input(format!("Language file '{}' not found.", name)))?;
        match self.requirement_type() {
            RequirementType::FileSystem => {
                let content = fs::read(path).map_err(|e| Error::from_io_error(e))?;
                Ok(Cow::Owned(content))
            }
            RequirementType::Embed => {
                if let Some(embed) = self.embedded_files() {
                    let file = embed.get(path).ok_or_else(|| {
                        Error::invalid_input(format!("Language file '{}' not found.", name))
                    })?;
                    Ok(Cow::Borrowed(file.0.data.as_ref()))
                } else {
                    Err(Error::invalid_input(format!(
                        "Asset file '{}' not found.",
                        name
                    )))
                }
            }
        }
    }

    pub fn required_templates_satisfied_with(&self, template_list: Vec<String>) -> (Vec<String>, Vec<String>) {
        let mut satisfied = Vec::new();
        let mut unsatisfied = Vec::new();
        for i in template_list.iter() {
            if !self.required_templates().contains(i) {
                unsatisfied.push(i.clone());
            } else {
                satisfied.push(i.clone());
            }
        }
        (satisfied, unsatisfied)
    }

    pub fn embedded_files(&self) -> &Option<EmbedCollector> {
        &self.embedded_files
    }
    pub fn required_templates(&self) -> &Vec<String> {
        &self.required
    }
    pub fn partial_file_map(&self) -> &HashMap<String, String> {
        &self.partial_file_map
    }
    pub fn template_file_map(&self) -> &HashMap<String, String> {
        &self.template_file_map
    }
    pub fn asset_file_map(&self) -> &HashMap<String, String> {
        &self.asset_file_map
    }
    pub fn language_file_map(&self) -> &HashMap<String, String> {
        &self.language_file_map
    }
    pub fn asset_path(&self) -> &str {
        &self.asset_path
    }
    pub fn partial_path(&self) -> &str {
        &self.partial_path
    }
    pub fn requirement_type(&self) -> &RequirementType {
        &self.requirement_type
    }
    pub fn slug(&self) -> &str {
        &self.slug
    }
    pub fn to_theme(self) -> ResultError<Theme> {
        Theme::new(self)
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    uri: ThemeUri,
    handlebar: Handlebar,
    requirements: Arc<TemplateRequirements>,
}

impl Theme {
    pub fn new(requirements: TemplateRequirements) -> ResultError<Self> {
        let uri = ThemeUri::default();
        let handlebar = Self::handle_bar_init(&requirements, &uri)?;
        Ok(Self {
            requirements: Arc::new(requirements),
            uri,
            handlebar,
        })
    }
    pub fn slug(&self) -> &str {
        self.requirements.slug()
    }
    pub fn uri(&self) -> &ThemeUri {
        &self.uri
    }
    pub fn handlebar(&self) -> &Handlebar {
        &self.handlebar
    }
    pub fn handlebar_mut(&mut self) -> &mut Handlebar {
        &mut self.handlebar
    }
    pub fn requirements(&self) -> &TemplateRequirements {
        &self.requirements
    }
    pub fn render<S: Serialize, T: AsRef<str>>(
        &self,
        template_name: T,
        data: &S,
    ) -> ResultError<String> {
        let template_name = template_name.as_ref();
        self.handlebar().render(template_name, data).map_err(|e| {
            Error::render_error(format!(
                "Failed to render template {}: {}",
                template_name, e
            ))
        })
    }
    fn handle_bar_init(
        requirements: &TemplateRequirements,
        theme_uri: &ThemeUri,
    ) -> ResultError<Handlebar> {
        let mut hb = Handlebar::new();

        // Register Helpers
        handle_bar_register(&mut hb, theme_uri);

        // partial first
        for (name, _) in requirements.partial_file_map() {
            let bytes = requirements.get_partial_file(name)?;
            let content = std::str::from_utf8(&bytes).map_err(|e| {
                Error::parse_error(format!("Partial {} is not valid UTF-8: {}", name, e))
            })?;
            hb.register_partial(name, content).map_err(|e| {
                Error::parse_error(format!("Failed to register partial {}: {}", name, e))
            })?;
        }

        // Template at last
        for (name, path) in requirements.template_file_map() {
            match requirements.requirement_type() {
                RequirementType::FileSystem => {
                    hb.register_template_file(name, Path::new(path))
                        .map_err(|e| {
                            Error::parse_error(format!(
                                "Failed to register template file {}: {}",
                                name, e
                            ))
                        })?;
                }
                RequirementType::Embed => {
                    let bytes = requirements.get_template_file(name)?;
                    let content = std::str::from_utf8(&bytes).map_err(|e| {
                        Error::parse_error(format!("Template {} is not valid UTF-8: {}", name, e))
                    })?;
                    hb.register_template_string(name, content).map_err(|e| {
                        Error::parse_error(format!(
                            "Failed to register embedded template {}: {}",
                            name, e
                        ))
                    })?;
                }
            }
        }

        Ok(hb)
    }
}
