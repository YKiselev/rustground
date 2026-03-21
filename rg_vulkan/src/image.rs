use vulkanalia::vk::{DeviceMemory, Image};

pub struct VkImage {
    image: Image,
    memory: DeviceMemory,
}

impl VkImage {
    pub fn new(image: Image, memory: DeviceMemory) -> VkImage {
        Self { image, memory }
    }
}
