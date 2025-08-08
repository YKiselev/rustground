use vulkanalia::{vk::{self, InstanceV1_0, KhrSurfaceExtension}, Instance};

use crate::error::{to_suitability, VkError};


#[derive(Copy, Clone, Debug)]
pub(crate) struct QueueFamilyIndices {
    pub graphics: u32,
    pub present: u32,
}

impl QueueFamilyIndices {
    pub fn get(
        instance: &Instance,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self, VkError> {
        let properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let mut present = None;
        for (index, _) in properties.iter().enumerate() {
            if unsafe {
                instance.get_physical_device_surface_support_khr(
                    physical_device,
                    index as u32,
                    surface,
                )?
            } {
                present = Some(index as u32);
                break;
            }
        }

        let graphics = properties
            .iter()
            .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|i| i as u32);

        if let (Some(graphics), Some(present)) = (graphics, present) {
            Ok(Self { graphics, present })
        } else {
            Err(to_suitability("Missing required queue families."))
        }
    }
}
