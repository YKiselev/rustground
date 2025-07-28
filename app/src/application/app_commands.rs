use std::sync::{atomic::AtomicBool, Arc};

use rg_common::{app::App, commands::{CommandBuilder, CommandOwner}};

use crate::error::AppError;

pub(crate) struct AppCommands {
    cmd_owner: CommandOwner,
}

impl AppCommands {
    pub fn new(app: Arc<App>) -> Result<Arc<Self>, AppError> {
        let mut builder = CommandBuilder::new(&app.commands);
        let app2 = Arc::clone(&app);
        builder.add0("quit", move || {
            app2.exit_flag
                .store(true, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        })?;
        let cmd_owner = builder.build();
        Ok(Arc::new(Self { cmd_owner }))
    }
}
