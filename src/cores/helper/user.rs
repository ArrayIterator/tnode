use nix::unistd::{Group, Uid, User as NixUser};
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct User {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct UserDetail {
    pub user: NixUser,
    pub group: Group,
}

// Implement methods for User struct
impl User {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn current() -> Self {
        Self::from(nix::unistd::getuid())
    }

    /// Create new object USER with username
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// let USER = User::with("root");
    /// ```
    pub fn with<T: AsRef<str>>(name: T) -> Self {
        Self {
            name: name.as_ref().to_string(),
        }
    }

    /// Create new object USER with username
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// let USER = User::from_name("root");
    /// ```
    pub fn from_name<T: AsRef<str>>(name: T) -> Self {
        Self::with(name)
    }

    /// Check if USER is root
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// let USER = User::with("root");
    /// if USER.is_root() {
    ///     println!("User is root");
    /// }
    /// ```
    pub fn is_root(&self) -> bool {
        match Self::user_of(&self.name) {
            Some(u) => u.uid.is_root(),
            None => false,
        }
    }

    /// Get USER home directory
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// let USER = User::with("root");
    /// if let Some(home) = USER.home() {
    ///     println!("Home directory: {}", home);
    /// }
    /// ```
    pub fn home(&self) -> Option<String> {
        if let Some(user) = Self::user_of(&self.name) {
            Some(user.dir.to_string_lossy().to_string())
        } else {
            None
        }
    }

    /// Get USER detail information
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// let USER = User::with("root");
    /// if let Some(detail) = USER.detail() {
    ///     println!("User detail: {:?}", detail);
    /// }
    /// ```
    pub fn detail(&self) -> Option<UserDetail> {
        Self::detail_of(&self.name)
    }

    /// Get USER primary group
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// let USER = User::with("root");
    /// if let Some(group) = USER.group() {
    ///     println!("Group: {:?}", group);
    /// }
    /// ```
    pub fn group(&self) -> Option<Group> {
        self.detail().map(|d| d.group)
    }

    /// Get nix USER object
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// let USER = User::with("root");
    /// if let Some(nix_user) = USER.USER() {
    ///     println!("Nix USER: {:?}", nix_user);
    /// }
    /// ```
    pub fn user(&self) -> Option<NixUser> {
        Self::user_of(&self.name)
    }

    /// Get USER ID
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// let USER = User::with("root");
    /// if let Some(uid) = USER.uid() {
    ///     println!("UID: {}", uid);
    /// }
    /// ```
    pub fn uid(&self) -> Option<Uid> {
        self.user().map(|u| u.uid)
    }

    /// Get group ID
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// let USER = User::with("root");
    /// if let Some(gid) = USER.gid() {
    ///     println!("GID: {}", gid);
    /// }
    /// ```
    pub fn gid(&self) -> Option<nix::unistd::Gid> {
        self.group().map(|g| g.gid)
    }

    /// Check if USER is inside a group
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// use nix::unistd::Group;
    /// let USER = User::with("root");
    /// if let Some(group) = User::group_of("root") {
    ///     if USER.inside_of(group) {
    ///         println!("User is in root group");
    ///     }
    /// }
    /// ```
    pub fn inside_of(&self, group: Group) -> bool {
        let user = match self.user() {
            Some(u) => u.name,
            None => return false,
        };
        group.mem.contains(&user)
    }

    /// Get USER detail by username
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// if let Some(detail) = User::detail_of("root") {
    ///     println!("User detail: {:?}", detail);
    /// }
    /// ```
    pub fn detail_of<T: AsRef<str>>(u: T) -> Option<UserDetail> {
        let user = NixUser::from_name(u.as_ref()).ok()??;
        let group = Group::from_gid(user.gid).ok()??;
        Some(UserDetail { user, group })
    }

    /// Get nix USER object by username
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// if let Some(nix_user) = User::user_of("root") {
    ///     println!("Nix USER: {:?}", nix_user);
    /// }
    /// ```
    pub fn user_of<T: AsRef<str>>(u: T) -> Option<NixUser> {
        NixUser::from_name(u.as_ref()).ok()?
    }

    /// Get group object by group name
    /// # Example:
    /// ```rust
    /// use crate::cores::system::USER::User;
    /// if let Some(group) = User::group_of("root") {
    ///     println!("Group: {:?}", group);
    /// }
    /// ```
    pub fn group_of<T: AsRef<str>>(u: T) -> Option<Group> {
        Group::from_name(u.as_ref()).ok()?
    }
}

impl From<NixUser> for User {
    fn from(value: NixUser) -> Self {
        Self { name: value.name }
    }
}

impl From<&NixUser> for User {
    fn from(value: &NixUser) -> Self {
        Self {
            name: value.name.clone(),
        }
    }
}

impl From<Uid> for User {
    fn from(value: Uid) -> Self {
        Self {
            name: NixUser::from_uid(value)
                .ok()
                .flatten()
                .map(|u| u.name)
                .unwrap_or_else(|| "unknown".to_string()),
        }
    }
}
impl From<&Uid> for User {
    fn from(value: &Uid) -> Self {
        Self::from(*value)
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}