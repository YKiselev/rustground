use std::{
    collections::HashSet,
    ffi::{CStr, CString, c_void},
};

use ash::{Entry, Instance, ext::debug_utils, vk};
use log::{info, warn};
use raw_window_handle::HasDisplayHandle;
use winit::window::Window;

use crate::{
    debug::DebugUtils, device::{VALIDATION_ENABLED, VALIDATION_LAYER}, error::VkError,
};

const APP_NAME: &CStr = c"Rust Ground";

pub(crate) fn create_instance(
    window: &Window,
    entry: &Entry,
) -> Result<(Instance, Option<DebugUtils>), VkError> {
    let application_info = vk::ApplicationInfo {
        p_application_name: APP_NAME.as_ptr(),
        application_version: 0,
        p_engine_name: APP_NAME.as_ptr(),
        engine_version: 0,
        api_version: vk::make_api_version(0, 1, 0, 0),
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