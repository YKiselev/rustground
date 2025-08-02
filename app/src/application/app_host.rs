use std::sync::Arc;

use rg_common::{App, Arguments};

use super::app_commands::AppCommands;

pub struct AppHost {
    pub app: Arc<App>,
    _app_commands: AppCommands,
}

impl AppHost {
    pub fn new(args: Arguments) -> Self {
        let app = Arc::new(App::new(args));
        let app_commands = AppCommands::new(app.clone()).expect("Failed to register app commands!");
        Self {
            app,
            _app_commands: app_commands
        }
    }
}
