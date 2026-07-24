use std::sync::Arc;

use rg_common::{App, Arguments};
use tracing::error;

use super::app_commands::AppCommands;

pub struct AppHost {
    pub app: Arc<App>,
    _commands: AppCommands,
}

impl AppHost {
    pub fn new(args: Arguments) -> Self {
        let app = Arc::new(App::new(args));
        let commands = AppCommands::new(app.clone())
            .inspect_err(|e| error!("Unable to register commands: {}", e))
            .unwrap();
        Self {
            app,
            _commands: commands,
        }
    }
}
