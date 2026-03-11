use crate::app::commands::server::Server;
use crate::cores::generator::uuid::Uuid;
use crate::cores::helper::user::User;
use crate::cores::runner::console::{ConsoleArguments, ConsoleCommand, ConsoleResult};
use crate::cores::system::commander::{ControlCommander, ParserInto};
use crate::cores::system::error::{Error, ResultError};
use crate::factory::cmd::Cmd;
use crate::factory::factory::Factory;
use crate::factory::server_stats::ServerStats;
use async_trait::async_trait;
use clap::{ArgMatches, Command, CommandFactory, Parser};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{event, execute};
use log::warn;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Flex};
use ratatui::prelude::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Row, Table};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Parser, Default)]
#[clap(name = "monitor", about = "Monitor the server")]
pub(crate) struct Monitor {
    #[command(flatten)]
    args: ConsoleArguments,
}

impl Monitor {
    /// Constructs a `Table` representing the server statistics for a user interface.
    ///
    /// This function processes the given server statistics and generates a tabular view
    /// that can be displayed in a TUI (terminal user interface). The table consists of
    /// categorized rows representing various server stat categories, keys, and their
    /// respective values.
    ///
    /// # Parameters
    /// - `info`: A reference to the `ServerStats` object containing server data to be displayed.
    /// - `connected`: A boolean flag indicating whether the server is currently connected.
    /// - `terminal_title`: An optional string specifying the title of the terminal window.
    ///   If `None` is provided, a default title of `"Server Info Monitoring"` is used.
    ///
    /// # Returns
    /// A `Table<'static>` instance formatted with the appropriate data and styles.
    /// It has:
    /// - Rows corresponding to server categories, keys, and values.
    /// - A title indicating the terminal status (connected/disconnected).
    /// - Styles and spacing adjusted dynamically based on the provided data.
    ///
    /// # Behavior
    /// - Iterates through the server data (via `Server::server_to_vector`) to generate
    ///   row entries for each category, key, and value.
    /// - Applies specific styles to indicate different categories and values.
    /// - Adds spacing and formatting to enhance readability.
    /// - Adjusts the table's border style and title based on the `connected` state.
    ///
    /// # Example
    /// ```rust
    /// let server_stats = ServerStats::new(); // Assume this provides mock server data
    /// let is_connected = true;
    /// let table = get_vector_rata_tui(&server_stats, is_connected, Some("Custom Title".to_string()));
    /// ```
    ///
    /// # Notes
    /// - Rows are grouped by category, with a blank row inserted between categories for separation.
    /// - If `connected` is `false`, the table title and border are styled with `Color::Red`
    ///   to indicate the disconnected state.
    /// - Some parts of the table styling (e.g., headers) are commented out and can be enabled if needed.
    ///
    /// # Dependencies
    /// - This function relies on organizational components such as `ServerStats`, `Server::server_to_vector`,
    ///   and various `tui` crate building blocks like `Table`, `Row`, `Cell`, `Style`, `Block`, `Borders`,
    ///   and `Constraint`.
    ///
    /// # Returns
    /// A formatted `Table` object ready for rendering in a TUI interface.
    pub(crate) fn get_vector_rata_tui(
        info: &ServerStats,
        connected: bool,
        terminal_title: Option<String>,
    ) -> Table<'static> {
        let mut rows = Vec::new();
        // let header_style = Style::default()
        //     .fg(Color::Cyan)
        //     .add_modifier(Modifier::BOLD);
        // let header = Row::new(vec![" Category ", " Key ", " Value "])
        //     .style(header_style)
        //     .height(1)
        //     .bottom_margin(1);
        for (category, category_fg, inner) in Server::server_to_vector(info, true, connected) {
            let mut printed = false;
            for (title, value) in inner {
                let head = if printed {
                    Cell::from("")
                } else {
                    Cell::from(format!(" {} ", category)).style(
                        Style::default()
                            .fg(category_fg)
                            .add_modifier(Modifier::BOLD),
                    )
                };
                printed = true;

                rows.push(
                    Row::new(vec![
                        head,
                        Cell::from(format!(" {} ", title))
                            .style(Style::default().add_modifier(Modifier::BOLD)),
                        Cell::from(format!(" {} ", value)),
                    ])
                    .bottom_margin(0),
                );
            }
            rows.push(Row::new(vec![
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
            ]));
        }
        let base_style = if connected {
            Style::default()
        } else {
            Style::default().fg(Color::Red)
        };
        let terminal_title = terminal_title.unwrap_or_else(|| "Server Info Monitoring".to_string());
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .title(format!(
                " {}{} ",
                terminal_title,
                if connected { "" } else { " (Disconnected)" }
            ))
            .border_style(base_style)
            .title_style(base_style);
        Table::new(
            rows,
            [
                Constraint::Max(15),
                Constraint::Min(25),
                Constraint::Min(100),
            ],
        )
        .column_spacing(1)
        // .header(header)
        .block(block)
        .flex(Flex::Legacy)
    }
}

#[async_trait(?Send)]
impl ConsoleCommand for Monitor {
    fn get_command(&self) -> Command {
        <Self as CommandFactory>::command()
    }

    fn reconfigure(&mut self, _: &ConsoleArguments, _: ArgMatches, local: ArgMatches) {
        match self.parse_matches_arg(local) {
            Ok(clone) => {
                self.args = clone.args;
            }
            Err(_) => {
                warn!(target: "app::commands", "Failed to reconfigure Monitor command with provided arguments");
            }
        }
    }
    fn get_console_arguments(&self) -> Option<ConsoleArguments> {
        Some(self.args.clone())
    }

    /// Asynchronously executes the primary functionality of the application, enabling terminal-based,
    /// real-time stats visualization with TUI (Text-based User Interface).
    ///
    /// This method performs the following steps:
    /// 1. Configures the terminal for raw mode and sets up the alternate screen
    ///    for non-blocking interactions.
    /// 2. Establishes a terminal backend using Crossterm for rendering widgets in the TUI.
    /// 3. Creates an Arc-wrapped, tokio Mutex-protected terminal instance for safe concurrent access.
    /// 4. Uses a commander instance to send requests to a server for statistics and processes responses
    ///    to render real-time data onto the terminal UI.
    /// 5. Handles terminal user input (`q` for quit, or `Ctrl+C` to exit) within a loop.
    /// 6. Gracefully handles errors while parsing server responses or updating the terminal UI.
    ///
    /// # Arguments
    /// * `_: &ConsoleArguments` - Unused placeholder for console-specific arguments (reserved for future use).
    /// * `_global: ArgMatches` - Unused placeholder for global argument matches (reserved for future use).
    /// * `_: ArgMatches` - Unused placeholder for additional argument matches (reserved for future use).
    ///
    /// # Returns
    /// * `ResultError<ConsoleResult>` - Returns `ConsoleResult::Ok` upon successful execution, or an error
    ///   if there are failures in terminal configuration, rendering, command execution, or other operations.
    ///
    /// # Errors
    /// This function may return an error if:
    /// * Enabling raw mode or entering the alternate screen fails.
    /// * Sending or parsing server responses fails.
    /// * Terminal rendering or configuration issues occur.
    /// * Handling terminal input events (e.g., reading key presses) encounters an issue.
    ///
    /// # Features
    /// * Listens to a server for periodic statistics updates using an async command/response pattern.
    /// * Render data dynamically using Rata-TUI widgets for visualization.
    /// * Responsive and controlled UI update mechanism with user-defined interaction support.
    ///
    /// # User Interaction
    /// * Press `q` to quit the application.
    /// * Press `Ctrl+C` to exit the program.
    ///
    /// # Cleanup
    /// * Ensures that raw mode is disabled and the terminal resets to its original state (leaves the alternate screen)
    ///   before exiting, even in case of errors.
    ///
    /// # Example Usage
    /// `execute` is typically called as part of a console application flow, integrating with a larger
    /// command-commanding system. It continuously updates and renders stats from the server while being
    /// responsive to user commands to exit.
    ///
    /// ```rust
    /// // Example usage
    /// let app = MyApp {};
    /// match app.execute(&args, global_matches, action_matches).await {
    ///     Ok(result) => println!("Execution completed successfully"),
    ///     Err(e) => eprintln!("Error: {}", e),
    /// }
    /// ```
    async fn execute(
        &self,
        global: &ConsoleArguments,
        _: ArgMatches,
        _: ArgMatches,
    ) -> ResultError<ConsoleResult> {
        if User::current().is_root() {
            let config = Cmd::console_config(global, self.get_console_arguments())?;
            Cmd::drop_privilege(&config.app().user())?;
        }

        Cmd::set_damon_name("monitor", None, None);
        enable_raw_mode().map_err(Error::from)?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(Error::from)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).map_err(Error::from)?;
        let arc = Arc::new(tokio::sync::Mutex::new(terminal));
        let mut last_table: Arc<Option<ServerStats>> = Arc::new(None);
        let commander = Factory::pick::<ControlCommander>()?;
        loop {
            let term = Arc::clone(&arc);
            let uuid = Uuid::v7().to_string();
            match commander
                .send_receive_timeout("statistic", uuid, None, Duration::from_secs(2))
                .await
            {
                Ok((buff, size, ..)) => match commander.parse_into::<ServerStats>(&buff, size) {
                    Ok(info) => {
                        let mut terminal = term.lock().await;
                        let inf = info.clone();
                        let vec = Self::get_vector_rata_tui(&info, true, None);
                        last_table = Arc::new(Some(inf));
                        match terminal.draw(|f| f.render_widget(vec, f.area())) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                    Err(e) => {
                        println!("Failed to parse info command response: {}", e);
                    }
                },
                Err(e) => {
                    commander.close();
                    let mut terminal = term.lock().await;
                    if let Some(t) = last_table.clone().as_ref() {
                        let vec = Self::get_vector_rata_tui(t, false, None);
                        match terminal.draw(|f| f.render_widget(vec, f.area())) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    } else {
                        let _ = terminal.draw(|f| {
                            let area = f.area();
                            let msg = format!("Target Down: {} | Retrying...", e);
                            let err_widget = ratatui::widgets::Paragraph::new(msg)
                                .style(Style::default().fg(Color::Red))
                                .block(
                                    Block::default()
                                        .borders(Borders::ALL)
                                        .title(" Connection Error "),
                                );
                            f.render_widget(err_widget, area);
                        });
                    }
                }
            };

            if event::poll(Duration::from_millis(1))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == event::KeyEventKind::Press {
                        if key.code == KeyCode::Char('q')
                            || (key.code == KeyCode::Char('c')
                                && key.modifiers.contains(KeyModifiers::CONTROL))
                        {
                            break;
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        disable_raw_mode()?;
        execute!(std::io::stdout(), LeaveAlternateScreen)?;
        Ok(ConsoleResult::Ok)
    }
}
