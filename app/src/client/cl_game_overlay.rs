use rg_common::ui::{canvas::{Canvas, WrapMode}, color::Color};

use crate::client::cl_ui_layer::UiLayer;

pub struct GameOverlay {}

impl GameOverlay {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self) {
        
    }
}

impl UiLayer for GameOverlay {

    fn draw(&mut self, canvas: &mut rg_vulkan::renderer::VulkanCanvas) {
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

    fn is_visible(&self) -> bool {
        false
    }
}
