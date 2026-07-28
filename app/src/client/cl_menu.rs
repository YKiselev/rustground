use rg_common::ui::{
    canvas::{Canvas, WrapMode},
    color::Color,
};
use rg_vulkan::renderer::VulkanCanvas;

use crate::client::cl_ui_layer::UiLayer;

pub(crate) struct Menu {
    visible: bool,
}

impl Menu {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub fn update(&mut self) {

    }
}

impl UiLayer for Menu {
    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: &winit::event::DeviceEvent,
    ) -> bool {
        false
    }

    fn draw(&mut self, canvas: &mut VulkanCanvas) {
        canvas.set_color(Color::RED);
        canvas.set_wrap_mode(WrapMode::Word);
        canvas.draw_text(50, 120, 400, "This is menu");
    }

    fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
}
