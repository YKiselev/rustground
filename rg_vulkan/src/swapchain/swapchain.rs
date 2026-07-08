use ash::{
    khr,
    vk::{self, SurfaceFormatKHR},
};
use log::info;
use winit::window::Window;

use crate::{
    error::VkError, image::VkImage, image::create_image, queue_family::QueueFamilyIndices,
    surface::VkSurface, swapchain::frames_in_flight::FramesInFlight,
};

///
/// Image objects
///
pub(crate) struct ImageObjects {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub depth_view: VkImage,
    pub framebuffer: vk::Framebuffer,
    render_finished: vk::Semaphore,
}

impl ImageObjects {
    fn new(
        device: &ash::Device,
        format: vk::Format,
        depth_format: vk::Format,
        image: vk::Image,
        pass: vk::RenderPass,
        extent: vk::Extent2D,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
    ) -> Result<Self, VkError> {
        let view = create_swapchain_image_view(image, format, device)?;
        let depth_view = create_depth_image(device, extent, depth_format, memory_properties)?;
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let attachments = [view, depth_view.view];
        let create_info = vk::FramebufferCreateInfo::default()
            .render_pass(pass)
            .attachments(&attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);
        let framebuffer = unsafe { device.create_framebuffer(&create_info, None) }?;
        let render_finished = unsafe { device.create_semaphore(&semaphore_info, None) }?;
        Ok(Self {
            image,
            view,
            depth_view,
            framebuffer,
            render_finished,
        })
    }

    fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_semaphore(self.render_finished, None);
            device.destroy_framebuffer(self.framebuffer, None);
            device.destroy_image_view(self.view, None);
            self.depth_view.destroy(device);
        }
    }
}

///
/// Swapchain
///
#[derive()]
pub(crate) struct Swapchain {
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<ImageObjects>,
    pub render_pass: vk::RenderPass,
    pub frames_in_flight: FramesInFlight,
}

impl Swapchain {
    pub fn new(
        instance: &ash::Instance,
        surface: &VkSurface,
        device: &ash::Device,
        swapchain_device: &khr::swapchain::Device,
        physical_device: vk::PhysicalDevice,
        window: &Window,
        depth_format: vk::Format,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        old_swapchain: vk::SwapchainKHR,
    ) -> Result<Swapchain, VkError> {
        let indices = QueueFamilyIndices::get(instance, surface, physical_device)?;
        let support = SwapchainSupport::get(surface, physical_device)?;
        let surface_format = get_swapchain_surface_format(&support.formats);
        let present_mode = find_best_swapchain_present_mode(&support.present_modes);
        let extent = support.get_swapchain_extent(window);
        let image_count = support.get_optimal_image_count();
        let mut queue_family_indices = vec![];
        let image_sharing_mode = if indices.graphics != indices.present {
            queue_family_indices.push(indices.graphics);
            queue_family_indices.push(indices.present);
            vk::SharingMode::CONCURRENT
        } else {
            vk::SharingMode::EXCLUSIVE
        };

        let info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.surface)
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
            .old_swapchain(old_swapchain);

        let swapchain = unsafe { swapchain_device.create_swapchain(&info, None) }?;
        let images = unsafe { swapchain_device.get_swapchain_images(swapchain) }?;
        let render_pass = create_render_pass(device, surface_format.format, depth_format)?;
        let images = images
            .into_iter()
            .map(|img| {
                ImageObjects::new(
                    device,
                    surface_format.format,
                    depth_format,
                    img,
                    render_pass,
                    extent,
                    memory_properties,
                )
            })
            .collect::<Result<Vec<ImageObjects>, VkError>>()?;
        let frames_in_flight = FramesInFlight::new(device, indices.graphics)?;
        Ok(Swapchain {
            format: surface_format.format,
            extent,
            swapchain,
            images,
            render_pass,
            frames_in_flight,
        })
    }

    /// Destroys this swapchain objects (except for vk::SwapchainKHR) and returns vk::SwapchainKHR for reuse
    /// Returned value should be destroyed afterwards!
    pub fn destroy(&mut self, device: &ash::Device) -> vk::SwapchainKHR {
        self.frames_in_flight.destroy(device);
        self.images.iter().for_each(|img| img.destroy(device));
        self.images.clear();

        unsafe {
            device.destroy_render_pass(self.render_pass, None);
        }
        std::mem::replace(&mut self.swapchain, vk::SwapchainKHR::null())
    }

    pub fn acquire_next_image(
        &self,
        device: &ash::Device,
        swapchain_device: &khr::swapchain::Device,
    ) -> Result<usize, VkError> {
        let frame = self.frames_in_flight.frame();
        let fences = [frame.in_flight_fence];
        unsafe {
            device.wait_for_fences(&fences, true, u64::MAX)?;
        };
        frame.reset_buffers(device)?;
        match unsafe {
            swapchain_device.acquire_next_image(
                self.swapchain,
                u64::MAX,
                frame.image_available,
                vk::Fence::null(),
            )
        } {
            Ok((image_index, suboptimal)) => {
                if suboptimal {
                    Err(VkError::SwapchainChanged)
                } else {
                    Ok(image_index as usize)
                }
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Err(VkError::SwapchainChanged),
            Err(e) => Err(e.into()),
        }
    }

    pub fn present(
        &self,
        window: &Window,
        device: &ash::Device,
        swapchain_device: &khr::swapchain::Device,
        graphics_queue: vk::Queue,
        present_queue: vk::Queue,
        image_index: usize,
    ) -> Result<bool, vk::Result> {
        let frame = self.frames_in_flight.frame();
        let fences = [frame.in_flight_fence];
        let wait_semaphores = [frame.image_available];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [frame.command_buffer];
        let signal_semaphores = [self.images[image_index].render_finished];
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            device.reset_fences(&fences)?;
            let infos = [submit_info];
            device.queue_submit(graphics_queue, &infos, frame.in_flight_fence)?;
        }

        window.pre_present_notify();

        let swapchains = [self.swapchain];
        let image_indices = [image_index as u32];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe { swapchain_device.queue_present(present_queue, &present_info) }
    }

    pub fn advance_frame_index(&mut self) {
        self.frames_in_flight.advance_frame_index();
    }
}

///
/// Swapchain support
///
#[derive(Clone, Debug)]
pub(crate) struct SwapchainSupport {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupport {
    pub fn get(surface: &VkSurface, physical_device: vk::PhysicalDevice) -> Result<Self, VkError> {
        Ok(Self {
            capabilities: surface.get_capabilities(physical_device)?,
            formats: surface.get_formats(physical_device)?,
            present_modes: surface.get_present_modes(physical_device)?,
        })
    }

    pub fn get_optimal_image_count(&self) -> u32 {
        let mut image_count = self.capabilities.min_image_count + 1;
        if self.capabilities.max_image_count != 0 && image_count > self.capabilities.max_image_count
        {
            image_count = self.capabilities.max_image_count;
        }
        image_count
    }

    fn get_swapchain_extent(&self, window: &Window) -> vk::Extent2D {
        get_swapchain_extent(window, &self.capabilities)
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
        .unwrap_or_else(|| *formats.first().expect("No format available!"))
}

fn find_best_swapchain_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    present_modes
        .iter()
        .cloned()
        .find(|m| *m == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO)
}

fn get_swapchain_extent(
    window: &Window,
    capabilities: &vk::SurfaceCapabilitiesKHR,
) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D::default()
            .width(window.inner_size().width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ))
            .height(window.inner_size().height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ))
    }
}

fn create_swapchain_image_view(
    image: vk::Image,
    format: vk::Format,
    device: &ash::Device,
) -> Result<vk::ImageView, VkError> {
    let components = vk::ComponentMapping::default()
        .r(vk::ComponentSwizzle::IDENTITY)
        .g(vk::ComponentSwizzle::IDENTITY)
        .b(vk::ComponentSwizzle::IDENTITY)
        .a(vk::ComponentSwizzle::IDENTITY);

    let subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1);

    let info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .components(components)
        .subresource_range(subresource_range);

    let view = unsafe { device.create_image_view(&info, None) }?;

    Ok(view)
}

fn create_framebuffers(
    views: &Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    extent: &vk::Extent2D,
    device: &ash::Device,
) -> Result<Vec<vk::Framebuffer>, VkError> {
    let result = views
        .iter()
        .map(|i| {
            let attachments = &[*i];
            let create_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);

            unsafe { device.create_framebuffer(&create_info, None) }
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(result)
}

fn create_semaphores(device: &ash::Device, count: usize) -> Result<Vec<vk::Semaphore>, VkError> {
    let semaphore_info = vk::SemaphoreCreateInfo::default();
    (0..count)
        .map(|_| {
            unsafe { device.create_semaphore(&semaphore_info, None) }
                .map_err(|e| VkError::VkErrorCode(e))
        })
        .collect()
}

pub fn create_depth_image(
    device: &ash::Device,
    extent: vk::Extent2D,
    depth_format: vk::Format,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
) -> Result<VkImage, VkError> {
    let (texture_image, texture_image_memory) = create_image(
        device,
        extent.width,
        extent.height,
        depth_format,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        memory_properties,
        1
    )?;

    let view_info = vk::ImageViewCreateInfo::default()
        .image(texture_image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(depth_format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::DEPTH,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    let texture_image_view = unsafe { device.create_image_view(&view_info, None) }?;

    Ok(VkImage::new(
        texture_image,
        texture_image_memory,
        texture_image_view,
    ))
}

pub(crate) fn create_render_pass(
    device: &ash::Device,
    format: vk::Format,
    depth_format: vk::Format,
) -> Result<vk::RenderPass, VkError> {
    let color_attachment = vk::AttachmentDescription::default()
        .format(format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    let color_attachment_ref = vk::AttachmentReference::default()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let color_attachments = [color_attachment_ref];

    let depth_attachment = vk::AttachmentDescription::default()
        .format(vk::Format::D32_SFLOAT)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let depth_attachment_ref = vk::AttachmentReference::default()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let subpass = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachments)
        .depth_stencil_attachment(&depth_attachment_ref);

    let attachments = [color_attachment, depth_attachment];
    let subpasses = [subpass];
    let info = vk::RenderPassCreateInfo::default()
        .attachments(&attachments)
        .subpasses(&subpasses);

    Ok(unsafe { device.create_render_pass(&info, None)? })
}
