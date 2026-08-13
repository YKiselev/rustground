use std::sync::Arc;

use rg_common::{App, commands::CommandOwner};

use crate::{
    client::{BoolFlag, SharedState, cl_game_actions::GameActionFlags},
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

    let state_clone = Arc::clone(&state);
    builder.add("forward", move || {
        state_clone
            .game_actions
            .insert(GameActionFlags::FORWARD.bits());
        Ok(())
    })?;

    let state_clone = Arc::clone(&state);
    builder.add("backward", move || {
        state_clone
            .game_actions
            .insert(GameActionFlags::BACKWARD.bits());
        Ok(())
    })?;

    let state_clone = Arc::clone(&state);
    builder.add("left", move || {
        state_clone
            .game_actions
            .insert(GameActionFlags::LEFT.bits());
        Ok(())
    })?;

    let state_clone = Arc::clone(&state);
    builder.add("right", move || {
        state_clone
            .game_actions
            .insert(GameActionFlags::RIGHT.bits());
        Ok(())
    })?;

    let state_clone = Arc::clone(&state);
    builder.add("strafe_left", move || {
        state_clone
            .game_actions
            .insert(GameActionFlags::STRAFE_LEFT.bits());
        Ok(())
    })?;

    let state_clone = Arc::clone(&state);
    builder.add("strafe_right", move || {
        state_clone
            .game_actions
            .insert(GameActionFlags::STRAFE_RIGHT.bits());
        Ok(())
    })?;

    let state_clone = Arc::clone(&state);
    builder.add("jump", move || {
        state_clone
            .game_actions
            .insert(GameActionFlags::JUMP.bits());
        Ok(())
    })?;

    let state_clone = Arc::clone(&state);
    builder.add("crouch", move || {
        state_clone
            .game_actions
            .insert(GameActionFlags::CROUCH.bits());
        Ok(())
    })?;

    let state_clone = Arc::clone(&state);
    builder.add("fire", move || {
        state_clone
            .game_actions
            .insert(GameActionFlags::FIRE.bits());
        Ok(())
    })?;

    let state_clone = Arc::clone(&state);
    builder.add("sprint", move || {
        state_clone
            .game_actions
            .insert(GameActionFlags::SPRINT.bits());
        Ok(())
    })?;

    let state_clone = Arc::clone(&state);
    builder.add("use", move || {
        state_clone
            .game_actions
            .insert(GameActionFlags::USE.bits());
        Ok(())
    })?;

    Ok(builder.build())
}
