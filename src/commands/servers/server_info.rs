#![allow(dead_code)]
use crate::commands::server::Server;
use crate::cores::generator::uuid::Uuid;
use crate::cores::system::commander::{ControlCommander, ParserInto};
use crate::cores::runner::console::ConsoleResult;
use crate::cores::helper::hack::Hack;
use crate::factory::server_stats::ServerStats;
use std::time::Duration;
use crate::cores::system::error::{ResultError};
use crate::factory::factory::Factory;

pub struct ServerInfo;

impl ServerInfo {
    fn show_vector(info: ServerStats) -> comfy_table::Table {
        let mut table = Hack::create_table(true);
        for (category, category_fg, inner) in Server::server_to_vector(&info, false, true) {
            let mut printed = false;
            for (title, value) in inner {
                let head = if printed {
                    comfy_table::Cell::new("")
                } else {
                    comfy_table::Cell::new(category.to_string())
                        .fg(Hack::rata_tui_color_to_comfy(category_fg))
                };
                printed = true;
                table.add_row(vec![
                    head,
                    comfy_table::Cell::new(title.to_string()),
                    comfy_table::Cell::new(value.to_string()),
                ]);
            }
        }
        println!("{}", table);
        table
    }

    pub(crate) async fn run() -> ResultError<ConsoleResult> {
        let commander = Factory::pick::<ControlCommander>()?;
        let uuid = Uuid::v7().to_string();
        let (buff, size, ..) = commander
            .send_receive_timeout("statistic", uuid, None, Duration::from_secs(2))
            .await?;
        let info = commander.parse_into::<ServerStats>(&buff, size)?;
        Self::show_vector(info);
        Ok(ConsoleResult::Ok)
    }
}
