use std::{
    collections::HashSet,
    ffi::{CStr, CString},
    sync::Arc,
};

use ash::{
    ext::debug_utils,
    vk::{self, PhysicalDevice},
};
use log::{info, warn};
use raw_window_handle::HasDisplayHandle;
use rg_common::App;
use uuid::Uuid;
use winit::window::Window;

use crate::{
    debug::DebugUtils,
    device::{VALIDATION_ENABLED, VALIDATION_LAYER},
    error::VkError,
};

const ENGINE_VERSION: u32 = vk::make_api_version(0, 1, 0, 0);
const API_VERSION: u32 = vk::make_api_version(0, 1, 0, 0);

pub(crate) fn create_instance(
    app: &Arc<App>,
    window: &Window,
    entry: &ash::Entry,
) -> Result<(ash::Instance, Option<DebugUtils>), VkError> {
    choose_best_physical_device(entry)?;
    let name = CString::new(app.name.as_str()).expect("App name contains null!");
    let app_version = vk::make_api_version(0, 1, 0, 0);
    let application_info = vk::ApplicationInfo {
        p_application_name: name.as_ptr(),
        application_version: app_version,
        p_engine_name: name.as_ptr(),
        engine_version: ENGINE_VERSION,
        api_version: API_VERSION,
        ..Default::default()
    };

    let available_layers = unsafe { entry.enumerate_instance_layer_properties() }?
        .iter()
        .map(|l| unsafe { CStr::from_ptr(l.layer_name.as_ptr()) }.to_owned())
        .collect::<HashSet<CString>>();

    info!("Supported layers:");
    for layer in available_layers.iter() {
        info!("\t{:?}", layer);
    }

    let validation_available = available_layers.contains(VALIDATION_LAYER);
    if VALIDATION_ENABLED && !validation_available {
        warn!("Validation layer unavailable!");
    }

    let layers = if VALIDATION_ENABLED && validation_available {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        Vec::new()
    };

    let display_handle = window.display_handle()?.as_raw();
    let mut extensions = ash_window::enumerate_required_extensions(display_handle)?.to_vec();

    if VALIDATION_ENABLED {
        extensions.push(debug_utils::NAME.as_ptr());
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        extensions.push(ash::khr::portability_enumeration::NAME.as_ptr());
        // Enabling this extension is a requirement when using `VK_KHR_portability_subset`
        extensions.push(ash::khr::get_physical_device_properties2::NAME.as_ptr());
    }

    // Required by Vulkan SDK on macOS since 1.3.216.
    let flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
        vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
    } else {
        vk::InstanceCreateFlags::empty()
    };

    let info = vk::InstanceCreateInfo::default()
        .application_info(&application_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .flags(flags);

    let instance = unsafe { entry.create_instance(&info, None) }?;

    let mut debug_utils = None;

    if VALIDATION_ENABLED {
        debug_utils = Some(DebugUtils::new(entry, &instance));
    }

    Ok((instance, debug_utils))
}

fn choose_best_physical_device(entry: &ash::Entry) -> Result<(), VkError> {
    let app_info = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);

    let create_info = vk::InstanceCreateInfo::default().application_info(&app_info);

    let instance = unsafe { entry.create_instance(&create_info, None)? };

    let physical_devices = unsafe { instance.enumerate_physical_devices()? };

    println!(
        "Found {} GPU(s) with Vulkan support:",
        physical_devices.len()
    );

    for (index, &device) in physical_devices.iter().enumerate() {
        let properties = unsafe { instance.get_physical_device_properties(device) };
        let device_name =
            unsafe { CStr::from_ptr(properties.device_name.as_ptr()).to_string_lossy() };

        let device_type = match properties.device_type {
            vk::PhysicalDeviceType::INTEGRATED_GPU => "Integrated GPU (iGPU)",
            vk::PhysicalDeviceType::DISCRETE_GPU => "Discrete GPU (dGPU)",
            vk::PhysicalDeviceType::VIRTUAL_GPU => "Virtual GPU",
            vk::PhysicalDeviceType::CPU => "CPU Fallback",
            _ => "Unknown Device Type",
        };

        let mut id_properties = vk::PhysicalDeviceIDProperties::default();
        let mut properties2 =
            vk::PhysicalDeviceProperties2::default().push_next(&mut id_properties);

        unsafe { instance.get_physical_device_properties2(device, &mut properties2) };

        let gpu_uuid: Uuid = Uuid::from_bytes(id_properties.device_uuid);
        let uuid_string = if gpu_uuid.is_nil() {
            format!("{}:{}", properties.vendor_id, properties.device_id)
        } else {
            gpu_uuid.to_string()
        };

        println!(
            "  [{}] {} ({}), UUID={}",
            index, device_name, device_type, uuid_string
        );
    }

    unsafe {
        instance.destroy_instance(None);
    }

    Ok(())
}
