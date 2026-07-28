use std::rc::Rc;

use crate::{app_logger::AppLoggerBuffer, client::cl_ui_layer::UiLayer};

pub struct Console {
    app_log_buffer: Rc<AppLoggerBuffer>,
    height: f32,
}

impl Console {
    pub fn new(app_log_buffer: Rc<AppLoggerBuffer>) -> Self {
        Self {
            app_log_buffer,
            height: 0.0,
        }
    }
}

impl UiLayer for Console {
    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: &winit::event::DeviceEvent,
    ) -> bool {
        false
    }

    fn draw(&self, canvas: &mut rg_vulkan::renderer::VulkanCanvas) {}

    fn toggle(&mut self) {

    }

    fn is_visible(&self) -> bool {
        self.height > 0.0
    }
}
