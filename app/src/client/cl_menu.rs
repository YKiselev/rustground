use rg_common::ui::{
    canvas::{Canvas, WrapMode},
    color::Color,
};
use rg_vulkan::renderer::VulkanCanvas;

pub(crate) struct Menu {
    visible: bool,
}

impl Menu {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub fn draw(&self, canvas: &mut VulkanCanvas) {
        if !self.visible {
            return;
        }

        canvas.set_color(Color::RED);
        canvas.set_wrap_mode(WrapMode::Word);
        canvas.draw_text(
            50,
            20,
            400,
            "This is menu",
        );
    }
}
