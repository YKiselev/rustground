use std::ffi::CStr;
use std::io::BufReader;
use std::sync::{Arc, RwLock};

use ash::khr::{self, swapchain};
use ash::vk;
use ash::vk::PhysicalDevice;
use log::info;
use rg_common::{App, Files};
use winit::window::Window;

use crate::config::Config;
use crate::device::DeviceId;
use crate::image::create_image;
use crate::memory::VkMemoryProperties;
use crate::surface::VkSurface;
use crate::{
    device::{create_logical_device, pick_physical_device},
    error::{VkError, to_generic},
    image::VkImage,
    queue_family::QueueFamilyIndices,
    swapchain::swapchain::Swapchain,
};

pub(crate) const DEVICE_EXTENSIONS: [&CStr; 1] = [swapchain::NAME];

pub(crate) const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[derive()]
pub struct VkInstance {
    surface: VkSurface,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub swapchain_device: khr::swapchain::Device,
    pub swapchain: Swapchain,
    pub command_pool: vk::CommandPool,
    pub sampler: vk::Sampler,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub depth_format: vk::Format,
}

impl VkInstance {
    pub fn new(
        app: &Arc<App>,
        config: &Arc<RwLock<Config>>,
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &Window,
    ) -> Result<Self, VkError> {
        info!("Creating Vulkan surface...");
        let surface = VkSurface::new(entry, instance, window)?;
        info!("Preparing config...");
        let mut cfg = config.write()?;
        let preferred_device_id = cfg
            .preferred_device
            .as_ref()
            .and_then(|v| DeviceId::parse(v));

        info!("Picking physical device...");
        let (device_id, physical_device, device_properties) =
            pick_physical_device(&instance, &surface, &preferred_device_id)?;
        let device_name =
            unsafe { CStr::from_ptr(device_properties.device_name.as_ptr()).to_string_lossy() };
        info!("Using {} ({})", device_name, device_id);

        if Some(&device_id) != preferred_device_id.as_ref() {
            cfg.preferred_device = Some(device_id.to_string());
        }

        let (device, graphics_queue, present_queue) =
            create_logical_device(&instance, &surface, physical_device)?;
        let memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };
        let depth_format = pick_depth_format(instance, physical_device)?;
        let command_pool = create_command_pool(&instance, &device, &surface, physical_device)?;
        let swapchain_device = khr::swapchain::Device::new(&instance, &device);
        let swapchain = Swapchain::new(
            &instance,
            &surface,
            &device,
            &swapchain_device,
            physical_device,
            window,
            depth_format,
            &memory_properties,
            vk::SwapchainKHR::null(),
        )?;
        let sampler = create_sampler(&device)?;

        let result = Self {
            surface,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain_device,
            swapchain,
            command_pool,
            sampler,
            memory_properties,
            depth_format,
        };
        Ok(result)
    }

    pub fn wait_idle(&self) -> Result<(), VkError> {
        unsafe {
            self.device.device_wait_idle()?;
        }
        Ok(())
    }

    pub fn begin_frame(&self) -> Result<usize, VkError> {
        self.swapchain
            .acquire_next_image(&self.device, &self.swapchain_device)
    }

    pub fn end_frame(&mut self, image_index: usize, window: &Window) -> Result<bool, VkError> {
        let result = self.swapchain.present(
            window,
            &self.device,
            &self.swapchain_device,
            self.graphics_queue,
            self.present_queue,
            image_index,
        );

        self.swapchain.advance_frame_index();

        let changed = result == Ok(true) || result == Err(vk::Result::ERROR_OUT_OF_DATE_KHR);
        if !changed {
            if let Err(e) = result {
                return Err(e.into());
            }
        }

        Ok(changed)
    }

    pub fn recreate_swapchain(
        &mut self,
        instance: &ash::Instance,
        window: &Window,
    ) -> Result<(), VkError> {
        self.wait_idle()?;
        let old_swapchain_khr = self.swapchain.destroy(&self.device);
        let result = Swapchain::new(
            instance,
            &self.surface,
            &self.device,
            &self.swapchain_device,
            self.physical_device,
            window,
            self.depth_format,
            &self.memory_properties,
            old_swapchain_khr,
        );
        self.destroy_swapchain_khr(old_swapchain_khr);
        self.swapchain = result?;

        Ok(())
    }

    pub fn copy_buffer(
        &self,
        source: vk::Buffer,
        destination: vk::Buffer,
        size: vk::DeviceSize,
    ) -> Result<(), VkError> {
        // Allocate
        let info = vk::CommandBufferAllocateInfo::default()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.command_pool)
            .command_buffer_count(1);

        let device = &self.device;
        let command_buffer = unsafe { device.allocate_command_buffers(&info) }?[0];

        // Commands
        let info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            device.begin_command_buffer(command_buffer, &info)?;
            let regions = [vk::BufferCopy::default().size(size)];
            device.cmd_copy_buffer(command_buffer, source, destination, regions.as_slice());
            device.end_command_buffer(command_buffer)?;
        }

        // Submit
        let command_buffers = &[command_buffer];
        let info = vk::SubmitInfo::default().command_buffers(command_buffers);

        unsafe {
            device.queue_submit(self.graphics_queue, &[info], vk::Fence::null())?;
            device.queue_wait_idle(self.graphics_queue)?;
        }

        // Cleanup
        let buffers = [command_buffer];
        unsafe { device.free_command_buffers(self.command_pool, buffers.as_slice()) };

        Ok(())
    }

    pub fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory), VkError> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { self.device.create_buffer(&buffer_info, None) }?;

        let requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };

        let memory_info = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(
                self.memory_properties
                    .get_memory_type_index(properties, requirements)?,
            );

        let buffer_memory = unsafe { self.device.allocate_memory(&memory_info, None) }?;

        unsafe { self.device.bind_buffer_memory(buffer, buffer_memory, 0) }?;

        Ok((buffer, buffer_memory))
    }

    pub fn create_texture_image(&self, files: &Files) -> Result<VkImage, VkError> {
        let file = files.read("textures/tex1.png")?;
        let image = BufReader::new(file);

        let decoder = png::Decoder::new(image);
        let mut reader = decoder
            .read_info()
            .map_err(|e| VkError::GenericError(e.to_string()))?;

        let size_in_bytes = reader.output_buffer_size().ok_or(VkError::GenericError("Out of memory!".to_string()))?;
        let mut pixels = vec![0; size_in_bytes];
        reader
            .next_frame(&mut pixels)
            .map_err(|e| VkError::GenericError(e.to_string()))?;

        let (width, height) = reader.info().size();
        let layers = [pixels];

        self.create_texture_image_from_pixels(width, height, &layers, vk::Format::R8G8B8A8_SRGB)
    }

    pub fn create_texture_image_from_pixels(
        &self,
        width: u32,
        height: u32,
        layer_pixels: &[Vec<u8>],
        format: vk::Format,
    ) -> Result<VkImage, VkError> {
        assert!(!layer_pixels.is_empty(), "There is no layers!");

        // Create (staging)
        let buf_size: usize = layer_pixels.iter().map(|l| l.len()).sum();
        let (staging_buffer, staging_buffer_memory) = self.create_buffer(
            buf_size as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;

        // Copy (staging)
        let device = &self.device;

        let mut dest_offset = 0;
        for i in 0..layer_pixels.len() {
            let layer_size = layer_pixels[i].len();
            self.copy_memory(
                staging_buffer_memory,
                dest_offset,
                layer_size as u64,
                vk::MemoryMapFlags::empty(),
                layer_pixels[i].as_ptr(),
                layer_size,
            )?;
            dest_offset += layer_size as u64;
        }

        // Create (image)
        let (texture_image, texture_image_memory) = create_image(
            &self.device,
            width,
            height,
            format,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &self.memory_properties,
            layer_pixels.len() as u32,
        )?;

        // Transition + Copy (image)
        self.transition_image_layout(
            texture_image,
            format,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )?;

        self.copy_buffer_to_image(staging_buffer, texture_image, width, height, layer_pixels.len() as u32)?;

        self.transition_image_layout(
            texture_image,
            format,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )?;

        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_buffer_memory, None)
        };
        let mut view_type = vk::ImageViewType::TYPE_2D;
        if layer_pixels.len() > 1 {
            view_type = vk::ImageViewType::TYPE_2D_ARRAY;
        }
        let view_info = vk::ImageViewCreateInfo::default()
            .image(texture_image)
            .view_type(view_type)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: layer_pixels.len() as u32,
            });

        let texture_image_view = unsafe { device.create_image_view(&view_info, None) }?;

        Ok(VkImage::new(
            texture_image,
            texture_image_memory,
            texture_image_view,
        ))
    }

    pub fn copy_memory<T>(
        &self,
        dest: vk::DeviceMemory,
        dest_offset: vk::DeviceSize,
        size: vk::DeviceSize,
        flags: vk::MemoryMapFlags,
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

    // fn create_image(
    //     &self,
    //     width: u32,
    //     height: u32,
    //     format: vk::Format,
    //     usage: vk::ImageUsageFlags,
    //     properties: vk::MemoryPropertyFlags,
    // ) -> Result<(vk::Image, vk::DeviceMemory), VkError> {
    //     let info = vk::ImageCreateInfo::default()
    //         .image_type(vk::ImageType::TYPE_2D)
    //         .extent(vk::Extent3D {
    //             width,
    //             height,
    //             depth: 1,
    //         })
    //         .mip_levels(1)
    //         .array_layers(1)
    //         .format(format)
    //         .tiling(vk::ImageTiling::OPTIMAL)
    //         .initial_layout(vk::ImageLayout::UNDEFINED)
    //         .usage(usage)
    //         .sharing_mode(vk::SharingMode::EXCLUSIVE)
    //         .samples(vk::SampleCountFlags::TYPE_1);

    //     let device = &self.device;
    //     let image = unsafe { device.create_image(&info, None) }?;

    //     // Memory

    //     let requirements = unsafe { device.get_image_memory_requirements(image) };

    //     let info = vk::MemoryAllocateInfo::default()
    //         .allocation_size(requirements.size)
    //         .memory_type_index(self.memory_properties.get_memory_type_index(properties, requirements)?);

    //     let image_memory = unsafe { device.allocate_memory(&info, None) }?;

    //     unsafe { device.bind_image_memory(image, image_memory, 0) }?;

    //     Ok((image, image_memory))
    // }

    fn transition_image_layout(
        &self,
        image: vk::Image,
        format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> Result<(), VkError> {
        let (src_access_mask, dst_access_mask, src_stage_mask, dst_stage_mask) =
            match (old_layout, new_layout) {
                (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                ),
                (
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                ) => (
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::AccessFlags::SHADER_READ,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                ),
                _ => {
                    return Err(VkError::GenericError(
                        "Unsupported image layout transition!".to_owned(),
                    ));
                }
            };

        let command_buffer = self.begin_single_time_commands()?;

        let subresource = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        let barrier = vk::ImageMemoryBarrier::default()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(subresource)
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask);

        unsafe {
            let mem_barriers = [];
            let buf_barriers = [];
            let img_barriers = [barrier];
            self.device.cmd_pipeline_barrier(
                command_buffer,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                mem_barriers.as_slice(),
                buf_barriers.as_slice(),
                img_barriers.as_slice(),
            )
        };

        self.end_single_time_commands(command_buffer)?;

        Ok(())
    }

    fn begin_single_time_commands(&self) -> Result<vk::CommandBuffer, VkError> {
        // Allocate

        let info = vk::CommandBufferAllocateInfo::default()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.command_pool)
            .command_buffer_count(1);

        let command_buffer = unsafe { self.device.allocate_command_buffers(&info) }?[0];

        // Begin

        let info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device.begin_command_buffer(command_buffer, &info)?;
        }

        Ok(command_buffer)
    }

    fn end_single_time_commands(&self, command_buffer: vk::CommandBuffer) -> Result<(), VkError> {
        // End

        (unsafe { self.device.end_command_buffer(command_buffer) })?;

        // Submit

        let command_buffers = &[command_buffer];
        let info = vk::SubmitInfo::default().command_buffers(command_buffers);

        unsafe {
            self.device
                .queue_submit(self.graphics_queue, &[info], vk::Fence::null())?;
            self.device.queue_wait_idle(self.graphics_queue)?;
        }

        // Cleanup

        unsafe {
            let buffers = [command_buffer];
            self.device
                .free_command_buffers(self.command_pool, buffers.as_slice())
        };

        Ok(())
    }

    fn copy_buffer_to_image(
        &self,
        buffer: vk::Buffer,
        image: vk::Image,
        width: u32,
        height: u32,
        layers_count: u32,
    ) -> Result<(), VkError> {
        let command_buffer = self.begin_single_time_commands()?;

        let layer_size = width * height;
        for i in 0..layers_count {
            let buffer_offset = (i * layer_size) as u64;
            let subresource = vk::ImageSubresourceLayers::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .mip_level(0)
                .base_array_layer(i)
                .layer_count(1);

            let region = vk::BufferImageCopy::default()
                .buffer_offset(buffer_offset)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(subresource)
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                });

            unsafe {
                let regions = [region];
                self.device.cmd_copy_buffer_to_image(
                    command_buffer,
                    buffer,
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    regions.as_slice(),
                )
            }
        }

        self.end_single_time_commands(command_buffer)?;

        Ok(())
    }

    fn destroy_swapchain_khr(&self, swapchain: vk::SwapchainKHR) {
        if swapchain != vk::SwapchainKHR::null() {
            unsafe {
                self.swapchain_device.destroy_swapchain(swapchain, None);
            }
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            let device = &self.device;
            let old_swapchain_khr = self.swapchain.destroy(device);
            self.destroy_swapchain_khr(old_swapchain_khr);
            device.destroy_sampler(self.sampler, None);
            device.destroy_command_pool(self.command_pool, None);
            device.destroy_device(None);
            std::ptr::drop_in_place(&mut self.surface);

            self.surface.destroy();
        }
    }
}

fn create_command_pool(
    instance: &ash::Instance,
    device: &ash::Device,
    surface: &VkSurface,
    physical_device: PhysicalDevice,
) -> Result<vk::CommandPool, VkError> {
    let indices = QueueFamilyIndices::get(instance, surface, physical_device)?;

    let info = vk::CommandPoolCreateInfo::default()
        .queue_family_index(indices.graphics)
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

    let command_pool = unsafe { device.create_command_pool(&info, None) }?;
    Ok(command_pool)
}

fn create_sampler(device: &ash::Device) -> Result<vk::Sampler, VkError> {
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::REPEAT)
        .address_mode_v(vk::SamplerAddressMode::REPEAT)
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .anisotropy_enable(false)
        .unnormalized_coordinates(false);

    let sampler = unsafe { device.create_sampler(&sampler_info, None) }?;
    Ok(sampler)
}

fn pick_depth_format(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<vk::Format, VkError> {
    let candidates = [
        vk::Format::D32_SFLOAT,
        vk::Format::D32_SFLOAT_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
    ];

    for &format in candidates.iter() {
        let props =
            unsafe { instance.get_physical_device_format_properties(physical_device, format) };

        if props
            .optimal_tiling_features
            .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
        {
            return Ok(format);
        }
    }

    Err(VkError::SuitabilityError(
        "Failed to find suitable depth format!",
    ))
}
