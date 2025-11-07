use std::u64;

use vulkanalia::{
    Device, Entry, Instance,
    vk::{
        self, CommandBuffer, CommandPoolCreateFlags, DeviceMemory, DeviceSize, DeviceV1_0,
        ExtDebugUtilsExtension, Handle, HasBuilder, InstanceV1_0, KhrSurfaceExtension,
        KhrSwapchainExtension, MemoryMapFlags, PhysicalDevice, Queue, SurfaceKHR,
    },
    window,
};
use winit::window::Window;

use crate::{
    create_instance::create_instance,
    device::{VALIDATION_ENABLED, create_logical_device, pick_physical_device},
    error::{VkError, to_generic},
    frames_in_flight::FramesInFlight,
    queue_family::QueueFamilyIndices,
    swapchain::Swapchain,
};

pub(crate) const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

pub(crate) const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[derive(Debug)]
pub struct VkInstance {
    pub instance: Instance,
    messenger: vk::DebugUtilsMessengerEXT,
    pub surface: SurfaceKHR,
    pub physical_device: PhysicalDevice,
    pub device: Device,
    pub graphics_queue: Queue,
    pub present_queue: Queue,
    pub swapchain: Swapchain,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub command_pool: vk::CommandPool,
    frame: usize,
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
            descriptor_set_layout: Default::default(),
            command_pool: Default::default(),
            frame: 0,
        };
        result.init_descriptor_set_layout()?;
        result.init_command_pool()?;
        result.init_swapchain(window)?;
        Ok(result)
    }

    pub fn wait_idle(&self) -> Result<(), VkError> {
        unsafe {
            self.device.device_wait_idle()?;
        }
        Ok(())
    }

    pub fn destroy(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.destroy_swapchain();
            let device = &self.device;

            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
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

    pub fn command_buffer(&self) -> CommandBuffer {
        self.swapchain.frames_in_flight.command_buffer(self.frame)
    }

    pub fn begin_frame(&self) -> Result<usize, VkError> {
        self.swapchain.acquire_next_image(&self.device, self.frame)
    }

    pub fn end_frame(&mut self, image_index: usize) -> Result<bool, VkError> {
        let result = self.swapchain.present(
            &self.device,
            self.graphics_queue,
            self.present_queue,
            self.frame,
            image_index,
        );

        self.frame = (self.frame + 1) % MAX_FRAMES_IN_FLIGHT;

        let changed = result == Ok(vk::SuccessCode::SUBOPTIMAL_KHR)
            || result == Err(vk::ErrorCode::OUT_OF_DATE_KHR);
        if !changed {
            if let Err(e) = result {
                return Err(e.into());
            }
        }

        Ok(changed)
    }

    pub fn recreate_swapchain(&mut self, window: &Window) -> Result<(), VkError> {
        self.wait_idle()?;
        self.destroy_swapchain();
        self.init_swapchain(window)?;
        Ok(())
    }

    fn init_swapchain(&mut self, window: &Window) -> Result<(), VkError> {
        self.swapchain = Swapchain::new(
            &self.instance,
            self.surface,
            &self.device,
            self.physical_device,
            window,
            self.descriptor_set_layout,
            self.command_pool,
        )?;
        Ok(())
    }

    fn init_command_pool(&mut self) -> Result<(), VkError> {
        let indices = QueueFamilyIndices::get(&self.instance, self.surface, self.physical_device)?;

        let info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(indices.graphics)
            .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        self.command_pool = unsafe { self.device.create_command_pool(&info, None) }?;
        Ok(())
    }

    fn init_descriptor_set_layout(&mut self) -> Result<(), VkError> {
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

    pub fn copy_buffer(
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

        unsafe {
            device.begin_command_buffer(command_buffer, &info)?;
            let regions = vk::BufferCopy::builder().size(size);
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

    pub fn create_buffer(
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

    fn create_texture_image(&self) -> Result<(), VkError> {
        /*
                let image = File::open("tutorial/resources/texture.png")?;

                let decoder = png::Decoder::new(image);
                let mut reader = decoder.read_info()?;

                let mut pixels = vec![0; reader.info().raw_bytes()];
                reader.next_frame(&mut pixels)?;

                let size = reader.info().raw_bytes() as u64;
                let (width, height) = reader.info().size();

                // Create (staging)

                let (staging_buffer, staging_buffer_memory) = self.create_buffer(
                    size,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                )?;

                // Copy (staging)
                let device = &self.device;

                self.copy_memory(
                    staging_buffer_memory,
                    0,
                    size,
                    vk::MemoryMapFlags::empty(),
                    pixels.as_ptr(),
                    pixels.len(),
                )?;

                // Create (image)

                let (texture_image, texture_image_memory) = self.create_image(
                    width,
                    height,
                    vk::Format::R8G8B8A8_SRGB,
                    vk::ImageTiling::OPTIMAL,
                    vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                )?;

                data.texture_image = texture_image;
                data.texture_image_memory = texture_image_memory;

                // Transition + Copy (image)

                transition_image_layout(
                    device,
                    data,
                    data.texture_image,
                    vk::Format::R8G8B8A8_SRGB,
                    vk::ImageLayout::UNDEFINED,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                )?;

                copy_buffer_to_image(
                    device,
                    data,
                    staging_buffer,
                    data.texture_image,
                    width,
                    height,
                )?;

                transition_image_layout(
                    device,
                    data,
                    data.texture_image,
                    vk::Format::R8G8B8A8_SRGB,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                )?;

                unsafe {
                    device.destroy_buffer(staging_buffer, None);
                    device.free_memory(staging_buffer_memory, None)
                };
        */
        Ok(())
    }

    pub fn copy_memory<T>(
        &self,
        dest: DeviceMemory,
        dest_offset: DeviceSize,
        size: DeviceSize,
        flags: MemoryMapFlags,
        src: *const T,
        count: usize,
    ) -> Result<(), VkError> {
        unsafe {
            let memory = self.device.map_memory(dest, dest_offset, size, flags)?;
            std::ptr::copy_nonoverlapping(src, memory.cast(), count);
            self.device.unmap_memory(dest);
        }
        Ok(())
    }

    fn create_image(
        &self,
        width: u32,
        height: u32,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Image, vk::DeviceMemory), VkError> {
        let info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::_2D)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::_1);

        let device = &self.device;
        let image = unsafe { device.create_image(&info, None) }?;

        // Memory

        let requirements = unsafe { device.get_image_memory_requirements(image) };

        let info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(self.get_memory_type_index(properties, requirements)?);

        let image_memory = unsafe { device.allocate_memory(&info, None) }?;

        unsafe { device.bind_image_memory(image, image_memory, 0) }?;

        Ok((image, image_memory))
    }

    fn destroy_swapchain(&mut self) {
        self.swapchain.destroy(&self.device, self.command_pool);
    }
}

fn create_surface(instance: &Instance, window: &Window) -> Result<SurfaceKHR, VkError> {
    Ok(unsafe { window::create_surface(instance, window, window) }?)
}
