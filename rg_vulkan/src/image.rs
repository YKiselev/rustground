use ash::{Device, vk};

#[derive(Default)]
pub struct VkImage {
    pub image: vk::Image,
    memory: vk::DeviceMemory,
    pub view: vk::ImageView,
}

impl VkImage {
    pub fn new(image: vk::Image, memory: vk::DeviceMemory, view: vk::ImageView) -> VkImage {
        Self {
            image,
            memory,
            view,
        }
    }

    pub fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
        }
    }
}
