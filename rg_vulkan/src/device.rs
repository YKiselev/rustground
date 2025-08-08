use std::collections::HashSet;

use log::{info, warn};
use vulkanalia::{
    vk::{self, DeviceV1_0, HasBuilder, InstanceV1_0, PhysicalDevice, Queue, SurfaceKHR}, Device, Entry, Instance, Version
};

use crate::{
    error::{VkError, to_generic, to_suitability},
    instance::DEVICE_EXTENSIONS,
    queue_family::QueueFamilyIndices,
    swapchain::SwapchainSupport,
};

pub(crate) const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

pub(crate) const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

pub(crate) const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

pub(crate) fn pick_physical_device(
    instance: &Instance,
    surface: SurfaceKHR,
) -> Result<PhysicalDevice, VkError> {
    for physical_device in unsafe { instance.enumerate_physical_devices() }? {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };

        if let Err(error) = check_physical_device(instance, surface, physical_device) {
            warn!(
                "Skipping (`{}`): {}",
                properties.device_name, error
            );
        } else {
            info!("Selected: `{}`, ({:?}).", properties.device_name, properties.device_type);
            return Ok(physical_device);
        }
    }

    Err(to_generic("Failed to find suitable physical device."))
}

pub(crate) fn check_physical_device(
    instance: &Instance,
    surface: SurfaceKHR,
    physical_device: PhysicalDevice,
) -> Result<(), VkError> {
    QueueFamilyIndices::get(instance, surface, physical_device)?;
    check_physical_device_extensions(instance, physical_device)?;
    let support = SwapchainSupport::get(instance, surface, physical_device)?;
    if support.formats.is_empty() || support.present_modes.is_empty() {
        return Err(to_suitability("Insufficient swapchain support."));
    }
    Ok(())
}

pub(crate) fn check_physical_device_extensions(
    instance: &Instance,
    physical_device: PhysicalDevice,
) -> Result<(), VkError> {
    let extensions =
        unsafe { instance.enumerate_device_extension_properties(physical_device, None) }?
            .iter()
            .map(|e| e.extension_name)
            .collect::<HashSet<_>>();
    if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
        Ok(())
    } else {
        Err(to_suitability("Missing required device extensions."))
    }
}

pub(crate) fn create_logical_device(
    entry: &Entry,
    instance: &Instance,
    surface: SurfaceKHR,
    physical_device: PhysicalDevice,
) -> Result<(Device, Queue, Queue), VkError> {
    let indices = QueueFamilyIndices::get(instance, surface, physical_device)?;

    let mut unique_indices = HashSet::new();
    unique_indices.insert(indices.graphics);
    unique_indices.insert(indices.present);

    let queue_priorities = &[1.0];
    let queue_infos = unique_indices
        .iter()
        .map(|i| {
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(*i)
                .queue_priorities(queue_priorities)
        })
        .collect::<Vec<_>>();

    // Layers

    let layers = if VALIDATION_ENABLED {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        vec![]
    };

    // Extensions
    let mut extensions = DEVICE_EXTENSIONS
        .iter()
        .map(|n| n.as_ptr())
        .collect::<Vec<_>>();

    // Required by Vulkan SDK on macOS since 1.3.216.
    if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
        extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
    }

    // Features
    let features = vk::PhysicalDeviceFeatures::builder();

    // Create
    let info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .enabled_features(&features);

    let device = unsafe { instance.create_device(physical_device, &info, None) }?;

    // Queues
    let graphics_queue = unsafe { device.get_device_queue(indices.graphics, 0) };
    let present_queue = unsafe { device.get_device_queue(indices.present, 0) };

    Ok((device, graphics_queue, present_queue))
}
