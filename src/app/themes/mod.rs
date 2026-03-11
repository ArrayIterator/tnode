// use rust_embed::RustEmbed;
// #[derive(RustEmbed)]
// #[folder = "src/themes/default/"]
// pub struct DefaultThemeEmbed;

use crate::cores::system::error::ResultError;
use crate::factory::app::App;

pub(crate) fn bind_themes(app: &App) -> ResultError<&App> {
    Ok(app)
}
