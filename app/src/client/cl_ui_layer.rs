use rg_vulkan::renderer::VulkanCanvas;
use winit::{event::DeviceEvent, event_loop::ActiveEventLoop};

pub trait UiLayer {
    ///
    /// Process device event.
    /// Returns true if event is consumed and should not be passed further.
    /// 
    fn device_event(&mut self, event_loop: &ActiveEventLoop, event: &DeviceEvent) -> bool {
        let _ = (event, event_loop);
        false
    }

    fn draw(&self, canvas: &mut VulkanCanvas);

    ///
    /// Toggles layer visibility.
    /// Returns 0 if there is no sublayers left (for e.g. menu)
    /// 
    fn toggle(&self) -> usize {
        0
    }
}
