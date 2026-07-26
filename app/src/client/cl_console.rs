use crate::client::cl_ui_layer::UiLayer;


pub struct Console {

}

impl Console {
    pub fn new() -> Self {
        Self {  }
    }
}

impl UiLayer for Console {
    fn device_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: &winit::event::DeviceEvent) -> bool {
        false
    }

    fn draw(&self, canvas: &mut rg_vulkan::renderer::VulkanCanvas) {
        
    }
}