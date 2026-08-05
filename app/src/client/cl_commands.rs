use std::sync::Arc;

use rg_common::{App, commands::CommandOwner};

use crate::{
    client::{BoolFlag, SharedState},
    error::AppError,
};

pub(super) fn init_client_commands(
    app: Arc<App>,
    state: Arc<SharedState>,
) -> Result<CommandOwner, AppError> {
    let mut builder = app.command_builder();

    let state_clone = Arc::clone(&state);
    builder.add("toggle_menu", move || {
        state_clone.toggle_menu.toggle();
        Ok(())
    })?;

    let state_clone = Arc::clone(&state);
    builder.add("toggle_console", move || {
        state_clone.toggle_console.toggle();
        Ok(())
    })?;

    let state_clone = Arc::clone(&state);
    builder.add("print_fps", move || {
        state_clone.print_fps.toggle();
        Ok(())
    })?;

    Ok(builder.build())
}
