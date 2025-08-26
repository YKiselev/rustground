use std::{
    collections::HashSet,
    ffi::{CStr, c_void},
};

use log::{debug, error, info, trace, warn};
use vulkanalia::{
    Entry, Instance,
    vk::{self, EntryV1_0, ExtDebugUtilsExtension, HasBuilder},
    window,
};
use winit::window::Window;

use crate::{
    device::{PORTABILITY_MACOS_VERSION, VALIDATION_ENABLED, VALIDATION_LAYER},
    error::VkError,
};

pub(crate) fn create_instance(
    window: &Window,
    entry: &Entry,
) -> Result<(Instance, vk::DebugUtilsMessengerEXT), VkError> {
    let application_info = vk::ApplicationInfo::builder()
        .application_name(b"Rust Ground\0")
        .application_version(vk::make_version(1, 0, 0))
        .engine_name(b"RustGround\0")
        .engine_version(vk::make_version(1, 0, 0))
        .api_version(vk::make_version(1, 0, 0));

    let available_layers = unsafe { entry.enumerate_instance_layer_properties() }?
        .iter()
        .map(|l| l.layer_name)
        .collect::<HashSet<_>>();

    info!("Supported layers:");
    for layer in available_layers.iter() {
        info!("\t{:?}", layer.as_cstr());
    }

    let validation_available = available_layers.contains(&VALIDATION_LAYER);
    if VALIDATION_ENABLED && !validation_available {
        warn!("Validation layer unavailable!");
    }

    let layers = if VALIDATION_ENABLED && validation_available {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        Vec::new()
    };

    let mut extensions = window::get_required_instance_extensions(window)
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<_>>();

    if VALIDATION_ENABLED {
        extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
    }

    // Required by Vulkan SDK on macOS since 1.3.216.
    let flags = if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
        info!("Enabling extensions for macOS portability.");
        extensions.push(
            vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION
                .name
                .as_ptr(),
        );
        extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
        vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
    } else {
        vk::InstanceCreateFlags::empty()
    };

    let info = vk::InstanceCreateInfo::builder()
        .application_info(&application_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .flags(flags);

    let instance = unsafe { entry.create_instance(&info, None) }?;
    let mut messenger = Default::default();

    if VALIDATION_ENABLED {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .user_callback(Some(debug_callback));

        messenger = unsafe { instance.create_debug_utils_messenger_ext(&debug_info, None) }?;
    }

    Ok((instance, messenger))
}

extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let id_name = unsafe { CStr::from_ptr(data.message_id_name).to_string_lossy() };
    let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();

    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("({:?}, {:?}) {}", type_, id_name, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warn!("({:?}, {:?}) {}", type_, id_name, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        debug!("({:?}, {:?}) {}", type_, id_name, message);
    } else {
        trace!("({:?}, {:?}) {}", type_, id_name, message);
    }

    vk::FALSE
}
