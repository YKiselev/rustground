use rg_vulkan::renderer::VulkanCanvas;
use winit::{event::DeviceEvent, event_loop::ActiveEventLoop};

pub trait UiLayer {
    fn device_event(&mut self, event_loop: &ActiveEventLoop, event: &DeviceEvent) -> bool;

    fn draw(&self, canvas: &mut VulkanCanvas);
}
