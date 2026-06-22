use std::collections::HashSet;

use ash::{
    Device, Instance,
    vk::{self, PhysicalDevice, Queue},
};
use log::{info, warn};
use std::ffi::CStr;

use crate::{
    error::{VkError, to_generic, to_suitability},
    instance::DEVICE_EXTENSIONS,
    queue_family::QueueFamilyIndices,
    surface::VkSurface,
    swapchain::SwapchainSupport,
};

pub(crate) const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

pub(crate) const VALIDATION_LAYER: &CStr = c"VK_LAYER_KHRONOS_validation";

//pub(crate) const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

pub(crate) fn pick_physical_device(
    instance: &Instance,
    surface: &VkSurface,
) -> Result<PhysicalDevice, VkError> {
    for physical_device in unsafe { instance.enumerate_physical_devices() }? {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let device_name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) };

        if let Err(error) = check_physical_device(instance, surface, physical_device) {
            warn!("Skipping (`{:?}`): {}", device_name, error);
        } else {
            info!(
                "Selected: `{:?}`, ({:?}).",
                device_name, properties.device_type
            );
            return Ok(physical_device);
        }
    }

    Err(to_generic("Failed to find suitable physical device."))
}

pub(crate) fn check_physical_device(
    instance: &Instance,
    surface: &VkSurface,
    physical_device: PhysicalDevice,
) -> Result<(), VkError> {
    QueueFamilyIndices::get(instance, surface, physical_device)?;
    check_physical_device_extensions(instance, physical_device)?;
    let support = SwapchainSupport::get(surface, physical_device)?;
    if support.formats.is_empty() || support.present_modes.is_empty() {
        return Err(to_suitability("Insufficient swapchain support."));
    }
    Ok(())
}

pub(crate) fn check_physical_device_extensions(
    instance: &Instance,
    physical_device: PhysicalDevice,
) -> Result<(), VkError> {
    let extensions = unsafe { instance.enumerate_device_extension_properties(physical_device) }?
        .iter()
        .map(|e| unsafe { CStr::from_ptr(e.extension_name.as_ptr()) })
        .collect::<HashSet<_>>();
    if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
        Ok(())
    } else {
        Err(to_suitability("Missing required device extensions."))
    }
}

pub(crate) fn create_logical_device(
    instance: &Instance,
    surface: &VkSurface,
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
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(*i)
                .queue_priorities(queue_priorities)
        })
        .collect::<Vec<_>>();

    // Layers
    // let layers = if VALIDATION_ENABLED {
    //     vec![VALIDATION_LAYER.as_ptr()]
    // } else {
    //     vec![]
    // };

    // Extensions
    let mut extensions = DEVICE_EXTENSIONS
        .iter()
        .map(|n| n.as_ptr())
        .collect::<Vec<_>>();

    // Required by Vulkan SDK on macOS since 1.3.216.
    // if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
    //     extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
    // }

    // Features
    let features = vk::PhysicalDeviceFeatures::default();

    // Create
    let info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_infos)
        //.enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .enabled_features(&features);

    let device = unsafe { instance.create_device(physical_device, &info, None) }?;

    // Queues
    let graphics_queue = unsafe { device.get_device_queue(indices.graphics, 0) };
    let present_queue = unsafe { device.get_device_queue(indices.present, 0) };

    Ok((device, graphics_queue, present_queue))
}
