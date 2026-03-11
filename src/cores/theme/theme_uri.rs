#[derive(Debug, Clone)]
pub struct ThemeUri {
    base_url: String,
}

impl ThemeUri {
    pub const ASSET_PREFIX: &'static str = "/theme/assets/";

    pub fn new<T: AsRef<str>>(base_url: T) -> Self {
        Self {
            base_url: Self::clean_base(base_url),
        }
    }
    pub fn set_base_url<T: AsRef<str>>(&mut self, base_url: T) {
        let base_url = Self::clean_base(base_url);
        self.base_url = base_url;
    }
    //noinspection HttpUrlsUsage
    fn clean_base<T: AsRef<str>>(base_url: T) -> String {
        let mut base_url = base_url.as_ref();
        // replace it after ? or # to make it clean
        base_url = base_url
            .split_once(|c| c == '?' || c == '#')
            .map_or(base_url, |(url, _)| url);
        if base_url.ends_with('/') {
            base_url = &base_url[..base_url.len() - 1];
        }
        let mut base = base_url.to_string();
        let lower_url = base_url.to_lowercase();
        // check if not http protocol use /
        if !lower_url.starts_with("http://") && !lower_url.starts_with("https://") {
            // replace leading / to make it clean
            base_url = base_url.strip_prefix('/').unwrap_or(base_url);
            // add prefix / to base url
            base = format!("/{}", base_url).to_string();
        }
        if base == "/" {
            // make empty base url to empty string
            base = "".to_string();
        }
        base
    }

    fn clean_path<T: AsRef<str>>(path: T) -> String {
        let mut path = path.as_ref();
        if !path.is_empty() {
            // sles check if ? or #
            if path.starts_with('#') || path.starts_with('?') {
                return path.to_string();
            }
            // remove the leading slash only if it's not the root path
            if path.starts_with('/') {
                path = path.trim_start_matches('/');
            }
            // check if path == "/" add trailing slash
            if path == "/" {
                path = "/";
            } else if !path.is_empty() {
                return format!("/{}", path);
            }
        }
        path.to_string()
    }

    /// Retrieves the base URL associated with the current instance.
    ///
    /// # Returns
    ///
    /// A string slice (`&str`) that represents the base URL stored within the instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// struct Config {
    ///     base_url: String,
    /// }
    ///
    /// impl Config {
    ///     pub fn base_url(&self) -> &str {
    ///         &self.base_url
    ///     }
    /// }
    ///
    /// let config = Config { base_url: String::from("https://example.com") };
    /// assert_eq!(config.base_url(), "https://example.com");
    /// ```
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    //noinspection HttpUrlsUsage
    /// Generates a complete URL for a given asset path within the theme.
    ///
    /// This method takes a relative path to an asset and resolves it to
    /// the corresponding absolute URL by leveraging the theme's
    /// `asset_path_for` functionality. This is useful for retrieving assets
    /// such as images, stylesheets, or scripts, ensuring they are referenced
    /// correctly within the context of the theme.
    ///
    /// # Type Parameters
    /// - `T`: A type that can be converted to a string slice (`&str`). Typically,
    ///   this will be a `&str` or a `String`.
    ///
    /// # Arguments
    /// - `path`: The relative path of the asset within the theme directory. This
    ///   path is resolved into the full URL via the theme's `asset_path_for` method.
    ///
    /// # Returns
    /// A `String` representing the fully resolved URL for the asset.
    ///
    /// # Example
    /// ```rust
    /// let theme_manager = ThemeManager::new();
    /// let asset_url = theme_manager.assets_url("images/logo.png");
    /// assert_eq!(asset_url, "https://example.com/assets/default/images/logo.png");
    /// ```
    ///
    /// # Note
    /// Ensure that the theme is correctly configured with the appropriate base
    /// path or URL before calling this method, as the resulting URL is dependent
    /// on the theme's settings.
    pub fn assets_url<T: AsRef<str>>(&self, path: T) -> String {
        let cleaned_path = Self::clean_path(path);
        let mut full_url = format!(
            "{}{}",
            self.base_url.trim_end_matches('/'),
            Self::ASSET_PREFIX
        );
        if !full_url.starts_with("http://") || !full_url.starts_with("https://") {
            full_url = full_url.replace("//", "/");
        }
        format!("{}{}", full_url, cleaned_path)
    }
}

impl Default for ThemeUri {
    fn default() -> Self {
        Self::new("")
    }
}
