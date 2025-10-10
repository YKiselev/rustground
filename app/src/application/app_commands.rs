use std::sync::{Arc, atomic::Ordering};

use rg_common::{
    App,
    commands::{CommandBuilder, CommandOwner},
};

use crate::error::AppError;

#[allow(dead_code)]
pub(crate) struct AppCommands(CommandOwner);

impl AppCommands {
    pub fn new(app: Arc<App>) -> Result<Self, AppError> {
        let mut builder = CommandBuilder::new(&app.commands);
        let app_ref = Arc::clone(&app);
        builder.add("quit", move || {
            app_ref.exit_flag.store(true, Ordering::Relaxed);
            Ok(())
        })?;
        // let app_ref = Arc::clone(&app);
        // builder.add("exec", move |name: &str| {
        //     if let Some(script) = app_ref.files.read_file(name) {

        //     }
        //     //app_ref.
        // })?;
        Ok(Self(builder.build()))
    }
}
