use crate::cores::runner::console::{ConsoleArguments, ConsoleCommand, ConsoleResult};
use crate::cores::system::error::{Error, ResultError};
use crate::cores::system::runtime::Runtime;
use clap::{CommandFactory, FromArgMatches, Parser};
use nix::libc;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::sync::Arc;

/// Command Line Interface (CLI) structure for the application.
///
/// This structure is responsible for parsing and managing all command-line
/// arguments and settings provided to the application. It utilizes the `clap`
/// crate's `Parser` and `Args` for command-line argument parsing, and
/// includes additional fields to manage the application's internal state
/// and console commands.
///
/// # Attributes
///
/// * `raw` - A vector of raw input strings representing the unprocessed
///   command-line arguments. This field is not directly parsed from the
///   command-line input (`#[arg(skip)]`).
///
/// * `registry` - A hash map for storing available console commands,
///   where the key is the command name (as a string), and the value is
///   an `Arc<Mutex>` wrapped `ConsoleCommand` trait object. This field
///   is used internally to manage the registration and execution of
///   console commands, and is not part of the command-line arguments
///   (`#[arg(skip)]`).
///
/// * `args` - Flattened structure containing additional arguments defined
///   in the `ConsoleArguments` type. This allows the `Cli` structure to
///   gather subcommand or argument details from `ConsoleArguments`.
///
/// * `running` - A boolean flag used internally to manage the state of
///   the application's runtime. This value is not exposed or modified
///   via command-line arguments (`#[arg(skip)]`).
///
/// # Macros
///
/// * `#[derive(Debug)]` - Automatically derives the `Debug` trait to enable
///   easy debugging of the `Cli` structure.
///
/// * `#[derive(Parser)]` - Used to generate the command-line parser from
///   the `Cli` structure using the `clap` crate.
///
/// # Command Attributes
///
/// * `#[command(name = APP_NAME, version = APP_VERSION_FULL, long_about = None, about = APP_DESCRIPTION)]`
///   Sets metadata for the command-line application, such as the name,
///   version, and description. These parameters should be populated with
///   the appropriate constants (e.g., `APP_NAME`, `APP_VERSION_FULL`, and
///   `APP_DESCRIPTION`).
///
/// * `#[command(flatten)]` - Used to include the fields of the `ConsoleArguments`
///   structure directly in the `Cli` structure for argument parsing.
///
/// * `#[arg(skip)]` - Specifies fields that should not be parsed from
///   the command-line input, but are instead used internally by the application.
///
/// # Usage
///
/// This structure serves as the primary entry point for parsing command-line
/// arguments. Use it to extract and process user input, manage console
/// commands, and store runtime flags for the application.
#[derive(Debug, Parser)]
#[command(
    name = Runtime::app_name(),
    version = Runtime::app_version_full(),
    long_about = None,
    about = Runtime::app_description()
)]
pub struct Cli {
    #[arg(skip)]
    raw: Vec<String>,
    #[arg(skip)]
    empty_arg: Vec<String>,
    #[arg(skip)]
    registry: HashMap<String, Arc<Mutex<dyn ConsoleCommand>>>,
    #[command(flatten)]
    args: ConsoleArguments,
    #[arg(skip)]
    running: bool,
}

impl Cli {
    /// Creates a new instance of the struct.
    ///
    /// # Returns
    /// A new instance of `Self` with the following initialized fields:
    /// - `raw`: A `Vec<String>` collected from command-line arguments, skipping the first argument (typically the executable name).
    /// - `registry`: An empty `HashMap` to store mappings or configurations.
    /// - `args`: An instance of `ConsoleArguments` initialized with its default implementation.
    /// - `running`: A boolean set to `false` indicating the initial state.
    ///
    /// # Example
    /// ```rust
    /// let instance = YourStruct::new();
    /// ```
    pub fn new() -> Self {
        let raw = env::args().skip(1).collect();
        let registry = HashMap::new();
        Self {
            raw,
            registry,
            empty_arg: vec![],
            args: ConsoleArguments::default(),
            running: false,
        }
    }

    /// Returns a reference to the `raw` vector containing the raw command-line arguments.
    ///
    /// # Returns
    ///
    /// A reference to a `Vec<String>` that holds the raw command-line argument strings.
    ///
    /// # Examples
    ///
    /// ```
    /// let args = command.raw_args();
    /// for arg in args {
    ///     println!("{}", arg);
    /// }
    /// ```
    ///
    /// This method provides read-only access to the raw arguments stored within the `raw` field of the struct.
    /// Modifications to the contents of the vector must be performed elsewhere if mutable access is required.
    pub fn raw_args(&self) -> &Vec<String> {
        &self.raw
    }

    /// Retrieves a reference to the `ConsoleArguments` associated with the current instance.
    ///
    /// # Returns
    /// A reference to the `ConsoleArguments` object (`&ConsoleArguments`) stored within the instance.
    ///
    /// # Example
    /// ```rust
    /// let instance = YourStruct::new();
    /// let args = instance.get_console_arguments();
    /// // Use `args` as needed
    /// ```
    ///
    /// This function is useful for accessing the command-line arguments
    /// passed to the application or for retrieving configuration details
    /// encapsulated in the `ConsoleArguments` object.
    pub fn get_console_arguments(&self) -> &ConsoleArguments {
        &self.args
    }

    pub fn add_command<T>(&mut self, cmd: T) -> ResultError<()>
    where
        T: ConsoleCommand + CommandFactory,
    {
        if self.running {
            return Err(Error::already_running(
                "Cannot add command after running the application.",
            ));
        }
        let command = cmd.get_command();
        let name = command.get_name();
        let registry = &mut self.registry;
        if registry.contains_key(name) {
            return Err(Error::already_exists("Command already exists."));
        }
        registry.insert(name.to_string(), Arc::new(Mutex::new(cmd)));
        Ok(())
    }

    pub fn has_command(&self, name: &str) -> bool {
        self.registry.contains_key(name)
    }

    pub fn remove_command(&mut self, name: &str) -> bool {
        if self.running {
            return false;
        }
        if let Some(_) = self.registry.remove(name) {
            return true;
        }
        false
    }
    pub fn set_empty_arg(&mut self, arg: Vec<&str>) {
        self.empty_arg = arg.iter().map(|e|e.to_string()).collect()
    }
    pub fn get_command(&self, name: &str) -> ResultError<Arc<Mutex<dyn ConsoleCommand>>> {
        self.registry
            .get(name)
            .cloned()
            .ok_or_else(|| Error::not_found(format!("Command {} not found.", name)))
    }

    pub fn merge_arg_prioritize<T: AsRef<ConsoleArguments>, U: AsRef<ConsoleArguments>>(
        prior_arg: T,
        additional_arg: Option<U>,
    ) -> ConsoleArguments {
        let mut console_arg = prior_arg.as_ref().clone();
        if let Some(arg) = additional_arg {
            let arg = arg.as_ref();
            if let Some(conf) = &arg.config {
                console_arg.config = Some(conf.clone());
            }
            console_arg.trace = arg.trace || console_arg.trace;
            console_arg.quiet = arg.quiet || console_arg.quiet;
            console_arg.verbose = arg.verbose || console_arg.verbose;
            console_arg.development = arg.development || console_arg.development;
        }
        console_arg
    }

    pub async fn run<F>(&self, before: F) -> ResultError<ConsoleResult>
    where
        F: FnOnce(&Cli, Arc<&dyn ConsoleCommand>, &ConsoleArguments) -> ResultError<()>,
    {
        let mut master = Self::command()
            .allow_external_subcommands(false)
            .no_binary_name(true)
            .arg_required_else_help(true);
        for (_, cmd_obj) in &self.registry {
            let cmd_obj = cmd_obj.lock().get_command();
            master = master.subcommand(cmd_obj);
        }
        let mut raw = self.raw.clone();
        if raw.is_empty() && !self.empty_arg.is_empty() {
            raw = self.empty_arg.clone();
        }
        let mut matches = match master.try_get_matches_from(&raw) {
            Ok(e) => e,
            Err(e) => e.exit(),
        };
        let res = match <Self as FromArgMatches>::from_arg_matches_mut(&mut matches) {
            Ok(e) => e,
            Err(e) => {
                // Since this is more of a development-time error, we aren't doing as fancy of a quit
                // as `get_matches`
                e.exit()
            }
        };
        let (command, args) = matches
            .subcommand()
            .ok_or_else(|| Error::unsupported("No command provided."))?;
        let cmd_arc = self.get_command(command)?;
        let mut cmd_guard = cmd_arc.lock();
        cmd_guard.reconfigure(&res.args, matches.clone(), args.clone());
        let cmd = Arc::new(&*cmd_guard);
        before(self, cmd, &res.args)?;
        cmd_guard
            .execute(&res.args, matches.clone(), args.clone())
            .await
    }

    /// Sets the process's command-line name and updates `program_invocation_name`.
    ///
    /// This function sets the current process's command-line name, which is visible
    /// via tools like `ps` or `top` under the process name column. It uses the
    /// `prctl` system call with the `PR_SET_NAME` option to achieve this. Additionally,
    /// it updates the `program_invocation_name` field if it is available, allowing
    /// the change to be reflected more consistently.
    ///
    /// # Arguments
    ///
    /// * `name`: A string slice specifying the new command-line name to set.
    ///   The string must not exceed the system-supported length for process names,
    ///   typically 15 characters for most systems.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success or an `Err(Error)` if an error occurs. Possible
    /// error scenarios include creating a `CString` from the input name failing due
    /// to embedded null bytes.
    ///
    /// # Errors
    ///
    /// An error of type `Error::from_io_error` is returned if:
    /// - The provided `name` argument contains internal null bytes, which are
    ///   forbidden in C strings.
    /// - The underlying system call fails (e.g., due to invalid permissions).
    ///
    /// # Safety
    ///
    /// This function performs unsafe operations by interacting directly with libc
    /// and raw pointers. It performs the following actions:
    /// - Calls `libc::prctl` to set the process name. Improper usage or invalid
    ///   arguments to `prctl` can lead to undefined behavior.
    /// - Modifies the `program_invocation_name` static mutable variable directly.
    ///   The validity of this pointer is assumed but not enforced, which could
    ///   result in undefined behavior if used improperly.
    ///
    /// **Caution:** Misuse of this function can result in undefined behavior, memory
    /// corruption, or crashing the program. Ensure the provided name is valid and
    /// doesn't exceed the maximum allowed length.
    ///
    /// # Example
    ///
    /// ```
    /// use your_crate_name::set_cmdline_name;
    ///
    /// fn main() {
    ///     if let Err(e) = set_cmdline_name("my_process") {
    ///         eprintln!("Failed to set process name: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// - Updating the `program_invocation_name` enables reflective changes in debugging
    ///   tools and logs. However, the behavior on certain systems may vary.
    /// - The maximum length of `name` should not exceed the system-specific maximum
    ///   for process names. Exceeding this length may truncate the string or lead to
    ///   undefined behavior due to buffer overflows.
    pub unsafe fn set_cmdline_name(name: &str) {
        unsafe {
            unsafe extern "C" {
                static mut program_invocation_name: *mut libc::c_char;
                static mut environ: *mut *mut libc::c_char;
            }

            if program_invocation_name.is_null() || environ.is_null() {
                return;
            }

            let start = program_invocation_name as usize;
            let mut end = start;

            let mut env = environ;
            while !(*env).is_null() {
                let ptr = *env;
                let len = libc::strlen(ptr) as usize;
                let current_end = ptr as usize + len;
                if current_end > end {
                    end = current_end;
                }
                env = env.add(1);
            }

            let total_len = end - start;

            libc::memset(program_invocation_name as *mut libc::c_void, 0, total_len);

            let bytes = name.as_bytes();
            let copy_len = std::cmp::min(bytes.len(), total_len - 1);

            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                program_invocation_name as *mut u8,
                copy_len,
            );

            *program_invocation_name.add(copy_len) = 0;
        }
    }
}

impl Default for Cli {
    fn default() -> Self {
        Self::new()
    }
}