use vulkanalia::{
    Device, Instance,
    vk::{self, DeviceV1_0, Format, HasBuilder, Image, ImageView, KhrSurfaceExtension, SurfaceKHR},
};
use winit::window::Window;

use crate::error::VkError;

#[derive(Debug, Default)]
pub(crate) struct Swapchain {
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub views: Vec<vk::ImageView>,
    pub render_pass: vk::RenderPass,
}

#[derive(Clone, Debug)]
pub(crate) struct SwapchainSupport {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupport {
    pub fn get(
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

pub(crate) fn get_swapchain_surface_format(
    formats: &[vk::SurfaceFormatKHR],
) -> vk::SurfaceFormatKHR {
    formats
        .iter()
        .cloned()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or_else(|| formats[0])
}

pub(crate) fn get_swapchain_present_mode(
    present_modes: &[vk::PresentModeKHR],
) -> vk::PresentModeKHR {
    present_modes
        .iter()
        .cloned()
        .find(|m| *m == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO)
}

pub(crate) fn get_swapchain_extent(
    window: &Window,
    capabilities: vk::SurfaceCapabilitiesKHR,
) -> vk::Extent2D {
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

pub fn create_swapchain_image_views(
    images: &Vec<Image>,
    format: Format,
    device: &Device,
) -> Result<Vec<ImageView>, VkError> {
    let image_views = images
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
                .format(format)
                .components(components)
                .subresource_range(subresource_range);

            unsafe { device.create_image_view(&info, None) }
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(image_views)
}
