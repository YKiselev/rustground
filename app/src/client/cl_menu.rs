use rg_common::ui::{
    canvas::{Canvas, WrapMode},
    color::Color,
};
use rg_vulkan::renderer::VulkanCanvas;

use crate::client::cl_ui_layer::UiLayer;

pub(crate) struct Menu {

}

impl Menu {
    pub fn new() -> Self {
        Self {  }
    }
}

impl UiLayer for Menu {
    fn device_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: &winit::event::DeviceEvent) -> bool {
        false
    }

    fn draw(&self, canvas: &mut VulkanCanvas) {
        canvas.set_color(Color::RED);
        canvas.set_wrap_mode(WrapMode::Word);
        canvas.draw_text(
            50,
            120,
            400,
            "This is menu",
        );
    }
}