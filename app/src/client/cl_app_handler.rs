use rg_common::save_config;
use rg_vulkan::renderer::VulkanRenderer;
use tracing::{error, info, warn};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, MouseScrollDelta, StartCause, WindowEvent},
    event_loop::{self, ActiveEventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowId,
};

use crate::{
    application::trigger_exit,
    client::{BoolFlag, Client, ClientEvent, cl_context::ClientContext, cl_ui_layer::UiLayer},
};

impl ApplicationHandler<ClientEvent> for Client {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        let _ = (event_loop, cause);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let _ = self.renderer.take();
        match VulkanRenderer::new(&self.app, event_loop) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(e) => error!("Unable to create Vulkan renderer: {:?}", e),
        }
        event_loop.listen_device_events(winit::event_loop::DeviceEvents::Always);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ClientEvent) {
        let _ = (event_loop, event);
        match event {
            ClientEvent::Exiting => {
                event_loop.exit();
            }
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if !self.window_state.focused {
            return;
        }

        let (_, _) = (event_loop, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;

        match self.app.vars.to_toml() {
            Ok(toml) => {
                save_config("config.toml", &self.app.files, toml);
            }
            Err(e) => {
                warn!("Unable to export vars to toml: {:?}", e);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Window close requested");
                trigger_exit();
            }
            WindowEvent::Resized(_) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.mark_resized();
                }
            }
            WindowEvent::Focused(focused) => {
                self.window_state.focused = focused;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                info!("Window={window_id:?} changed scale to {scale_factor}");
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.window_state.modifiers = modifiers.state();
            }
            WindowEvent::RedrawRequested => {
                if !event_loop.exiting() {
                    let ctx = ClientContext::new();
                    self.update(event_loop, &ctx);
                }
            }
            _ => (),
        }

        let action = self.actions.get_from_window_event(&event);
        let skip_ui = self.shared_state.toggle_console.is_set()
            || self.shared_state.toggle_menu.is_set()
            || action.map_or(false, |a| a.starts_with("toggle"));
        if !skip_ui {
            if self.menu.window_event(&event, self.window_state.modifiers) {
                return;
            }

            if self
                .console
                .window_event(&event, self.window_state.modifiers)
            {
                return;
            }
        }

        if let Some(action) = action {
            if let Err(e) = self.app.execute(action) {
                warn!("{}", e);
            }
        }
    }
}
