use crate::cores::system::error::{Error, ResultError};
use async_trait::async_trait;
use clap::{ArgMatches, Command, FromArgMatches, Parser};
use std::fmt::Debug;
use std::path::PathBuf;

/// Struct representing command-line arguments for the application.
///
/// This struct is used to define and parse the command-line arguments
/// that can be passed to the application at runtime. It uses the `clap`
/// crate's `Parser` derive macro to specify the available arguments and their behavior.
///
/// # Fields
///
/// * `config_file` - Optional path to the configuration file. Can be specified
///   using the `-c` or `--config-file` flag. Only accepts file paths, such as `config.toml`.
///
/// * `quiet` - A flag to suppress all output except for errors. Can be toggled
///   with the `-q` or `--quiet` flag. Mutually exclusive with the `verbose` option.
///
/// * `verbose` - A flag to enable verbose output. Use `-v` or `--verbose`
///   to activate verbose mode. Conflicts with the `quiet` option.
///
/// * `dev` - A flag to enable development mode. use `--dev` to enable it
#[derive(Debug, Parser, Clone, Default)]
#[command(author, version, about, long_about = None)]
pub struct ConsoleArguments {
    /// Path to the configuration file (e.g., config.toml)
    #[arg(
        short = 'c',
        long = "config",
        value_name = "FILE",
        help = "Path to the configuration file"
    )]
    pub config: Option<PathBuf>,
    #[arg(short = 'q', long, help = "Suppress all output except for errors")]
    pub quiet: bool,
    #[arg(short = 'v', long, help = "Enable verbose output")]
    pub verbose: bool,
    #[arg(short = 'T', long, help = "Enable trace output")]
    pub trace: bool,
    #[arg(long = "dev", help = "Set the development mode")]
    pub development: bool,
    #[arg(long = "allow-root", help = "Allow running as root USER", hide = true)]
    pub allow_root: bool,
}

/// Represents the result of an operation performed in a console-based context.
///
/// This enum defines various possible outcomes of an operation, providing a way
/// to categorize and handle different scenarios.
///
/// # Variants
///
/// * `Ok` - Indicates that the operation completed successfully.
/// * `Fail` - Indicates that the operation failed.
/// * `Canceled` - Indicates that the operation was canceled by the USER or system.
/// * `Terminated` - Indicates that the operation was terminated, possibly due to an interrupt or signal.
/// * `Err` - Indicates that an error was encountered during the operation.
#[derive(Debug)]
pub enum ConsoleResult {
    Ok,
    Fail,
    Canceled,
    Terminated,
    Err,
}

#[async_trait(?Send)]
pub trait ConsoleCommand: Debug + Send + Sync + 'static {
    fn get_command(&self) -> Command;

    /// Reconfigures the current object with the provided console arguments and matches.
    ///
    /// This method allows updating the internal state of the object based on new global and
    /// local argument matches derived from the command-line input. It ensures the application
    /// dynamically adapts its setup without needing to restart.
    ///
    /// # Arguments
    ///
    /// * `args` - A reference to the `ConsoleArguments` object that holds argument definitions
    ///   and configurations parsed from the command-line interface.
    /// * `global` - An `ArgMatches` object containing the parsed results of global arguments
    ///   (those applicable across multiple commands or contexts).
    /// * `local` - An `ArgMatches` object containing the parsed results of local arguments
    ///   (those applicable to a specific command or scope).
    ///
    /// # Returns
    ///
    /// A mutable reference to the current object (`Self`) after reconfiguration has been applied,
    /// allowing for method chaining.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut app_config = AppConfig::new();
    /// let console_args = ConsoleArguments::from_env();
    /// let global_matches = parse_global_args(&console_args);
    /// let local_matches = parse_local_args(&console_args);
    ///
    /// app_config.reconfigure(&console_args, global_matches, local_matches);
    /// ```
    ///
    /// # Notes
    ///
    /// - This method assumes valid `global` and `local` matches are provided; it does not perform
    ///   validation but applies them directly to the object's state.
    /// - The behavior of this method may vary based on the implementation of the `reconfigure` logic
    ///   within the object.
    fn reconfigure(
        &mut self,
        args: &ConsoleArguments,
        global: ArgMatches,
        local: ArgMatches
    );

    fn get_console_arguments(&self) -> Option<ConsoleArguments> {
        None
    }
    fn parse_matches_arg(&self, matches: ArgMatches) -> ResultError<Self>
    where
        Self: FromArgMatches,
    {
        match <Self as FromArgMatches>::from_arg_matches(&matches) {
            Ok(e) => {
                Ok(e)
            },
            Err(e) => Err(Error::from_error(e))
        }
    }

    /// Executes a command or operation based on the provided console arguments.
    ///
    /// # Parameters
    /// - `&self`: A reference to the instance of the struct/trait implementing this method.
    /// - `args`: A reference to the `ConsoleArguments` struct, containing the parsed arguments
    ///   and context required for execution.
    /// - `global`: An `ArgMatches` instance containing parsed global arguments shared across commands.
    /// - `local`: An `ArgMatches` instance containing parsed local arguments specific to the current command.
    ///
    /// # Returns
    /// A `ResultError<ConsoleResult>` which represents:
    /// - `Ok(ConsoleResult)`: Upon successful execution of the command.
    /// - `Err(ResultError)`: If an error occurs during the execution process.
    ///
    /// # Errors
    /// This function may return an error in the following scenarios:
    /// - If required arguments are missing or invalid.
    /// - If an error occurs during the execution of the command logic.
    ///
    /// # Example
    /// ```rust
    /// let result = command.execute(&args, global_matches, local_matches).await;
    /// match result {
    ///     Ok(output) => println!("Command executed successfully: {:?}", output),
    ///     Err(e) => eprintln!("Error executing command: {:?}", e),
    /// }
    /// ```
    async fn execute(
        &self,
        args: &ConsoleArguments,
        global: ArgMatches,
        local: ArgMatches,
    ) -> ResultError<ConsoleResult>;
}
