use vulkanalia::{
    loader::{LibloadingLoader, LIBRARY}, vk::{self, DeviceV1_0, ExtDebugUtilsExtension, InstanceV1_0}, Device, Entry, Instance
};
use winit::window::Window;

use crate::{
    device::{create_logical_device, pick_physical_device},
    error::{VkError, to_generic},
    instance::{VALIDATION_ENABLED, create_instance},
};

#[derive(Debug, Default)]
pub(crate) struct VkData {
    pub(super) messenger: vk::DebugUtilsMessengerEXT,
    pub(super) physical_device: vk::PhysicalDevice,
    pub(super) graphics_queue: vk::Queue,
}

#[derive(Debug)]
pub struct VulkanRenderer {
    entry: Entry,
    instance: Instance,
    data: VkData,
    device: Device,
}

impl VulkanRenderer {
    pub fn new(window: &Window) -> Result<Self, VkError> {
        unsafe {
            let loader = LibloadingLoader::new(LIBRARY).map_err(to_generic)?;
            let entry = Entry::new(loader).map_err(to_generic)?;
            let mut data = VkData::default();
            let instance = create_instance(window, &entry, &mut data)?;
            pick_physical_device(&instance, &mut data)?;
            let device = create_logical_device(&entry, &instance, &mut data)?;
            Ok(Self {
                entry,
                instance,
                data,
                device,
            })
        }
    }

    pub fn render(&mut self) {}

    pub fn destroy(&mut self) {
        unsafe {
            self.device.destroy_device(None);

            if VALIDATION_ENABLED {
                self.instance
                    .destroy_debug_utils_messenger_ext(self.data.messenger, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}
