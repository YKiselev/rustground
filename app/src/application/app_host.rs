use std::sync::Arc;

use rg_common::{app::App, Arguments};

use super::app_commands::AppCommands;

pub struct AppHost {
    pub app: Arc<App>,
    app_commands: Arc<AppCommands>,
}

impl AppHost {
    pub fn new(args: Arguments) -> Self {
        let app = App::new(args);
        let app_commands = AppCommands::new(app.clone()).unwrap();
        Self {
            app,
            app_commands
        }
    }
}
