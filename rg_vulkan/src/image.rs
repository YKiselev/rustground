use ash::vk;


pub struct VkImage {
    image: vk::Image,
    memory: vk::DeviceMemory,
}

impl VkImage {
    pub fn new(image: vk::Image, memory: vk::DeviceMemory) -> VkImage {
        Self { image, memory }
    }
}
