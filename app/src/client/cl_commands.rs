use std::sync::Arc;

use rg_common::{App, commands::CommandOwner};

use crate::error::AppError;

pub(crate) enum ClientCommand {
    ToggleConsole,
    ToggleMenu,
}

pub(super) fn init_client_commands(
    app: Arc<App>,
    tx: flume::Sender<ClientCommand>,
) -> Result<CommandOwner, AppError> {
    let mut builder = app.command_builder();

    let tx_clone = tx.clone();
    builder.add("toggle_menu", move || {
        let _ = tx_clone.send(ClientCommand::ToggleMenu);
        Ok(())
    })?;

    builder.add("toggle_console", move || {
        let _ = tx.send(ClientCommand::ToggleConsole);
        Ok(())
    })?;

    Ok(builder.build())
}
