use std::{
    collections::HashSet,
    ffi::{CStr, c_void},
};

use log::{debug, error, info, trace, warn};
use vulkanalia::{
    vk::{
        self, DeviceV1_0, EntryV1_0, ExtDebugUtilsExtension, Framebuffer, Handle, HasBuilder, ImageView, InstanceV1_0, KhrSurfaceExtension, KhrSwapchainExtension, PhysicalDevice, Queue, RenderPass, SurfaceKHR
    }, window, Device, Entry, Instance, Version
};
use winit::window::Window;

use crate::{error::{to_generic, to_suitability, VkError}, pipeline::{create_render_pass, Pipeline}};

pub(crate) const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

pub(crate) const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

pub(crate) const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

#[derive(Debug)]
pub struct VkInstance {
    instance: Instance,
    messenger: vk::DebugUtilsMessengerEXT,
    surface: SurfaceKHR,
    physical_device: PhysicalDevice,
    device: Device,
    graphics_queue: Queue,
    present_queue: Queue,
    swapchain: Swapchain,
    swapchain_image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    pipeline: Pipeline,
    framebuffers: Vec<vk::Framebuffer>,
}

#[derive(Debug)]
struct Swapchain {
    format: vk::Format,
    extent: vk::Extent2D,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
}

impl VkInstance {
    pub fn new(window: &Window, entry: &Entry) -> Result<Self, VkError> {
        let (instance, messenger) = create_instance(window, entry)?;
        let surface = create_surface(&instance, window)?;
        let physical_device = pick_physical_device(&instance, surface)?;
        let (device, graphics_queue, present_queue) =
            create_logical_device(&entry, &instance, surface, physical_device)?;
        let swapchain = Swapchain::new(window, &instance, &device, surface, physical_device)?;
        let swapchain_image_views = swapchain.create_swapchain_image_views(&device)?;
        let render_pass = create_render_pass(&instance, &device, swapchain.format)?;
        let pipeline = Pipeline::new(&device, swapchain.extent)?;
        let framebuffers = swapchain.create_framebuffers(&swapchain_image_views, &device, render_pass)?;
        Ok(Self {
            instance,
            messenger,
            surface,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain,
            swapchain_image_views,
            render_pass,
            pipeline,
            framebuffers,
        })
    }

    pub fn destroy(&self) {
        unsafe {
            self.device
                .destroy_pipeline_layout(self.pipeline.layout, None);
            self.device.destroy_render_pass(self.render_pass, None);
            self.framebuffers
                .iter()
                .for_each(|f| self.device.destroy_framebuffer(*f, None));
            self.swapchain_image_views
                .iter()
                .for_each(|v| self.device.destroy_image_view(*v, None));
            self.device
                .destroy_swapchain_khr(self.swapchain.swapchain, None);
            self.device.destroy_device(None);
            self.instance.destroy_surface_khr(self.surface, None);

            if VALIDATION_ENABLED {
                self.instance
                    .destroy_debug_utils_messenger_ext(self.messenger, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}

fn create_instance(
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
    let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();

    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warn!("({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        debug!("({:?}) {}", type_, message);
    } else {
        trace!("({:?}) {}", type_, message);
    }

    vk::FALSE
}

fn create_surface(instance: &Instance, window: &Window) -> Result<SurfaceKHR, VkError> {
    Ok(unsafe { window::create_surface(instance, window, window) }?)
}

fn pick_physical_device(
    instance: &Instance,
    surface: SurfaceKHR,
) -> Result<PhysicalDevice, VkError> {
    for physical_device in unsafe { instance.enumerate_physical_devices() }? {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };

        if let Err(error) = unsafe { check_physical_device(instance, surface, physical_device) } {
            warn!(
                "Skipping physical device (`{}`): {}",
                properties.device_name, error
            );
        } else {
            info!("Selected physical device (`{}`).", properties.device_name);
            return Ok(physical_device);
        }
    }

    Err(to_generic("Failed to find suitable physical device."))
}

fn check_physical_device(
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

fn check_physical_device_extensions(
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

fn create_logical_device(
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

#[derive(Copy, Clone, Debug)]
struct QueueFamilyIndices {
    graphics: u32,
    present: u32,
}

impl QueueFamilyIndices {
    fn get(
        instance: &Instance,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self, VkError> {
        let properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let mut present = None;
        for (index, properties) in properties.iter().enumerate() {
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

#[derive(Clone, Debug)]
struct SwapchainSupport {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupport {
    fn get(
        instance: &Instance,
        surface: SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self, VkError> {
        unsafe {
            Ok(Self {
                capabilities: instance
                    .get_physical_device_surface_capabilities_khr(physical_device, surface)?,
                formats: instance
                    .get_physical_device_surface_formats_khr(physical_device, surface)?,
                present_modes: instance
                    .get_physical_device_surface_present_modes_khr(physical_device, surface)?,
            })
        }
    }
}

fn get_swapchain_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    formats
        .iter()
        .cloned()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or_else(|| formats[0])
}

fn get_swapchain_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    present_modes
        .iter()
        .cloned()
        .find(|m| *m == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO)
}

fn get_swapchain_extent(window: &Window, capabilities: vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D::builder()
            .width(window.inner_size().width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ))
            .height(window.inner_size().height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ))
            .build()
    }
}

impl Swapchain {
    fn new(
        window: &Window,
        instance: &Instance,
        device: &Device,
        surface: SurfaceKHR,
        physical_device: PhysicalDevice,
    ) -> Result<Self, VkError> {
        let indices = QueueFamilyIndices::get(instance, surface, physical_device)?;
        let support = SwapchainSupport::get(instance, surface, physical_device)?;

        let surface_format = get_swapchain_surface_format(&support.formats);
        let present_mode = get_swapchain_present_mode(&support.present_modes);
        let extent = get_swapchain_extent(window, support.capabilities);

        let mut image_count = support.capabilities.min_image_count + 1;
        if support.capabilities.max_image_count != 0
            && image_count > support.capabilities.max_image_count
        {
            image_count = support.capabilities.max_image_count;
        }
        let mut queue_family_indices = vec![];
        let image_sharing_mode = if indices.graphics != indices.present {
            queue_family_indices.push(indices.graphics);
            queue_family_indices.push(indices.present);
            vk::SharingMode::CONCURRENT
        } else {
            vk::SharingMode::EXCLUSIVE
        };

        let info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(image_sharing_mode)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(support.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null());

        let swapchain = unsafe { device.create_swapchain_khr(&info, None) }?;
        let images = unsafe { device.get_swapchain_images_khr(swapchain) }?;

        Ok(Self {
            format: surface_format.format,
            extent,
            swapchain,
            images,
        })
    }

    fn create_swapchain_image_views(&self, device: &Device) -> Result<Vec<ImageView>, VkError> {
        let image_views = self
            .images
            .iter()
            .map(|i| {
                let components = vk::ComponentMapping::builder()
                    .r(vk::ComponentSwizzle::IDENTITY)
                    .g(vk::ComponentSwizzle::IDENTITY)
                    .b(vk::ComponentSwizzle::IDENTITY)
                    .a(vk::ComponentSwizzle::IDENTITY);

                let subresource_range = vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1);

                let info = vk::ImageViewCreateInfo::builder()
                    .image(*i)
                    .view_type(vk::ImageViewType::_2D)
                    .format(self.format)
                    .components(components)
                    .subresource_range(subresource_range);

                unsafe { device.create_image_view(&info, None) }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(image_views)
    }

    fn create_framebuffers(
        &self,
        swapchain_image_views: &Vec<ImageView>,
        device: &Device,
        render_pass: RenderPass
    ) -> Result<Vec<Framebuffer>, VkError> {
        let framebuffers = swapchain_image_views
            .iter()
            .map(|i| {
                let attachments = &[*i];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(attachments)
                    .width(self.extent.width)
                    .height(self.extent.height)
                    .layers(1);

                unsafe { device.create_framebuffer(&create_info, None) }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(framebuffers)
    }
}
