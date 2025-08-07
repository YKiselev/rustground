use std::{
    collections::HashSet,
    ffi::{CStr, c_void},
    time::Instant,
};

use cgmath::{Deg, point3, vec2, vec3};
use log::{debug, error, info, trace, warn};
use vulkanalia::{
    Device, Entry, Instance, Version,
    vk::{
        self, DeviceV1_0, EntryV1_0, ExtDebugUtilsExtension, Handle, HasBuilder, ImageView,
        InstanceV1_0, KhrSurfaceExtension, KhrSwapchainExtension, PhysicalDevice, Queue,
        SurfaceKHR,
    },
    window,
};
use winit::window::Window;

use crate::{
    error::{VkError, to_generic, to_suitability},
    pipeline::{Pipeline, create_render_pass},
    types::{Mat4, Vec2, Vec3},
    uniform::UniformBufferObject,
};

pub(crate) const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

pub(crate) const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

pub(crate) const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[rustfmt::skip]
static VERTICES: [Vertex; 4] = [
    Vertex::new(vec2(-0.5, -0.5), vec3(1.0, 0.0, 0.0)),
    Vertex::new(vec2(0.5, -0.5), vec3(0.0, 1.0, 0.0)),
    Vertex::new(vec2(0.5, 0.5), vec3(0.0, 0.0, 1.0)),
    Vertex::new(vec2(-0.5, 0.5), vec3(1.0, 1.0, 1.0)),
];
const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pos: Vec2,
    color: Vec3,
}

impl Vertex {
    const fn new(pos: Vec2, color: Vec3) -> Self {
        Self { pos, color }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0)
            .build();
        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec2>() as u32)
            .build();
        [pos, color]
    }
}

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
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline: Pipeline,
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    uniform_buffers: Vec<vk::Buffer>,
    uniform_buffers_memory: Vec<vk::DeviceMemory>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    images_in_flight: Vec<vk::Fence>,
    frame: usize,
    resized: bool,
    start: Instant,
}

#[derive(Debug, Default)]
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
        let mut result = Self {
            instance,
            messenger,
            surface,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain: Swapchain::default(),
            swapchain_image_views: vec![],
            render_pass: Default::default(),
            descriptor_set_layout: Default::default(),
            pipeline: Pipeline::default(),
            framebuffers: vec![],
            command_pool: Default::default(),
            vertex_buffer: Default::default(),
            vertex_buffer_memory: Default::default(),
            index_buffer: Default::default(),
            index_buffer_memory: Default::default(),
            uniform_buffers: vec![],
            uniform_buffers_memory: vec![],
            descriptor_pool: Default::default(),
            descriptor_sets: vec![],
            command_buffers: vec![],
            image_available_semaphores: vec![],
            render_finished_semaphores: vec![],
            in_flight_fences: vec![],
            images_in_flight: vec![],
            frame: 0,
            resized: false,
            start: Instant::now(),
        };
        result.init_swapchain(window)?;
        result.init_render_pass()?;
        result.init_descriptor_set_layout()?;
        result.init_pipeline()?;
        result.init_framebuffers()?;
        result.init_command_pool()?;
        result.init_vertex_buffer()?;
        result.init_index_buffer()?;
        result.init_uniform_buffers()?;
        result.init_descriptor_pool()?;
        result.init_descriptor_sets()?;
        result.init_command_buffers()?;
        result.init_sync_objects()?;
        Ok(result)
    }

    pub fn destroy(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.destroy_swapchain();

            let device = &self.device;
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            device.destroy_buffer(self.index_buffer, None);
            device.free_memory(self.index_buffer_memory, None);
            device.destroy_buffer(self.vertex_buffer, None);
            device.free_memory(self.vertex_buffer_memory, None);

            self.in_flight_fences
                .iter()
                .for_each(|f| device.destroy_fence(*f, None));
            self.render_finished_semaphores
                .iter()
                .for_each(|s| device.destroy_semaphore(*s, None));
            self.image_available_semaphores
                .iter()
                .for_each(|s| device.destroy_semaphore(*s, None));
            device.destroy_command_pool(self.command_pool, None);
            device.destroy_device(None);
            self.instance.destroy_surface_khr(self.surface, None);

            if VALIDATION_ENABLED {
                self.instance
                    .destroy_debug_utils_messenger_ext(self.messenger, None);
            }

            self.instance.destroy_instance(None);
        }
    }

    pub fn mark_resized(&mut self) {
        self.resized = true;
    }

    pub fn render(&mut self, window: &Window) -> Result<(), VkError> {
        let in_flight_fence = self.in_flight_fences[self.frame];

        unsafe {
            let fences = &[in_flight_fence];
            self.device.wait_for_fences(fences, true, u64::MAX)?;
        }

        let wait_semaphore = self.image_available_semaphores[self.frame];
        let result = unsafe {
            self.device.acquire_next_image_khr(
                self.swapchain.swapchain,
                u64::MAX,
                wait_semaphore,
                vk::Fence::null(),
            )
        };

        let image_index = match result {
            Ok((image_index, _)) => image_index as usize,
            Err(vk::ErrorCode::OUT_OF_DATE_KHR) => return self.recreate_swapchain(window),
            Err(e) => return Err(to_generic(e.to_string())),
        };

        let image_in_flight = self.images_in_flight[image_index];
        if !image_in_flight.is_null() {
            unsafe {
                let fences = &[image_in_flight];
                self.device.wait_for_fences(fences, true, u64::MAX)?;
            }
        }

        self.images_in_flight[image_index] = in_flight_fence;

        self.update_uniform_buffer(image_index)?;

        let wait_semaphores = &[wait_semaphore];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = &[self.command_buffers[image_index]];
        let signal_semaphores = &[self.render_finished_semaphores[image_index]];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .command_buffers(command_buffers)
            .signal_semaphores(signal_semaphores);

        unsafe {
            let fences = &[in_flight_fence];
            self.device.reset_fences(fences)?;
            let infos = &[submit_info];
            self.device
                .queue_submit(self.graphics_queue, infos, in_flight_fence)?;
        }

        let swapchains = &[self.swapchain.swapchain];
        let image_indices = &[image_index as u32];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(signal_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);

        let result = unsafe {
            self.device
                .queue_present_khr(self.present_queue, &present_info)
        };
        let changed = result == Ok(vk::SuccessCode::SUBOPTIMAL_KHR)
            || result == Err(vk::ErrorCode::OUT_OF_DATE_KHR);
        if self.resized || changed {
            self.resized = false;
            self.recreate_swapchain(window)?;
        } else if let Err(e) = result {
            return Err(to_generic(e.to_string()));
        }
        self.frame = (self.frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    fn update_uniform_buffer(&self, image_index: usize) -> Result<(), VkError> {
        let time = self.start.elapsed().as_secs_f32();

        let model = Mat4::from_axis_angle(vec3(0.0, 0.0, 1.0), Deg(90.0) * time);

        let view = Mat4::look_at_rh(
            point3::<f32>(2.0, 2.0, 2.0),
            point3::<f32>(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, 1.0),
        );

        let mut proj = cgmath::perspective(
            Deg(45.0),
            self.swapchain.extent.width as f32 / self.swapchain.extent.height as f32,
            0.1,
            10.0,
        );

        proj[1][1] *= -1.0; // OGL legacy)

        let ubo = UniformBufferObject { model, view, proj };

        let memory = unsafe {
            self.device.map_memory(
                self.uniform_buffers_memory[image_index],
                0,
                size_of::<UniformBufferObject>() as u64,
                vk::MemoryMapFlags::empty(),
            )
        }?;

        unsafe {
            std::ptr::copy_nonoverlapping(&ubo, memory.cast(), 1);
            self.device
                .unmap_memory(self.uniform_buffers_memory[image_index]);
        }

        Ok(())
    }

    fn recreate_swapchain(&mut self, window: &Window) -> Result<(), VkError> {
        unsafe { self.device.device_wait_idle() }?;
        self.destroy_swapchain();

        self.init_swapchain(window)?;
        self.init_render_pass()?;
        self.init_pipeline()?;
        self.init_framebuffers()?;
        self.init_uniform_buffers()?;
        self.init_descriptor_pool()?;
        self.init_descriptor_sets()?;
        self.init_command_buffers()?;

        self.images_in_flight
            .resize(self.swapchain.images.len(), vk::Fence::null());

        Ok(())
    }

    fn init_sync_objects(&mut self) -> Result<(), VkError> {
        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            self.image_available_semaphores
                .push(unsafe { self.device.create_semaphore(&semaphore_info, None) }?);
            self.in_flight_fences
                .push(unsafe { self.device.create_fence(&fence_info, None) }?);
        }
        for _ in self.swapchain.images.iter() {
            self.render_finished_semaphores
                .push(unsafe { self.device.create_semaphore(&semaphore_info, None) }?);
        }
        self.images_in_flight = self
            .swapchain
            .images
            .iter()
            .map(|_| vk::Fence::null())
            .collect();

        Ok(())
    }

    fn init_swapchain(&mut self, window: &Window) -> Result<(), VkError> {
        info!("Initializing swapchain");
        let indices = QueueFamilyIndices::get(&self.instance, self.surface, self.physical_device)?;
        let support = SwapchainSupport::get(&self.instance, self.surface, self.physical_device)?;

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
            .surface(self.surface)
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

        let swapchain = unsafe { self.device.create_swapchain_khr(&info, None) }?;
        let images = unsafe { self.device.get_swapchain_images_khr(swapchain) }?;

        self.swapchain = Swapchain {
            format: surface_format.format,
            extent,
            swapchain,
            images,
        };
        self.swapchain_image_views = self.swapchain.create_swapchain_image_views(&self.device)?;
        Ok(())
    }

    fn init_framebuffers(&mut self) -> Result<(), VkError> {
        info!("Init framebuffers");
        self.framebuffers = self
            .swapchain_image_views
            .iter()
            .map(|i| {
                let attachments = &[*i];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(self.render_pass)
                    .attachments(attachments)
                    .width(self.swapchain.extent.width)
                    .height(self.swapchain.extent.height)
                    .layers(1);

                unsafe { self.device.create_framebuffer(&create_info, None) }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }

    fn init_command_pool(&mut self) -> Result<(), VkError> {
        info!("Init command pool");
        let indices = QueueFamilyIndices::get(&self.instance, self.surface, self.physical_device)?;

        let info = vk::CommandPoolCreateInfo::builder().queue_family_index(indices.graphics);

        self.command_pool = unsafe { self.device.create_command_pool(&info, None) }?;
        Ok(())
    }

    fn init_command_buffers(&mut self) -> Result<(), VkError> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(self.framebuffers.len() as u32);

        let device = &self.device;
        let command_buffers = unsafe { device.allocate_command_buffers(&allocate_info) }?;

        // Commands

        for (i, command_buffer) in command_buffers.iter().enumerate() {
            let info = vk::CommandBufferBeginInfo::builder();

            unsafe { device.begin_command_buffer(*command_buffer, &info) }?;

            let render_area = vk::Rect2D::builder()
                .offset(vk::Offset2D::default())
                .extent(self.swapchain.extent);

            let color_clear_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            };

            let clear_values = &[color_clear_value];
            let info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[i])
                .render_area(render_area)
                .clear_values(clear_values);

            unsafe {
                device.cmd_begin_render_pass(*command_buffer, &info, vk::SubpassContents::INLINE);
                device.cmd_bind_pipeline(
                    *command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline.pipeline,
                );

                device.cmd_bind_vertex_buffers(*command_buffer, 0, &[self.vertex_buffer], &[0]);
                device.cmd_bind_index_buffer(
                    *command_buffer,
                    self.index_buffer,
                    0,
                    vk::IndexType::UINT16,
                );
                device.cmd_bind_descriptor_sets(
                    *command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline.layout,
                    0,
                    &[self.descriptor_sets[i]],
                    &[],
                );

                device.cmd_draw_indexed(*command_buffer, INDICES.len() as u32, 1, 0, 0, 0);

                device.cmd_end_render_pass(*command_buffer);
                device.end_command_buffer(*command_buffer)?;
            }
        }

        self.command_buffers = command_buffers;

        Ok(())
    }

    fn init_vertex_buffer(&mut self) -> Result<(), VkError> {
        let size = (size_of::<Vertex>() * VERTICES.len()) as u64;

        let (staging_buffer, staging_buffer_memory) = self.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;

        let device = &self.device;

        // Copy (staging)

        let memory = unsafe {
            device.map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())
        }?;

        unsafe {
            std::ptr::copy_nonoverlapping(VERTICES.as_ptr(), memory.cast(), VERTICES.len());

            device.unmap_memory(staging_buffer_memory);
        }

        // Create (vertex)

        let (vertex_buffer, vertex_buffer_memory) = self.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        self.vertex_buffer = vertex_buffer;
        self.vertex_buffer_memory = vertex_buffer_memory;

        // Copy (vertex)

        self.copy_buffer(staging_buffer, vertex_buffer, size)?;

        // Cleanup

        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_buffer_memory, None);
        }

        Ok(())
    }

    fn init_index_buffer(&mut self) -> Result<(), VkError> {
        // Create (staging)

        let size = (size_of::<u16>() * INDICES.len()) as u64;

        let (staging_buffer, staging_buffer_memory) = self.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;

        let device = &self.device;

        // Copy (staging)

        let memory = unsafe {
            device.map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())
        }?;

        unsafe {
            std::ptr::copy_nonoverlapping(INDICES.as_ptr(), memory.cast(), INDICES.len());
            device.unmap_memory(staging_buffer_memory);
        };

        // Create (index)

        let (index_buffer, index_buffer_memory) = self.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        self.index_buffer = index_buffer;
        self.index_buffer_memory = index_buffer_memory;

        // Copy (index)

        self.copy_buffer(staging_buffer, index_buffer, size)?;

        // Cleanup

        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_buffer_memory, None);
        }

        Ok(())
    }

    fn init_uniform_buffers(&mut self) -> Result<(), VkError> {
        self.uniform_buffers.clear();
        self.uniform_buffers_memory.clear();

        for _ in 0..self.swapchain.images.len() {
            let (uniform_buffer, uniform_buffer_memory) = self.create_buffer(
                size_of::<UniformBufferObject>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            )?;

            self.uniform_buffers.push(uniform_buffer);
            self.uniform_buffers_memory.push(uniform_buffer_memory);
        }

        Ok(())
    }

    fn init_descriptor_pool(&mut self) -> Result<(), VkError> {
        info!("Init descriptor pool");
        let ubo_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(self.swapchain.images.len() as u32);

        let pool_sizes = &[ubo_size];
        let info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(pool_sizes)
            .max_sets(self.swapchain.images.len() as u32);

        self.descriptor_pool = unsafe { self.device.create_descriptor_pool(&info, None) }?;

        Ok(())
    }

    fn init_render_pass(&mut self) -> Result<(), VkError> {
        self.render_pass = create_render_pass(&self.device, self.swapchain.format)?;
        Ok(())
    }

    fn init_pipeline(&mut self) -> Result<(), VkError> {
        info!("Init pipeline");
        self.pipeline = Pipeline::new(
            &self.device,
            self.swapchain.extent,
            self.render_pass,
            self.descriptor_set_layout,
        )?;
        info!("End of pipeline init");
        Ok(())
    }

    fn init_descriptor_set_layout(&mut self) -> Result<(), VkError> {
        info!("Init descriptor set layout");
        let ubo_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);

        let bindings = &[ubo_binding];
        let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);

        self.descriptor_set_layout =
            unsafe { self.device.create_descriptor_set_layout(&info, None) }?;

        Ok(())
    }

    fn init_descriptor_sets(&mut self) -> Result<(), VkError> {
        info!("Init descriptor sets");
        // Allocate

        let layouts = vec![self.descriptor_set_layout; self.swapchain.images.len()];
        let info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&layouts);

        self.descriptor_sets = unsafe { self.device.allocate_descriptor_sets(&info) }?;

        // Update

        for i in 0..self.swapchain.images.len() {
            let info = vk::DescriptorBufferInfo::builder()
                .buffer(self.uniform_buffers[i])
                .offset(0)
                .range(size_of::<UniformBufferObject>() as u64);

            let buffer_info = &[info];
            let ubo_write = vk::WriteDescriptorSet::builder()
                .dst_set(self.descriptor_sets[i])
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(buffer_info);

            unsafe {
                self.device
                    .update_descriptor_sets(&[ubo_write], &[] as &[vk::CopyDescriptorSet])
            };
        }

        Ok(())
    }

    fn copy_buffer(
        &self,
        source: vk::Buffer,
        destination: vk::Buffer,
        size: vk::DeviceSize,
    ) -> Result<(), VkError> {
        // Allocate

        let info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.command_pool)
            .command_buffer_count(1);

        let device = &self.device;
        let command_buffer = unsafe { device.allocate_command_buffers(&info) }?[0];

        // Commands

        let info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe { device.begin_command_buffer(command_buffer, &info) }?;

        let regions = vk::BufferCopy::builder().size(size);
        unsafe {
            device.cmd_copy_buffer(command_buffer, source, destination, &[regions]);
            device.end_command_buffer(command_buffer)?;
        }

        // Submit

        let command_buffers = &[command_buffer];
        let info = vk::SubmitInfo::builder().command_buffers(command_buffers);

        unsafe {
            device.queue_submit(self.graphics_queue, &[info], vk::Fence::null())?;
            device.queue_wait_idle(self.graphics_queue)?;
        }

        // Cleanup

        unsafe { device.free_command_buffers(self.command_pool, &[command_buffer]) };

        Ok(())
    }

    fn get_memory_type_index(
        &self,
        properties: vk::MemoryPropertyFlags,
        requirements: vk::MemoryRequirements,
    ) -> Result<u32, VkError> {
        let memory = unsafe {
            self.instance
                .get_physical_device_memory_properties(self.physical_device)
        };
        (0..memory.memory_type_count)
            .find(|i| {
                let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
                let memory_type = memory.memory_types[*i as usize];
                suitable && memory_type.property_flags.contains(properties)
            })
            .ok_or_else(|| to_generic("Failed to find suitable memory type."))
    }

    fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory), VkError> {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { self.device.create_buffer(&buffer_info, None) }?;

        let requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };

        let memory_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(self.get_memory_type_index(properties, requirements)?);

        let buffer_memory = unsafe { self.device.allocate_memory(&memory_info, None) }?;

        unsafe { self.device.bind_buffer_memory(buffer, buffer_memory, 0) }?;

        Ok((buffer, buffer_memory))
    }

    fn destroy_swapchain(&mut self) {
        unsafe {
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.uniform_buffers
                .iter()
                .for_each(|b| self.device.destroy_buffer(*b, None));
            self.uniform_buffers_memory
                .iter()
                .for_each(|m| self.device.free_memory(*m, None));
            self.device
                .free_command_buffers(self.command_pool, &self.command_buffers);
            self.framebuffers
                .iter()
                .for_each(|f| self.device.destroy_framebuffer(*f, None));
            self.pipeline.destroy(&self.device);
            self.device.destroy_render_pass(self.render_pass, None);
            self.swapchain_image_views
                .iter()
                .for_each(|v| self.device.destroy_image_view(*v, None));
            self.device
                .destroy_swapchain_khr(self.swapchain.swapchain, None);
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

        if let Err(error) = check_physical_device(instance, surface, physical_device) {
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
        for (index, _) in properties.iter().enumerate() {
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
}
