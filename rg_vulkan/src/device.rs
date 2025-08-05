use log::{info, warn};
use vulkanalia::{vk::{self, DeviceV1_0, HasBuilder, InstanceV1_0}, Device, Entry, Instance};

use crate::{error::{to_generic, to_suitability, VkError}, instance::{PORTABILITY_MACOS_VERSION, VALIDATION_ENABLED, VALIDATION_LAYER}, renderer::VkData};

pub(super) unsafe fn pick_physical_device(instance: &Instance, data: &mut VkData) -> Result<(), VkError> {
    for physical_device in instance.enumerate_physical_devices()? {
        let properties = instance.get_physical_device_properties(physical_device);

        if let Err(error) = check_physical_device(instance, data, physical_device) {
            warn!("Skipping physical device (`{}`): {}", properties.device_name, error);
        } else {
            info!("Selected physical device (`{}`).", properties.device_name);
            data.physical_device = physical_device;
            return Ok(());
        }
    }

    Err(to_generic("Failed to find suitable physical device."))
}

unsafe fn check_physical_device(
    instance: &Instance,
    data: &VkData,
    physical_device: vk::PhysicalDevice,
) -> Result<(), VkError> {
    QueueFamilyIndices::get(instance, data, physical_device)?;
    Ok(())
}

pub(super) unsafe fn create_logical_device(entry: &Entry, instance: &Instance, data: &mut VkData) -> Result<Device, VkError> {
    // Queue Create Infos

    let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;

    let queue_priorities = &[1.0];
    let queue_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(indices.graphics)
        .queue_priorities(queue_priorities);

    // Layers

    let layers = if VALIDATION_ENABLED {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        vec![]
    };

    // Extensions

    let mut extensions = vec![];

    // Required by Vulkan SDK on macOS since 1.3.216.
    if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
        extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
    }

    // Features

    let features = vk::PhysicalDeviceFeatures::builder();

    // Create

    let queue_infos = &[queue_info];
    let info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(queue_infos)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .enabled_features(&features);

    let device = instance.create_device(data.physical_device, &info, None)?;

    // Queues

    data.graphics_queue = device.get_device_queue(indices.graphics, 0);

    Ok(device)
}


#[derive(Copy, Clone, Debug)]
struct QueueFamilyIndices {
    graphics: u32,
}


impl QueueFamilyIndices {
    unsafe fn get(instance: &Instance, data: &VkData, physical_device: vk::PhysicalDevice) -> Result<Self, VkError> {
        let properties = instance.get_physical_device_queue_family_properties(physical_device);

        let graphics = properties
            .iter()
            .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|i| i as u32);

        if let Some(graphics) = graphics {
            Ok(Self { graphics })
        } else {
            Err(to_suitability("Missing required queue families."))
        }
    }
}