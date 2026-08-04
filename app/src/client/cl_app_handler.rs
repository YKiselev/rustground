use rg_common::save_config;
use rg_vulkan::renderer::VulkanRenderer;
use tracing::{error, info, warn};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, MouseScrollDelta, StartCause, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowId,
};

use crate::{
    application::trigger_exit,
    client::{Client, ClientEvent, cl_context::ClientContext, cl_ui_layer::UiLayer},
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
        if self.menu.device_event(event_loop, &event) {
            return;
        }

        if self.console.device_event(event_loop, &event) {
            return;
        }

        if self.game_overlay.device_event(event_loop, &event) {
            return;
        }

        if let Some(action) = self.actions.get_from_event(&event) {
            if let Err(e) = self.app.commands.execute(action) {
                warn!("{}", e);
            }
        }
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
        //self.state.destroy();
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
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    info!("Mouse wheel Line Delta: ({x},{y})");
                }
                MouseScrollDelta::PixelDelta(px) => {
                    info!("Mouse wheel Pixel Delta: ({},{})", px.x, px.y);
                }
            },
            WindowEvent::RedrawRequested => {
                if !event_loop.exiting() {
                    let ctx = ClientContext::new();
                    self.update(event_loop, &ctx);
                }
            }
            WindowEvent::KeyboardInput { ref event, .. } => {
                match event.physical_key {
                    PhysicalKey::Code(key_code) if event.state == ElementState::Released => {
                        match key_code {
                            //KeyCode::Escape => self.toggle_menu(),
                            //KeyCode::Backquote => self.toggle_console(),
                            KeyCode::Space => {
                                info!("fps: {:.2}", self.frame_stats.calc_fps());
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }

                if self.menu.keyboard_input(event, self.window_state.modifiers) {
                    return;
                }

                if self
                    .console
                    .keyboard_input(event, self.window_state.modifiers)
                {
                    return;
                }
            }
            _ => (),
        }
    }
}
