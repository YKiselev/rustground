use std::{
    collections::HashSet,
    ffi::{CStr, CString},
    sync::Arc,
};

use ash::{
    ext::debug_utils,
    vk::{self},
};
use log::{debug, info, log_enabled, warn};
use raw_window_handle::HasDisplayHandle;
use rg_common::App;
use winit::window::Window;

use crate::{
    debug::DebugUtils,
    device::{VALIDATION_ENABLED, VALIDATION_LAYER, get_physical_device_id},
    error::VkError,
};

const ENGINE_VERSION: u32 = vk::make_api_version(0, 0, 3, 0);
///
/// Vulkan API version used
/// 
const API_VERSION: u32 = vk::API_VERSION_1_2;

pub(crate) fn create_instance(
    app: &Arc<App>,
    window: &Window,
    entry: &ash::Entry,
) -> Result<(ash::Instance, Option<DebugUtils>), VkError> {
    print_available_devices(entry)?;
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

    if log_enabled!(log::Level::Debug) {
        debug!("Supported layers:");
        for layer in available_layers.iter() {
            debug!("\t{:?}", layer);
        }
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
    debug!("Enumerating required extensions...");
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

    info!("Creating instance...");
    let instance = unsafe { entry.create_instance(&info, None) }?;

    let mut debug_utils = None;

    if VALIDATION_ENABLED {
        info!("Creating debug_utils...");
        debug_utils = Some(DebugUtils::new(entry, &instance));
    }

    Ok((instance, debug_utils))
}

fn print_available_devices(entry: &ash::Entry) -> Result<(), VkError> {
    let app_info = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);

    let create_info = vk::InstanceCreateInfo::default().application_info(&app_info);

    info!("Creating temporary instance...");
    let instance = unsafe { entry.create_instance(&create_info, None)? };

    info!("Enumerting physical device");
    let physical_devices = unsafe { instance.enumerate_physical_devices()? };

    let mut info = format!(
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

        let device_id = get_physical_device_id(&instance, device, &properties);

        info.push_str(&format!(
            "\n  #{} {} ({}), id = {:?}",
            index,
            device_name,
            device_type,
            device_id.to_string()
        ));
    }

    info!("{}", info);

    unsafe {
        instance.destroy_instance(None);
    }

    Ok(())
}
