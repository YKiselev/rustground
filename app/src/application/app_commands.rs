use std::sync::{atomic::Ordering, Arc};

use rg_common::{app::App, commands::{CommandBuilder, CommandOwner}};

use crate::error::AppError;

#[allow(dead_code)]
pub(crate) struct AppCommands(CommandOwner);

impl AppCommands {
    pub fn new(app: Arc<App>) -> Result<Self, AppError> {
        let mut builder = CommandBuilder::new(&app.commands);
        let app2 = Arc::clone(&app);
        builder.add("quit", move || {
            app2.exit_flag
                .store(true, Ordering::Relaxed);
            Ok(())
        })?;
        Ok(Self(builder.build()))
    }
}
