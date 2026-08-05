use rg_vulkan::renderer::VulkanCanvas;
use winit::{event::WindowEvent, keyboard::ModifiersState};

pub trait UiLayer {
    fn window_event(&mut self, event: &WindowEvent, modifiers: ModifiersState) -> bool {
        let (_,_) = (event, modifiers);
        false
    }

    fn draw(&mut self, canvas: &mut VulkanCanvas);

    fn is_visible(&self) -> bool;

    ///
    /// Toggles layer visibility.
    ///
    fn toggle(&mut self) {}
}
