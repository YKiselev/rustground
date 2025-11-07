use vulkanalia::{
    Device, Instance, VkResult,
    vk::{
        self, DescriptorPool, DeviceV1_0, Extent2D, Fence, Format, Framebuffer, Handle, HasBuilder,
        Image, ImageView, KhrSurfaceExtension, KhrSwapchainExtension, PhysicalDevice, RenderPass,
        Semaphore, SurfaceFormatKHR, SurfaceKHR, SwapchainKHR,
    },
};
use winit::window::Window;

use crate::{error::VkError, pipeline::create_render_pass, queue_family::QueueFamilyIndices};

#[derive(Debug)]
pub(crate) struct SwapchainBootstrap<'a> {
    instance: &'a Instance,
    pub surface: SurfaceKHR,
    device: &'a Device,
    physical_device: PhysicalDevice,
    pub window: &'a Window,
    descriptor_set_layout: vk::DescriptorSetLayout,
}

impl<'a> SwapchainBootstrap<'a> {
    pub(crate) fn new(
        instance: &'a Instance,
        surface: SurfaceKHR,
        device: &'a Device,
        physical_device: PhysicalDevice,
        window: &'a Window,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Self {
        Self {
            instance,
            surface,
            device,
            physical_device,
            window,
            descriptor_set_layout,
        }
    }

    fn get_queue_family_indices(&self) -> Result<QueueFamilyIndices, VkError> {
        QueueFamilyIndices::get(self.instance, self.surface, self.physical_device)
    }

    fn get_swapchain_support(&self) -> Result<SwapchainSupport, VkError> {
        SwapchainSupport::get(self.instance, self.surface, self.physical_device)
    }

    fn create_swapchain(&self, info: &vk::SwapchainCreateInfoKHR) -> Result<SwapchainKHR, VkError> {
        Ok(unsafe { self.device.create_swapchain_khr(info, None)? })
    }

    fn get_images(&self, swapchain: SwapchainKHR) -> Result<Vec<Image>, VkError> {
        Ok(unsafe { self.device.get_swapchain_images_khr(swapchain) }?)
    }

    fn create_views(&self, format: Format, images: &Vec<Image>) -> Result<Vec<ImageView>, VkError> {
        Ok(create_swapchain_image_views(images, format, self.device)?)
    }

    fn create_render_pass(&self, format: Format) -> Result<RenderPass, VkError> {
        create_render_pass(self.device, format)
    }

    fn create_framebuffers(
        &self,
        views: &Vec<ImageView>,
        render_pass: RenderPass,
        extent: &Extent2D,
    ) -> Result<Vec<Framebuffer>, VkError> {
        create_framebuffers(views, render_pass, extent, self.device)
    }

    fn create_semaphores(&self, count: usize) -> Result<Vec<Semaphore>, VkError> {
        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        (0..count)
            .map(|_| {
                unsafe { self.device.create_semaphore(&semaphore_info, None) }
                    .map_err(|e| VkError::VkErrorCode(e))
            })
            .collect()
    }

    fn create_descriptor_sets(
        &self,
        pool: DescriptorPool,
        count: usize,
    ) -> Result<Vec<vk::DescriptorSet>, VkError> {
        let layouts = vec![self.descriptor_set_layout; count];
        let info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool)
            .set_layouts(&layouts);

        unsafe { self.device.allocate_descriptor_sets(&info) }.map_err(|e| VkError::VkErrorCode(e))
    }

    fn create_descriptor_pool(&self, count: usize) -> Result<DescriptorPool, VkError> {
        let ubo_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(count as u32);

        let pool_sizes = &[ubo_size];
        let info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(pool_sizes)
            .max_sets(count as u32);

        Ok(unsafe { self.device.create_descriptor_pool(&info, None) }?)
    }
}

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
}

impl Swapchain {
    pub fn new<'a>(bootstrap: &SwapchainBootstrap<'a>) -> Result<Swapchain, VkError> {
        let indices = bootstrap.get_queue_family_indices()?;
        let support = bootstrap.get_swapchain_support()?;
        let surface_format = get_swapchain_surface_format(&support.formats);
        let present_mode = get_swapchain_present_mode(&support.present_modes);
        let extent = support.get_swapchain_extent(bootstrap.window);

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
            .surface(bootstrap.surface)
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

        let swapchain = bootstrap.create_swapchain(&info)?;
        let images = bootstrap.get_images(swapchain)?;
        let views = bootstrap.create_views(surface_format.format, &images)?;
        let render_pass = bootstrap.create_render_pass(surface_format.format)?;
        let framebuffers = bootstrap.create_framebuffers(&views, render_pass, &extent)?;
        let render_finished = bootstrap.create_semaphores(images.len())?;
        let descriptor_pool = bootstrap.create_descriptor_pool(images.len())?;
        let descriptor_sets = bootstrap.create_descriptor_sets(descriptor_pool, images.len())?;

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
        })
    }

    pub fn destroy(&mut self, device: &Device) {
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

    pub fn aquire_next_image(
        &self,
        device: &Device,
        acquire_semaphore: Semaphore,
    ) -> VkResult<u32> {
        let (image_index, _) = unsafe {
            device.acquire_next_image_khr(
                self.swapchain,
                u64::MAX,
                acquire_semaphore,
                vk::Fence::null(),
            )
        }?;
        Ok(image_index)
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
