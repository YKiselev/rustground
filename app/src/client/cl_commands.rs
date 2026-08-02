use std::sync::{Arc, Mutex};

use rg_common::{App, commands::CommandOwner};

use crate::error::AppError;

pub(crate) enum ClientCommand {
    ToggleConsole,
    ToggleMenu,
}

pub(super) fn init_client_commands(
    app: Arc<App>,
    //state: Arc<Mutex<ClientState>>,
) -> Result<CommandOwner, AppError> {
    let mut builder = app.command_builder();

    //let state_clone = Arc::clone(&state);
    builder.add("cl_restart", move || {
        //let _ = tx_clone.send(ClientCommand::Restart);
        Ok(())
    })?;

    //let state_clone = Arc::clone(&state);
    builder.add("toggle_menu", move || {
        // if let Ok(guard) = state_clone.lock() {
        //     guard.toggle_menu();
        // }
        Ok(())
    })?;

    //let state_clone = Arc::clone(&state);
    builder.add("toggle_console", move || {
        //let _ = tx.send(ClientCommand::ToggleConsole);
        Ok(())
    })?;

    Ok(builder.build())
}
