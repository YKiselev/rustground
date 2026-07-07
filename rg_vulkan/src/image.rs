use ash::{Device, vk};

use crate::{error::VkError, memory::VkMemoryProperties};

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

pub(crate) fn create_image(
    device: &ash::Device,
    width: u32,
    height: u32,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    properties: vk::MemoryPropertyFlags,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
) -> Result<(vk::Image, vk::DeviceMemory), VkError> {
    let info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .format(format)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(vk::SampleCountFlags::TYPE_1);

    let image = unsafe { device.create_image(&info, None) }?;

    // Memory

    let requirements = unsafe { device.get_image_memory_requirements(image) };

    let info = vk::MemoryAllocateInfo::default()
        .allocation_size(requirements.size)
        .memory_type_index(memory_properties.get_memory_type_index(properties, requirements)?);

    let image_memory = unsafe { device.allocate_memory(&info, None) }?;

    unsafe { device.bind_image_memory(image, image_memory, 0) }?;

    Ok((image, image_memory))
}
