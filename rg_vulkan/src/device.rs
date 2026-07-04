use std::{collections::HashSet, fmt, str::FromStr};

use ash::{
    Device, Instance,
    vk::{self, PhysicalDevice, PhysicalDeviceProperties, Queue},
};
use log::{debug, log_enabled};
use std::ffi::CStr;
use uuid::Uuid;

use crate::{
    error::{VkError, to_suitability},
    instance::DEVICE_EXTENSIONS,
    queue_family::QueueFamilyIndices,
    surface::VkSurface,
    swapchain::swapchain::SwapchainSupport,
};

pub(crate) const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

pub(crate) const VALIDATION_LAYER: &CStr = c"VK_LAYER_KHRONOS_validation";

//pub(crate) const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

///
/// Device id
///
#[derive(Debug, PartialEq)]
pub(crate) enum DeviceId {
    Uuid(Uuid),
    VendorAndDeviceId(u32, u32),
}

impl DeviceId {
    pub fn parse<S>(value: S) -> Option<Self>
    where
        S: AsRef<str>,
    {
        try_parse_device_id(value)
    }
}

impl Default for DeviceId {
    fn default() -> Self {
        DeviceId::Uuid(Uuid::nil())
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceId::Uuid(uuid) => write!(f, "{}", uuid),
            DeviceId::VendorAndDeviceId(vendor_id, device_id) => {
                write!(f, "{}:{}", vendor_id, device_id)
            }
        }
    }
}

///
///
///
pub(crate) fn pick_physical_device(
    instance: &Instance,
    surface: &VkSurface,
    device_id: &Option<DeviceId>,
) -> Result<(DeviceId, PhysicalDevice), VkError> {
    let physical_devices = enumerate_physical_devices(instance)?;

    match device_id {
        Some(id) => {
            if let Some((id, dev)) =
                find_physical_device(instance, surface, &physical_devices, |dev, props| {
                    get_physical_device_id(instance, dev, props) == *id
                })
            {
                return Ok((id, dev));
            }
        }
        None => {}
    }
    find_physical_device(instance, surface, &physical_devices, |_, _| true).ok_or(
        VkError::SuitabilityError("No suitable physical device found!"),
    )
}

fn enumerate_physical_devices(
    instance: &Instance,
) -> Result<Vec<(PhysicalDevice, PhysicalDeviceProperties)>, VkError> {
    let result = unsafe {
        instance
            .enumerate_physical_devices()?
            .iter()
            .map(|&d| {
                let props = instance.get_physical_device_properties(d);
                (d, props)
            })
            .collect()
    };
    Ok(result)
}

fn find_physical_device<F>(
    instance: &Instance,
    surface: &VkSurface,
    devices: &Vec<(PhysicalDevice, PhysicalDeviceProperties)>,
    predicate: F,
) -> Option<(DeviceId, PhysicalDevice)>
where
    F: Fn(PhysicalDevice, &PhysicalDeviceProperties) -> bool,
{
    for &(device, properties) in devices {
        if let Err(error) = check_physical_device(instance, surface, device) {
            if log_enabled!(log::Level::Debug) {
                let device_name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) };
                debug!("Skipping (`{:?}`): {}", device_name, error);
            }
        } else if predicate(device, &properties) {
            let id = get_physical_device_id(instance, device, &properties);
            return Some((id, device));
        }
    }
    None
}

pub(crate) fn get_physical_device_id(
    instance: &Instance,
    device: PhysicalDevice,
    properties: &PhysicalDeviceProperties,
) -> DeviceId {
    let mut id_properties = vk::PhysicalDeviceIDProperties::default();
    let mut properties2 = vk::PhysicalDeviceProperties2::default().push_next(&mut id_properties);

    unsafe { instance.get_physical_device_properties2(device, &mut properties2) };

    let uuid: Uuid = Uuid::from_bytes(id_properties.device_uuid);
    if uuid.is_nil() {
        DeviceId::VendorAndDeviceId(properties.vendor_id, properties.device_id)
    } else {
        DeviceId::Uuid(uuid)
    }
}

fn try_parse_device_id<S>(value: S) -> Option<DeviceId>
where
    S: AsRef<str>,
{
    if let Ok(uuid) = Uuid::from_str(value.as_ref()) {
        return Some(DeviceId::Uuid(uuid));
    } else {
        let parts: Vec<&str> = value.as_ref().split(":").collect();
        if parts.len() == 2 {
            let v_id = parts.get(0).and_then(|v| v.parse::<u32>().ok());
            let d_id = parts.get(1).and_then(|v| v.parse::<u32>().ok());
            if let (Some(vendor_id), Some(device_id)) = (v_id, d_id) {
                return Some(DeviceId::VendorAndDeviceId(vendor_id, device_id));
            }
        }
    }
    None
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
        .map(|e| unsafe { CStr::from_ptr(e.extension_name.as_ptr()).to_owned() })
        .collect::<HashSet<_>>();
    if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(*e)) {
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
