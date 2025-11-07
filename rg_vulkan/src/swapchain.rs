use vulkanalia::{
    Device, Instance, VkResult,
    vk::{
        self, CommandPool, DescriptorPool, DescriptorSetLayout, DeviceV1_0, ErrorCode, Extent2D,
        Fence, Format, Framebuffer, Handle, HasBuilder, Image, ImageView, KhrSurfaceExtension,
        KhrSwapchainExtension, PhysicalDevice, Queue, RenderPass, Semaphore, SuccessCode,
        SurfaceFormatKHR, SurfaceKHR, SwapchainKHR,
    },
};
use winit::window::Window;

use crate::{
    error::VkError, frames_in_flight::FramesInFlight, pipeline::create_render_pass,
    queue_family::QueueFamilyIndices,
};

#[derive(Debug, Default)]
pub(crate) struct Swapchain {
    pub format: Format,
    pub extent: Extent2D,
    pub swapchain: SwapchainKHR,
    pub images: Vec<Image>,
    pub views: Vec<ImageView>,
    pub render_pass: RenderPass,
    pub framebuffers: Vec<Framebuffer>,
    pub render_finished: Vec<vk::Semaphore>,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub frames_in_flight: FramesInFlight,
}

impl Swapchain {
    pub fn new(
        instance: &Instance,
        surface: SurfaceKHR,
        device: &Device,
        physical_device: PhysicalDevice,
        window: &Window,
        descriptor_set_layout: vk::DescriptorSetLayout,
        command_pool: vk::CommandPool,
    ) -> Result<Swapchain, VkError> {
        let indices = QueueFamilyIndices::get(instance, surface, physical_device)?;
        let support = SwapchainSupport::get(instance, surface, physical_device)?;
        let surface_format = get_swapchain_surface_format(&support.formats);
        let present_mode = get_swapchain_present_mode(&support.present_modes);
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
        let views = create_swapchain_image_views(&images, surface_format.format, device)?;
        let render_pass = create_render_pass(device, surface_format.format)?;
        let framebuffers = create_framebuffers(&views, render_pass, &extent, device)?;
        let render_finished = create_semaphores(device, images.len())?;
        let descriptor_pool = create_descriptor_pool(device, images.len())?;
        let descriptor_sets =
            create_descriptor_sets(device, descriptor_set_layout, descriptor_pool, images.len())?;
        let frames_in_flight = FramesInFlight::new(device, command_pool)?;
        Ok(Swapchain {
            format: surface_format.format,
            extent,
            swapchain,
            images,
            views,
            render_pass,
            framebuffers,
            render_finished,
            descriptor_pool,
            descriptor_sets,
            frames_in_flight,
        })
    }

    pub fn destroy(&mut self, device: &Device, pool: CommandPool) {
        self.frames_in_flight.destroy(device, pool);

        unsafe {
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.render_finished
                .iter()
                .for_each(|s| device.destroy_semaphore(*s, None));
            self.render_finished.clear();
            self.framebuffers
                .iter()
                .for_each(|f| device.destroy_framebuffer(*f, None));
            self.framebuffers.clear();
            device.destroy_render_pass(self.render_pass, None);
            self.views
                .iter()
                .for_each(|v| device.destroy_image_view(*v, None));
            self.views.clear();
            device.destroy_swapchain_khr(self.swapchain, None);
        }
    }

    pub fn acquire_next_image(&self, device: &Device, frame: usize) -> Result<usize, VkError> {
        let fences = &[self.frames_in_flight.frence(frame)];
        unsafe {
            device.wait_for_fences(fences, true, u64::MAX)?;
            device.reset_fences(fences)?;
        };
        match unsafe {
            device.acquire_next_image_khr(
                self.swapchain,
                u64::MAX,
                self.frames_in_flight.image_available_semaphore(frame),
                vk::Fence::null(),
            )
        } {
            Ok((image_index, code)) => {
                if code == SuccessCode::SUBOPTIMAL_KHR {
                    Err(VkError::SwapchainChanged)
                } else {
                    Ok(image_index as usize)
                }
            }
            Err(vk::ErrorCode::OUT_OF_DATE_KHR) => Err(VkError::SwapchainChanged),
            Err(e) => Err(e.into()),
        }
    }

    pub fn present(
        &self,
        device: &Device,
        graphics_queue: Queue,
        present_queue: Queue,
        frame: usize,
        image_index: usize,
    ) -> Result<SuccessCode, ErrorCode> {
        let wait_semaphores = &[self.frames_in_flight.image_available_semaphore(frame)];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = &[self.frames_in_flight.command_buffer(frame)];
        let signal_semaphores = &[self.render_finished[image_index]];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .command_buffers(command_buffers)
            .signal_semaphores(signal_semaphores);

        unsafe {
            let infos = &[submit_info];
            device.queue_submit(graphics_queue, infos, self.frames_in_flight.frence(frame))?;
        }

        let swapchains = &[self.swapchain];
        let image_indices = &[image_index as u32];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(signal_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);

        unsafe { device.queue_present_khr(present_queue, &present_info) }
    }
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

    pub fn get_optimal_image_count(&self) -> u32 {
        let mut image_count = self.capabilities.min_image_count + 1;
        if self.capabilities.max_image_count != 0 && image_count > self.capabilities.max_image_count
        {
            image_count = self.capabilities.max_image_count;
        }
        image_count
    }

    fn get_swapchain_extent(&self, window: &Window) -> vk::Extent2D {
        get_swapchain_extent(window, self.capabilities)
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

fn create_swapchain_image_views(
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

fn create_framebuffers(
    views: &Vec<ImageView>,
    render_pass: RenderPass,
    extent: &Extent2D,
    device: &Device,
) -> Result<Vec<Framebuffer>, VkError> {
    let result = views
        .iter()
        .map(|i| {
            let attachments = &[*i];
            let create_info = vk::FramebufferCreateInfo::builder()
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

fn create_semaphores(device: &Device, count: usize) -> Result<Vec<Semaphore>, VkError> {
    let semaphore_info = vk::SemaphoreCreateInfo::builder();
    (0..count)
        .map(|_| {
            unsafe { device.create_semaphore(&semaphore_info, None) }
                .map_err(|e| VkError::VkErrorCode(e))
        })
        .collect()
}

fn create_descriptor_pool(device: &Device, count: usize) -> Result<DescriptorPool, VkError> {
    let ubo_size = vk::DescriptorPoolSize::builder()
        .type_(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(count as u32);

    let pool_sizes = &[ubo_size];
    let info = vk::DescriptorPoolCreateInfo::builder()
        .pool_sizes(pool_sizes)
        .max_sets(count as u32);

    Ok(unsafe { device.create_descriptor_pool(&info, None) }?)
}

fn create_descriptor_sets(
    device: &Device,
    layout: DescriptorSetLayout,
    pool: DescriptorPool,
    count: usize,
) -> Result<Vec<vk::DescriptorSet>, VkError> {
    let layouts = vec![layout; count];
    let info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(pool)
        .set_layouts(&layouts);

    unsafe { device.allocate_descriptor_sets(&info) }.map_err(|e| VkError::VkErrorCode(e))
}
