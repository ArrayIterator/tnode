use crate::cores::helper::hack::Hack;
use crate::cores::runner::console::{ConsoleArguments, ConsoleCommand, ConsoleResult};
use crate::cores::system::error::{Error, ResultError};
use crate::cores::system::runtime::Runtime;
use crate::factory::config::Config;
use async_trait::async_trait;
use clap::{ArgMatches, Command, CommandFactory, Parser};
use colored::Colorize;
use comfy_table::{Cell, Color};
use log::warn;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::exit;

/// Function to display your Config struct in a beautiful table
pub fn print_config_table(config: &Config) {
    let mut table = Hack::create_table(true);
    let app = config.app();
    table.add_row(vec![
        Cell::new("Application")
            .fg(Color::Magenta)
            .add_attribute(comfy_table::Attribute::Bold),
        Cell::new("Configuration File"),
        Cell::new(config.file().to_string_lossy().to_string()).fg(Color::Blue),
    ]);
    table.add_row(vec![
        Cell::new("")
            .fg(Color::Magenta)
            .add_attribute(comfy_table::Attribute::Bold),
        Cell::new("Root Directory"),
        Cell::new(
            config
                .file()
                .parent()
                .unwrap()
                .to_string_lossy()
                .to_string(),
        )
        .fg(Color::Blue),
    ]);
    table.add_row(vec![
        Cell::new(""),
        Cell::new("Mode"),
        Cell::new(app.mode()).fg(match app.mode() {
            "production" => Color::Green,
            "development" => Color::Red,
            _ => Color::Yellow,
        }),
    ]);
    table.add_row(vec![
        Cell::new(""),
        Cell::new("TCP Listeners"),
        Cell::new(app.tcp().join("\n")),
    ]);
    // socket
    let socket = app.socket();
    table.add_row(vec![
        Cell::new(""),
        Cell::new("Unix Socket"),
        Cell::new(if socket.is_empty() {
            "Disabled".to_string()
        } else {
            socket.to_string()
        }),
    ]);
    table.add_row(vec![
        Cell::new(""),
        Cell::new("Timeout"),
        Cell::new(format!("{} s", app.operation_timeout())),
    ]);

    let db = config.database();
    table.add_row(vec![
        Cell::new("Database")
            .fg(Color::Magenta)
            .add_attribute(comfy_table::Attribute::Bold),
        Cell::new("Host"),
        Cell::new(format!("{}:{}", db.host(), db.port())),
    ]);
    table.add_row(vec![
        Cell::new(""),
        Cell::new("User"),
        Cell::new(db.username()),
    ]);
    table.add_row(vec![
        Cell::new(""),
        Cell::new("Database Name"),
        Cell::new(db.database()),
    ]);
    table.add_row(vec![
        Cell::new(""),
        Cell::new("Connections"),
        Cell::new(format!(
            "Min: {}, Max: {}, Timeout: {} s",
            db.min_connections(),
            db.max_connections(),
            db.acquire_timeout()
        )),
    ]);
    table.add_row(vec![
        Cell::new(""),
        Cell::new("Logging"),
        Cell::new(format!(
            "Enable: {}, Level: {}",
            if db.log_enable() { "Yes" } else { "No" },
            db.log_level()
        )),
    ]);
    table.add_row(vec![
        Cell::new(""),
        Cell::new("Slow Log"),
        Cell::new(format!(
            "Level: {}, Threshold: {} s",
            db.log_slow_level(),
            db.log_threshold()
        )),
    ]);
    println!("{table}");
}

#[derive(Debug, Parser, Default)]
#[clap(name = "init", about = "Initialize configuration file")]
pub(crate) struct Init {
    #[command(flatten)]
    args: ConsoleArguments,
}

#[async_trait(?Send)]
impl ConsoleCommand for Init {
    /// Retrieves the command associated with the current instance of the type.
    ///
    /// This function calls the `command` method from the `CommandFactory` trait
    /// implementation for the current type. It provides a convenient way to obtain
    /// the corresponding `Command` object.
    ///
    /// # Returns
    ///
    /// - `Command`: The command object associated with the current type.
    ///
    /// # Example
    ///
    /// ```rust
    /// let instance = MyStruct::new(); // Assuming MyStruct implements CommandFactory
    /// let command = instance.get_command();
    /// println!("{:?}", command);
    /// ```
    ///
    /// # Notes
    ///
    /// The type implementing this function must also implement the `CommandFactory` trait.
    fn get_command(&self) -> Command {
        <Self as CommandFactory>::command()
    }

    /// Reconfigures the current instance using the provided argument matches.
    ///
    /// This function attempts to parse the `local` argument matches to update the internal
    /// state of the struct. If the parsing is successful, the arguments (`args`)
    /// are updated. Otherwise, a warning is logged to indicate failure.
    ///
    /// # Parameters
    /// - `&mut self`: A mutable reference to the current instance.
    /// - `_: &ConsoleArguments`: A reference to the console arguments (unused in this function).
    /// - `_: ArgMatches`: The global argument matches (unused in this function).
    /// - `local: ArgMatches`: The locally scoped argument matches used for reconfiguration.
    ///
    /// # Behavior
    /// - If `self.parse_matches_arg(local)` returns `Ok(clone)`, the internal `args` are updated
    ///   with `clone.args`.
    /// - If the parsing fails (returns `Err`), a warning is logged stating the failure.
    ///
    /// # Warnings
    /// This function does not use the `ConsoleArguments` or global `ArgMatches` directly and
    /// skips them with `_`. Future modifications to the logic may consider incorporating
    /// these parameters if needed.
    fn reconfigure(&mut self, _: &ConsoleArguments, _: ArgMatches, local: ArgMatches) {
        match self.parse_matches_arg(local) {
            Ok(clone) => {
                self.args = clone.args;
            }
            Err(_) => {
                warn!(target: "app::commands", "Failed to reconfigure Init command with provided arguments");
            }
        }
    }
    /// Retrieves the console arguments stored within the current instance.
    ///
    /// # Returns
    ///
    /// * `Option<ConsoleArguments>` - An `Option` containing a clone of the stored
    /// `ConsoleArguments` if available, or `None` if not.
    ///
    /// # Example
    ///
    /// ```rust
    /// let instance = SomeStruct::new();
    /// if let Some(args) = instance.get_console_arguments() {
    ///     // Use the retrieved arguments
    ///     println!("{:?}", args);
    /// }
    /// ```
    ///
    /// This method assumes that the `self.args` field has been initialized
    /// and contains the necessary command-line arguments.
    fn get_console_arguments(&self) -> Option<ConsoleArguments> {
        Some(self.args.clone())
    }
    async fn execute(
        &self,
        args: &ConsoleArguments,
        _global: ArgMatches,
        local: ArgMatches,
    ) -> ResultError<ConsoleResult> {
        if Runtime::is_root() {
            return Err(Error::invalid_state(
                "Cannot initialize configuration file as root user",
            ));
        }
        let cloned = self.parse_matches_arg(local)?;
        let config_file = if let Some(cfg) = &cloned.args.config {
            cfg.as_path()
        } else {
            match &args.config {
                None => Runtime::config_file(),
                Some(e) => &e,
            }
        };

        let root_dir: &Path = Runtime::root_dir();
        if !root_dir.exists() {
            println!(
                "{}",
                format!(
                    "Configuration directory is not exist: {}",
                    root_dir.to_string_lossy().to_string().red().italic()
                )
                .red()
            );
            exit(1)
        }

        if !root_dir.is_dir() {
            println!(
                "{}",
                format!(
                    "{} is not directory",
                    root_dir.to_string_lossy().to_string().red().italic()
                )
                .red()
            );
            exit(1)
        }
        if config_file.exists() {
            println!(
                "{}",
                format!(
                    "{} : {}",
                    "CONFIGURATION FILE ALREADY EXISTS".red(),
                    config_file.to_string_lossy().to_string().yellow().italic()
                )
                .bold()
            );
            exit(1)
        }
        let ask = |question: &str| -> String {
            print!("{}", question);
            std::io::stdout().flush().unwrap();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            input.trim().to_string()
        };
        let config_content = Config::default_config();
        let configuration = Config::load_from_content(config_content, &config_file);
        let mut already_asked = false;
        match &configuration {
            Err(e) => {
                println!(
                    "{}",
                    format!(
                        "Failed to load default configuration for {} : {}",
                        config_file.to_string_lossy().to_string().red().italic(),
                        e
                    )
                    .red()
                );
                exit(1);
            }
            Ok(conf) => {
                print_config_table(conf);
                loop {
                    let answer: String;
                    if already_asked {
                        answer = ask(&format!("Please answer {} ", "Y(es)/N(o)".bold().yellow()));
                    } else {
                        already_asked = true;
                        answer = ask(&format!(
                            "Do you want to create configuration file? {} : ",
                            "Y(es)/N(o)".bold().yellow()
                        ));
                    }
                    match answer.as_str().to_lowercase().as_str() {
                        "y" | "yes" => {
                            if !root_dir.exists() {
                                println!(
                                    "{}",
                                    format!("Creating configuration directory at : {:?}", root_dir)
                                        .green()
                                );
                                if fs::create_dir_all(&root_dir).is_err() {
                                    println!(
                                        "{}",
                                        format!(
                                            "Failed to create configuration directory at : {}",
                                            root_dir.to_string_lossy().to_string().red().italic()
                                        )
                                        .red()
                                    );
                                    exit(1)
                                }
                            }
                            if !root_dir.is_dir() {
                                println!(
                                    "{}",
                                    format!(
                                        "Configuration directory is not exist: {}",
                                        root_dir.to_string_lossy().to_string().red().italic()
                                    )
                                    .red()
                                );
                                exit(1)
                            }
                            // Creating configuration file
                            if fs::write(&config_file, config_content).is_err() {
                                println!(
                                    "{}",
                                    format!(
                                        "Failed to create configuration file at : {}",
                                        config_file.to_string_lossy().to_string().red().italic()
                                    )
                                    .red()
                                );
                                exit(1)
                            }

                            let vec_dir_storage = vec![
                                ("Storage directory", Runtime::storage_dir()),
                                ("Var directory", Runtime::var_dir()),
                                ("Data directory", Runtime::data_dir()),
                                ("Uploads directory", Runtime::uploads_dir()),
                                ("Temp directory", Runtime::temp_dir()),
                                ("Cache directory", Runtime::cache_dir()),
                                ("Log directory", Runtime::log_dir()),
                                ("Lib directory", Runtime::lib_dir()),
                                ("Themes directory", Runtime::themes_dir()),
                                ("Modules directory", Runtime::modules_dir()),
                                ("Public directory", Runtime::public_dir()),
                            ];
                            for bases in vec_dir_storage {
                                // let named = bases.0;
                                let dir = bases.1;
                                if !dir.exists() {
                                    // println!("{}", format!("Creating {} directory at : {}", named, dir.display().to_string().green()));
                                    fs::create_dir_all(&dir).unwrap_or_else(|_| {
                                        fs::remove_file(&config_file).unwrap_or_default();
                                        println!(
                                            "{}",
                                            format!(
                                                "Failed to create directory for : {}",
                                                dir.to_string_lossy().to_string().red().italic()
                                            )
                                            .red()
                                        );
                                        exit(1)
                                    });
                                }
                            }
                            println!("{}", "CONFIGURATION FILE CREATED!".yellow().bold());
                            println!(
                                "{}",
                                "PLEASE EDIT CONFIGURATION FILE BEFORE RUNNING APPLICATION"
                                    .yellow()
                                    .bold()
                            );
                            exit(0);
                        }
                        "n" | "no" => {
                            println!("{}", "OPERATION CANCELLED!".to_string().red().bold());
                            exit(0);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
