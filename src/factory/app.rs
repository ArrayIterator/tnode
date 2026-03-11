#![allow(dead_code, unused)]
#![deny(unused_imports)]

use crate::cores::runner::cli::Cli;
use crate::cores::runner::console::{ConsoleArguments, ConsoleResult};
use crate::cores::system::error::{Error, ResultError};
use crate::cores::system::runtime::Runtime;
use crate::factory::config::Config;
use crate::factory::factory::Factory;
use log::LevelFilter;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry, fmt, reload};

#[derive(Debug)]
pub struct App {
    run: AtomicBool,
    filter_handle: Handle<EnvFilter, Registry>,
}

static APP: OnceLock<Arc<App>> = OnceLock::new();
struct RunGuard<'a> {
    state: &'a AtomicBool,
}

impl<'a> Drop for RunGuard<'a> {
    fn drop(&mut self) {
        self.state.store(false, Ordering::SeqCst);
    }
}

impl App {
    /// Provides a globally accessible instance of the implementing type.
    ///
    /// This function leverages a `OnceCell` to initialize and store a single static
    /// instance of the type. The instance is created by invoking the `create_instance`
    /// method of the implementing type. Subsequent calls return the same initialized instance.
    ///
    /// # Return
    /// - A static reference to the singleton instance of the type.
    ///
    /// # Requirements
    /// - The implementing type must satisfy the `Sized` trait.
    /// - A global `OnceCell` named `APP` must be defined for the type.
    ///
    /// # Example
    /// ```
    /// // Assuming `MyType` implements `create_instance` and uses `instance`:
    /// let shared_instance = MyType::instance();
    /// ```
    ///
    /// # Notes
    /// - Thread-safe initialization is guaranteed by `OnceCell`.
    /// - This function assumes a global `OnceCell` (`APP`) is properly defined
    ///   and of the correct type.
    pub fn instance() -> Arc<Self>
    where
        Self: Sized,
    {
        APP.get_or_init(|| {
            let (filter_layer, reload_handle) = reload::Layer::new(EnvFilter::new("info"));
            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt::layer())
                .init();
            Arc::new(Self {
                filter_handle: reload_handle,
                run: AtomicBool::new(false),
            })
        })
        .clone()
    }

    /// Determines the appropriate logging level filter based on the provided console arguments.
    ///
    /// # Parameters
    /// - `arg`: A reference to a `ConsoleArguments` instance that contains user-specified options
    ///   affecting the logging level.
    ///
    /// # Returns
    /// - A `LevelFilter` enum value corresponding to the desired logging level:
    ///   - `LevelFilter::Trace` if the `development` or `trace` flag is set.
    ///   - `LevelFilter::Off` if the `quiet` flag is set.
    ///   - `LevelFilter::Debug` if the `verbose` flag is set.
    ///   - `LevelFilter::Info` as the default logging level if none of the above flags are set.
    ///
    /// # Priority
    /// The function evaluates the logging level in the following order of precedence:
    /// 1. `development` (highest priority)
    /// 2. `quiet`
    /// 3. `verbose`
    /// 4. `trace`
    /// 5. Default to `LevelFilter::Info` if no matching flags are set.
    ///
    /// # Example
    /// ```
    /// let args = ConsoleArguments {
    ///     development: true,
    ///     quiet: false,
    ///     verbose: false,
    ///     trace: false,
    /// };
    /// let level = filter_level(&args);
    /// assert_eq!(level, LevelFilter::Trace);
    /// ```
    pub fn filter_level(arg: &ConsoleArguments) -> LevelFilter {
        match () {
            _ if arg.development => LevelFilter::Trace,
            _ if arg.quiet => LevelFilter::Off,
            _ if arg.verbose => LevelFilter::Debug,
            _ if arg.trace => LevelFilter::Trace,
            _ => LevelFilter::Info,
        }
    }

    /// Compares the logging level settings between two sets of console arguments and determines
    /// the appropriate logging level filter based on the merged result.
    ///
    /// # Parameters
    /// - `arg2`: A reference to the second set of [`ConsoleArguments`].
    /// - `global`: A reference to the global set of [`ConsoleArguments`].
    ///
    /// # Returns
    /// - A [`LevelFilter`] representing the appropriate logging level derived from the merged console
    ///   arguments.
    ///
    /// # Merging Behavior
    /// The method merges the two sets of console arguments using the following rules:
    /// - `development`: The resulting value is `true` if either `global.development` or `arg2.development` is `true`.
    /// - `quiet`: The resulting value is `true` if either `global.quiet` or `arg2.quiet` is `true`.
    /// - `verbose`: The resulting value is `true` if either `global.verbose` or `arg2.verbose` is `true`.
    /// - `trace`: The resulting value is `true` if either `global.trace` or `arg2.trace` is `true`.
    /// - `config`: If `global.config` is `Some`, it takes precedence. Otherwise, `arg2.config` is used.
    /// - `allow_root`: The resulting value is `true` if either `global.allow_root` or `arg2.allow_root` is `true`.
    ///
    /// After merging the values, the method invokes [`Self::filter_level`] on the merged configuration
    /// to determine the final [`LevelFilter`].
    ///
    /// # Example
    /// ```
    /// let global_args = ConsoleArguments {
    ///     development: false,
    ///     quiet: true,
    ///     verbose: false,
    ///     trace: false,
    ///     config: Some("config.toml".into()),
    ///     allow_root: false,
    /// };
    ///
    /// let additional_args = ConsoleArguments {
    ///     development: true,
    ///     quiet: false,
    ///     verbose: true,
    ///     trace: false,
    ///     config: None,
    ///     allow_root: true,
    /// };
    ///
    /// let logging_level = compare_level(&additional_args, &global_args);
    /// // `logging_level` will be determined based on the merged configuration.
    /// ```
    pub fn compare_level(arg2: &ConsoleArguments, global: &ConsoleArguments) -> LevelFilter {
        let merged = ConsoleArguments {
            development: global.development || arg2.development,
            quiet: global.quiet || arg2.quiet,
            verbose: global.verbose || arg2.verbose,
            trace: global.trace || arg2.trace,
            config: global.config.clone().or(arg2.config.clone()),
            allow_root: global.allow_root || arg2.allow_root,
        };
        Self::filter_level(&merged)
    }

    /// Returns a reference to the filter handle associated with the current instance.
    ///
    /// The filter handle is used to manage and modify the `EnvFilter` and `Registry`
    /// configuration of the current logging or tracing setup.
    ///
    /// # Returns
    ///
    /// A reference to a [`Handle<EnvFilter, Registry>`] associated with the instance.
    ///
    /// [`Handle<EnvFilter, Registry>`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/reload/struct.Handle.html
    ///
    /// # Example
    ///
    /// ```rust
    /// let filter_handle = instance.get_filter_handle();
    /// filter_handle.modify(|filter| {
    ///     filter.add_directive("my_crate=info".parse().unwrap());
    /// });
    /// ```
    ///
    /// # Notes
    /// - Ensure that any modifications made through the returned handle are thread-safe
    ///   if the handle is being shared across threads.
    pub fn get_filter_handle(&self) -> &Handle<EnvFilter, Registry> {
        &self.filter_handle
    }

    /// Sets the log level for the application.
    ///
    /// This method adjusts the logging configuration dynamically by updating
    /// the filter level based on the provided `LevelFilter`. It maps the given
    /// level to a corresponding `EnvFilter` and reloads the filter handle with
    /// the new configuration.
    ///
    /// # Arguments
    ///
    /// * `filter` - A `LevelFilter` value specifying the desired log level to set.
    ///   The supported levels are:
    ///   - `LevelFilter::Off`: Disables logging entirely.
    ///   - `LevelFilter::Error`: Enables logging of error messages only.
    ///   - `LevelFilter::Warn`: Enables logging of warnings and above.
    ///   - `LevelFilter::Info`: Enables logging of informational messages and above.
    ///   - `LevelFilter::Debug`: Enables debug-level logging and above.
    ///   - `LevelFilter::Trace`: Enables trace-level logging and above.
    ///
    /// # Returns
    ///
    /// * `Ok(())` on successful update of the log level.
    /// * `Err(Error)` if there is an issue reloading the filter handle.
    ///
    /// # Errors
    ///
    /// An error can occur if the filter handle fails to reload with the specified
    /// `EnvFilter`. This error is wrapped as a custom `Error::other`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use log::LevelFilter;
    ///
    /// let logger = MyLogger::new(); // Assume `MyLogger` is a struct implementing this method
    /// logger.set_log_level(LevelFilter::Info).expect("Failed to set log level");
    /// ```
    ///
    /// In the example above, the logging level is set to `Info`, enabling
    /// informational messages, warnings, and errors to be logged.
    pub fn set_log_level(&self, filter: LevelFilter) -> ResultError<()> {
        let env_filter = match filter {
            LevelFilter::Off => EnvFilter::new("off"),
            LevelFilter::Error => EnvFilter::new("error"),
            LevelFilter::Warn => EnvFilter::new("warn"),
            LevelFilter::Info => EnvFilter::new("info"),
            LevelFilter::Debug => EnvFilter::new("debug"),
            LevelFilter::Trace => EnvFilter::new("trace"),
        };
        self.get_filter_handle()
            .reload(env_filter)
            .map_err(|e| Error::other(e))
    }

    /// Runs the main async operation for the program.
    ///
    /// This function performs the following:
    /// 1. Ensures that the `run` state is safely modified to prevent concurrent executions.
    /// 2. Creates a `RunGuard` to manage the runtime state of the operation.
    /// 3. Retrieves the CLI instance through a factory and executes its associated `run` method.
    /// 4. Fetches console arguments, calculates the log level, and logs it appropriately.
    /// 5. Resolves the runtime configuration file based on the console arguments.
    /// 6. Checks if the configuration file exists:
    ///    - If it exists, attempts to load the configuration from the file and attaches it to the factory.
    ///    - If it does not exist, the function completes without errors.
    /// 7. Handles errors arising from configuration file loading.
    ///
    /// ### Returns
    /// - `Ok(ConsoleResult::Canceled)`: If the `run` method is already executing.
    /// - `Err`: If there is an error retrieving the CLI instance or loading the configuration file.
    ///
    /// ### Errors
    /// - Returns any error encountered during:
    ///   - Retrieving dependencies (e.g., CLI instance or configuration file).
    ///   - Loading or parsing the configuration file.
    ///
    /// ### Example Usage
    /// ```rust
    /// let my_runner = Runner::new(factory_instance);
    /// let result = my_runner.run().await;
    /// match result {
    ///     Ok(console_result) => {
    ///         println!("Execution completed with result: {:?}", console_result);
    ///     }
    ///     Err(err) => {
    ///         eprintln!("An error occurred: {:?}", err);
    ///     }
    /// }
    /// ```
    pub async fn run(&self) -> ResultError<ConsoleResult> {
        if self.run.swap(true, Ordering::SeqCst) {
            return Ok(ConsoleResult::Canceled);
        }
        let _guard = RunGuard { state: &self.run };
        let cli = &Factory::pick::<Cli>()?;
        cli.run(|cli, cmd, console| -> ResultError<()> {
            let cli_argument = cli.get_console_arguments();
            let level = Self::compare_level(cli_argument, console);
            Self::log_level(level);
            let config_file =
                Runtime::config_file_of(console.clone(), Some(cli.get_console_arguments().clone()));
            if !config_file.exists() {
                return Ok(());
            }
            match Config::load_from_file(config_file.as_path()) {
                Ok(c) => {
                    Factory::register::<Config>(c);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        })
        .await
    }

    /// Sets the logging level for the application.
    ///
    /// This function modifies the log level to the specified `filter`, which dictates the minimum
    /// severity of log messages that will be output. Any log messages below this level will be filtered out.
    ///
    /// # Parameters
    /// - `filter`: A `LevelFilter` enum value representing the desired log level.
    ///   Possible values include `Off`, `Error`, `Warn`, `Info`, `Debug`, and `Trace`.
    ///
    /// # Returns
    /// - `Ok(())` if the logging level was successfully set.
    /// - `Err(ResultError)` if there was an issue updating the logging level.
    ///
    /// # Examples
    /// ```rust
    /// use log::LevelFilter;
    ///
    /// // Set the log level to `Info`.
    /// let result = YourStruct::log_level(LevelFilter::Info);
    /// match result {
    ///     Ok(()) => println!("Log level successfully updated."),
    ///     Err(err) => eprintln!("Failed to set log level: {:?}", err),
    /// }
    /// ```
    pub fn log_level(filter: LevelFilter) -> ResultError<()> {
        Self::instance().set_log_level(filter)
    }

    /// Checks if the current process or task is running.
    ///
    /// This function reads the value of the `run` atomic boolean using a sequentially consistent
    /// memory ordering to ensure proper synchronization across threads. It returns `true` if the
    /// process or task is running, and `false` otherwise.
    ///
    /// # Returns
    ///
    /// * `true` - If the process or task is running.
    /// * `false` - If the process or task is not running.
    ///
    /// # Example
    ///
    /// ```rust
    /// let status = my_task.is_running();
    /// if status {
    ///     println!("The task is currently running.");
    /// } else {
    ///     println!("The task is not running.");
    /// }
    /// ```
    ///
    /// # Note
    /// The use of `Ordering::SeqCst` ensures the strongest memory ordering guarantees,
    /// which can be important for correctness when dealing with concurrent or multithreaded code.
    pub fn is_running(&self) -> bool {
        self.run.load(Ordering::SeqCst)
    }
}
