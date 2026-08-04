use std::sync::{Arc, Mutex, OnceLock};

use rg_common::{App, commands::CommandOwner};

use crate::error::AppError;

pub(crate) enum ClientCommand {
    ToggleConsole,
    ToggleMenu,
}

/// If at sometime point it would be necessary to call state from command handler:
///
/// But note: callback should not call any methods which can access Vulkan pointers!!!

pub(super) fn init_client_commands(
    app: Arc<App>,
    //state: Arc<ClientState>,
) -> Result<CommandOwner, AppError> {
    let mut builder = app.command_builder();

    //let state_clone = Arc::clone(&state);
    builder.add("cl_restart", || {
        //state_clone.lock().unwrap().toggle_console();
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
