use crate::cores::runner::cli::Cli;
use crate::cores::runner::console::ConsoleArguments;
use crate::cores::system::error::{Error, ResultError};
use crate::cores::system::runtime::Runtime;
use crate::factory::config::Config;
use crate::factory::factory::Factory;
use log::{info, warn};
use nix::libc;
use nix::unistd::User;
use std::sync::Arc;

/// A struct representing a command in the system.
///
/// The `Cmd` struct serves as a placeholder or marker type for implementing
/// specific functionality related to commands. It can be extended or used
/// in different contexts where command processing is required.
///
/// # Examples
///
/// Basic usage:
/// ```rust
/// let command = Cmd;
/// // Use the command in your application logic
/// ```
///
/// # Notes
/// - This struct currently does not contain any fields or methods.
/// - Future implementations may extend its functionality.
pub struct Cmd;

impl Cmd {
    /// Configures the application console by loading or initializing a configuration file.
    ///
    /// # Arguments
    ///
    /// * `global` - A `ConsoleArguments` object containing global arguments for the console application.
    /// * `command` - A boxed trait object implementing `ConsoleCommand` that provides specific console command arguments and behavior.
    ///
    /// # Returns
    ///
    /// Returns a `ResultError` containing an `Arc<Config>` on success if the configuration is successfully loaded or initialized,
    /// or an error if the configuration file cannot be found or loaded.
    ///
    /// # Behavior
    ///
    /// - The function checks if a `Config` instance already exists within the application's factory.
    ///   - If no `Config` exists, it attempts to locate the configuration file path based on the provided `global` and `command` arguments.
    ///   - If the configuration file does not exist at the specified path, the function returns an error indicating
    ///     that the configuration file is missing and suggests running the `init` command to generate it.
    ///   - If the configuration file exists, it is loaded and attached to the application's factory for reuse.
    /// - If a `Config` instance already exists within the application, the function retrieves it.
    ///
    /// # Errors
    ///
    /// - Returns an error of type `Error::file_not_found` if the configuration file cannot be located at the expected path.
    /// - Returns an error propagated from `Config::load_from_file` if the configuration file cannot be successfully loaded.
    /// - Returns an error propagated from `App::app` if retrieving the existing configuration instance fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// let global_args = ConsoleArguments::new();
    /// let command = Box::new(MyConsoleCommand::new());
    /// match console_config(global_args, command) {
    ///     Ok(config) => {
    ///         println!("Configuration loaded successfully: {:?}", config);
    ///     }
    ///     Err(err) => {
    ///         eprintln!("Failed to load configuration: {}", err);
    ///     }
    /// }
    /// ```
    pub fn console_config(
        global: &ConsoleArguments,
        args: Option<ConsoleArguments>,
    ) -> ResultError<Arc<Config>> {
        match Factory::pick::<Config>() {
            Ok(e) => Ok(e),
            Err(_) => {
                let config_file = Runtime::config_file_of(global.clone(), args);
                if !config_file.exists() {
                    return Err(Error::file_not_found(format!(
                        "Config file {} not found. call `{} init` to initialize it",
                        config_file.display(),
                        Runtime::app_name()
                    )));
                }
                Ok(Factory::register(Config::load_from_file(
                    config_file.as_path(),
                )?))
            }
        }
    }

    /// Drops elevated privileges by switching to a specified non-root user.
    ///
    /// This function is typically called when the application starts as the root user
    /// and needs to switch to a less-privileged user based on the configuration provided.
    /// It ensures that the application does not continue running with root privileges,
    /// which can improve security in the event of an exploit or improper system configuration.
    ///
    /// # Parameters
    /// - `config`: A [`Config`] object containing the application's configuration, which
    ///   includes the name of the non-root user the application should switch to.
    ///
    /// # Behavior
    /// 1. The function checks whether the application is currently running as the root user.
    ///    - If the application is not running as root, the function returns immediately with `Ok(())`.
    /// 2. Retrieves the user information for the specified non-root user from the system.
    ///    - If the user does not exist or the lookup fails, an error is returned.
    ///    - If the non-root user is still the root user, an error is returned.
    /// 3. If the application is running as root:
    ///    - Configures the system to retain capabilities after privilege dropping using `libc::prctl`.
    ///    - Switches the current process's effective UID and GID to the non-root user specified in the configuration.
    ///    - If any of these operations fail, an error is returned.
    ///
    /// # Errors
    /// - Returns an [`Error::invalid_data`] if:
    ///   - Retrieving the user information fails.
    ///   - The specified user does not exist on the system.
    ///   - The specified user is root.
    ///   - Setting the UID or GID fails due to a system-level error.
    ///
    /// # Safety
    /// - The use of `libc::prctl` involves an unsafe block as it directly interacts
    ///   with low-level system calls. Proper care must ensure its usage is restricted
    ///   to valid parameters to avoid undefined behavior.
    ///
    /// # Examples
    /// ```no_run
    /// # use crate::drop_privilege;
    /// # use crate::Config;
    /// # fn main() -> Result<(), Error> {
    /// let config = Config::new();
    /// drop_privilege(config)?;
    /// Ok(())
    /// # }
    /// ```
    ///
    /// # Important Notes
    /// - The process must ensure it does not perform this operation if it requires
    ///   root privileges to function correctly.
    /// - GID is switched after setting the UID to ensure dropping privileges effectively.
    ///
    /// # See Also
    /// - [`nix::unistd::setuid()`]
    /// - [`nix::unistd::setgid()`]
    /// - [`libc::prctl()`]
    pub fn drop_privilege<T: AsRef<str>>(user: T) -> ResultError<()> {
        if !Runtime::is_root() {
            return Ok(());
        }
        warn!(target: "factory", "Running as root, dropping privileges");
        let user = user.as_ref();
        let user = User::from_name(user)
            .map_err(|e| {
                Error::invalid_data(format!(
                    "Failed to retrieve user information for '{}': {}",
                    user, e
                ))
            })?
            .ok_or_else(|| {
                Error::invalid_data(format!("User '{}' not found on the system.", user))
            })?;
        if user.uid.is_root() {
            return Err(Error::invalid_data(format!(
                "User '{}' cannot be root.",
                user.name
            )));
        }
        // set gid
        info!(target: "factory", "Running as root, switching to user '{}'", user.name);
        unsafe {
            // keep capability
            libc::prctl(libc::PR_SET_KEEPCAPS, 1, 0, 0, 0);
        }
        info!(target: "factory", "Running as user '{}', dropping privileges", user.name);
        nix::unistd::setgid(nix::unistd::Gid::from_raw(user.gid.as_raw()))
            .map_err(|e| Error::invalid_data(format!("Failed to set GID: {}", e)))?;
        // set uid
        nix::unistd::setuid(nix::unistd::Uid::from_raw(user.uid.as_raw()))
            .map_err(|e| Error::invalid_data(format!("Failed to set UID: {}", e)))?;
        #[cfg(target_os = "linux")]
        {
            use caps::{CapSet, Capability, raise};
            raise(None, CapSet::Effective, Capability::CAP_NET_BIND_SERVICE).map_err(|e| {
                Error::permission_denied(format!("Can not raise cap net privilege: {}", e))
            })?;
        }
        Ok(())
    }

    /// Sets the daemon name based on the provided parameters and updates the command line name.
    ///
    /// This function constructs a daemon name string that includes application name, base name, process ID,
    /// user information, root directory, and an optional configuration file path. The formatted name is
    /// then propagated to the CLI for display or logging purposes.
    ///
    /// # Type Parameters
    /// - `T`: A type that can be referenced as a string slice. This allows for flexibility in the input type for the `based` parameter.
    ///
    /// # Parameters
    /// - `based`: The base name or identifier to include in the daemon name.
    /// - `config`: An optional reference to a [`Config`] object. If provided, the configuration file path
    ///   will be appended to the daemon name.
    ///
    /// # Returns
    /// - `Ok(String)`: On success, returns the formatted daemon name as a string.
    /// - `Err(ResultError<String>)`: If an error occurs while setting the CLI command line name, returns an error.
    ///
    /// # Behavior
    /// - Retrieves the process ID, user details, and root directory information using [`Runtime`] utilities.
    /// - If a configuration object is provided, it retrieves and includes the configuration file path in the name.
    /// - Calls [`Cli::set_cmdline_name`] to update the command line name with the constructed daemon name.
    ///
    /// # Errors
    /// Returns an error if updating the command line name with [`Cli::set_cmdline_name`] fails.
    ///
    /// # Example
    /// ```
    /// # use your_crate::{set_damon_name, Config, ResultError};
    /// let based = "my_daemon";
    /// let config = Some(&Config::default());
    ///
    /// match set_damon_name(based, config) {
    ///     Ok(name) => println!("Daemon name set to: {}", name),
    ///     Err(e) => eprintln!("Failed to set daemon name: {}", e),
    /// }
    /// ```
    ///
    /// # Dependencies
    /// This function relies on the following components:
    /// - [`APP_NAME`]: A constant representing the application name.
    /// - [`Runtime`]: Provides utilities for retrieving process- and user-related data.
    /// - [`Cli`]: Provides CLI-related utilities, such as setting the command line name.
    ///
    /// # Notes
    /// - Ensure that [`Runtime`] and [`Cli`] are properly initialized before calling this function.
    /// - The function is marked as `pub(crate)` and is intended to be used internally within the crate.
    ///
    /// # See Also
    /// - [`Runtime::pid`]
    /// - [`Runtime::user`]
    /// - [`Runtime::root_dir`]
    /// - [`Cli::set_cmdline_name`]
    pub fn set_damon_name<T: AsRef<str>>(based: T, pid: Option<i32>, config: Option<&Config>) {
        let cfg = if let Some(c) = config {
            let cf = c.file();
            format!(" [conf: {}]", &cf.to_string_lossy())
        } else {
            "".to_string()
        };
        let pid = if let Some(p) = pid {
            p
        } else {
            Runtime::pid() as i32
        };
        let name = format!(
            "{}: {} [pid: {}] [user: {}] [root: {}]{}",
            Runtime::app_name(),
            based.as_ref(),
            pid,
            Runtime::user(),
            Runtime::root_dir().to_string_lossy(),
            cfg
        );
        unsafe {
            Cli::set_cmdline_name(&name);
        }
    }
}
