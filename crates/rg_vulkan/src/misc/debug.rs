use ash::ext::debug_utils;
use ash::vk::{DebugUtilsMessengerEXT, Handle};
use ash::{Entry, Instance, vk};
use core::ffi::c_void;
use tracing::{debug, error, trace, warn};

pub(crate) struct DebugUtils {
    loader: debug_utils::Instance,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl DebugUtils {
    pub fn new(entry: &Entry, instance: &Instance) -> Self {
        let loader = ash::ext::debug_utils::Instance::new(entry, instance);
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(debug_callback));

        let messenger = unsafe {
            loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap_or(DebugUtilsMessengerEXT::null())
        };
        Self {
            loader: loader,
            messenger: messenger,
        }
    }

    pub fn destroy(&mut self) {
        if !self.messenger.is_null() {
            unsafe {
                self.loader
                    .destroy_debug_utils_messenger(self.messenger, None);
            }
        }
    }
}

extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let id_name = unsafe { data.message_id_name_as_c_str().unwrap_or(c"") };
    let message = unsafe { data.message_as_c_str().unwrap_or(c"") };

    match severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            trace!("({:?}, {:?}) {:?}", type_, id_name, message)
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            debug!("({:?}, {:?}) {:?}", type_, id_name, message)
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("({:?}, {:?}) {:?}", type_, id_name, message)
        }
        _ => {
            error!("({:?}, {:?}) {:?}", type_, id_name, message)
        }
    }

    vk::FALSE
}
