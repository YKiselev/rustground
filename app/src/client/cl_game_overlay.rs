use rg_common::ui::{canvas::{Canvas, WrapMode}, color::Color};

use crate::client::cl_ui_layer::UiLayer;

pub struct GameOverlay {}

impl GameOverlay {
    pub fn new() -> Self {
        Self {}
    }
}

impl UiLayer for GameOverlay {
    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _event: &winit::event::DeviceEvent,
    ) -> bool {
        false
    }

    fn draw(&self, canvas: &mut rg_vulkan::renderer::VulkanCanvas) {
        let w = canvas.width();
        let h = canvas.height();

        canvas.set_color(Color::RED);
        canvas.set_wrap_mode(WrapMode::Word);
        canvas.draw_text(
            (w/2) as i32,
            (h/2) as i32,
            w/2,
            "Hello, Vulkan user! THis is your in-game overlay.",
        );
    }
}
