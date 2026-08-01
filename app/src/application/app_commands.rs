use std::sync::Arc;

use rg_common::{App, commands::CommandOwner};

use crate::{application::trigger_exit, error::AppError};

#[allow(dead_code)]
pub(crate) struct AppCommands(CommandOwner);

impl AppCommands {
    pub fn new(app: Arc<App>) -> Result<Self, AppError> {
        let mut builder = app.command_builder();
        let app_ref = Arc::clone(&app);
        builder.add("quit", move || {
            trigger_exit();
            Ok(())
        })?;
        Ok(Self(builder.build()))
    }
}
