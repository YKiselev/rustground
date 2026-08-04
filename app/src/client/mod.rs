mod cl_actions;
mod cl_app_handler;
mod cl_async_dispatch;
mod cl_commands;
mod cl_config;
mod cl_console;
mod cl_context;
mod cl_fps;
mod cl_game_actions;
mod cl_game_overlay;
mod cl_menu;
mod cl_net;
mod cl_ui_layer;
mod cl_world;
mod client;

use std::sync::{Arc, RwLock};

pub(crate) use cl_async_dispatch::{Request, Response, run_client_worker};

use rg_common::{App, commands::CommandOwner, world::HyperCube};
use rg_vulkan::renderer::VulkanRenderer;
use winit::keyboard::ModifiersState;

use crate::client::{
    cl_actions::ClientActions, cl_config::ClientConfig, cl_console::Console, cl_fps::FrameStats,
    cl_game_actions::GameActions, cl_game_overlay::GameOverlay, cl_menu::Menu,
    cl_net::ClientNetwork,
};

pub enum ClientEvent {
    Exiting,
}

#[derive(Debug, Default)]
struct WindowState {
    modifiers: ModifiersState,
    focused: bool,
    cursor_captured: bool,
}

pub struct Client {
    app: Arc<App>,
    config: Arc<RwLock<ClientConfig>>,
    net: ClientNetwork,
    renderer: Option<VulkanRenderer>,
    renderer_failed: bool,
    max_fps: f32,
    frame_stats: FrameStats,
    window_state: WindowState,
    hyper_cube: HyperCube,
    game_overlay: GameOverlay,
    menu: Menu,
    console: Console,
    actions: ClientActions,
    game_actions: Arc<GameActions>,
    _commands: CommandOwner,
}
