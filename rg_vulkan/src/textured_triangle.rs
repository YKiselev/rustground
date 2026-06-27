use ash::Device;
use ash::vk;
use cgmath::{Deg, point3, vec2, vec3};
use log::error;
use rg_common::App;
use std::sync::Arc;

use crate::image::VkImage;
use crate::renderer::create_default_viewport_and_scissor;
use crate::vertex::Pos2Color3Tex2Vertex;
use crate::{
    error::{VkError, to_generic},
    instance::VkInstance,
    pipeline::create_shader_module,
    types::Mat4,
    uniform::UniformBufferObject,
};

#[rustfmt::skip]
static VERTICES: [Pos2Color3Tex2Vertex; 4] = [
    Pos2Color3Tex2Vertex::new(vec2(-0.5, -0.5), vec3(1.0, 0.0, 0.0), vec2(0.0, 0.0)),
    Pos2Color3Tex2Vertex::new(vec2(0.5, -0.5), vec3(0.0, 1.0, 0.0), vec2(1.0, 0.0)),
    Pos2Color3Tex2Vertex::new(vec2(0.5, 0.5), vec3(0.0, 0.0, 1.0), vec2(1.0, 1.0)),
    Pos2Color3Tex2Vertex::new(vec2(-0.5, 0.5), vec3(1.0, 1.0, 1.0), vec2(0.0, 1.0)),
];
const INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

#[derive()]
pub struct TexturedTriangle {
    app: Arc<App>,
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    uniform_buffers: Vec<vk::Buffer>,
    uniform_buffers_memory: Vec<vk::DeviceMemory>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    texture: VkImage,
}

impl TexturedTriangle {
    pub fn new(instance: &VkInstance, app: &Arc<App>) -> Result<Self, VkError> {
        let vert = include_bytes!("../../base/resources/shaders/tex-shader.vert.spv");
        let frag = include_bytes!("../../base/resources/shaders/tex-shader.frag.spv");

        let vert_shader_module = create_shader_module(&instance.device, &vert[..])?;
        let frag_shader_module = create_shader_module(&instance.device, &frag[..])?;

        let vert_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_shader_module)
            .name(c"main");

        let frag_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_shader_module)
            .name(c"main");

        let binding_descriptions = &[Pos2Color3Tex2Vertex::binding_description()];
        let attribute_descriptions = Pos2Color3Tex2Vertex::attribute_descriptions();
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let (viewport, scissor) = create_default_viewport_and_scissor(instance.swapchain.extent);
        let viewports = &[viewport];
        let scissors = &[scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(viewports)
            .scissors(scissors);

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false);

        let attachments = &[attachment];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let descriptor_set_layout = create_descriptor_set_layout(&instance.device)?;
        let layouts = &[descriptor_set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(layouts);
        let layout = unsafe { instance.device.create_pipeline_layout(&layout_info, None) }?;
        let dynamic_states = [
            ash::vk::DynamicState::VIEWPORT,
            ash::vk::DynamicState::SCISSOR,
        ];
        let dynamic_state =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let stages = &[vert_stage, frag_stage];
        let info = vk::GraphicsPipelineCreateInfo::default()
            .stages(stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .dynamic_state(&dynamic_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .layout(layout)
            .render_pass(instance.swapchain.render_pass)
            .subpass(0);

        let infos = [info];
        let mut result = unsafe {
            instance.device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                infos.as_slice(),
                None,
            )
        }
        .unwrap();

        if result.is_empty() {
            error!("No pipeline in result!");
            return Err(to_generic("No pipeline in result!"));
        }

        let pipeline = result.remove(0);

        unsafe {
            instance
                .device
                .destroy_shader_module(vert_shader_module, None)
        };
        unsafe {
            instance
                .device
                .destroy_shader_module(frag_shader_module, None)
        };

        let descriptor_set_count = instance.swapchain.images.len();
        let descriptor_pool = create_descriptor_pool(&instance.device, descriptor_set_count)?;
        let descriptor_sets = create_descriptor_sets(
            &instance.device,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_set_count,
        )?;
        let texture = instance.create_texture_image(&app.files)?;
        let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(instance)?;
        let (index_buffer, index_buffer_memory) = create_index_buffer(instance)?;
        let (uniform_buffers, uniform_buffers_memory) = create_uniform_buffers(instance)?;

        Ok(Self {
            app: Arc::clone(app),
            layout,
            pipeline,
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            uniform_buffers,
            uniform_buffers_memory,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_sets,
            texture,
        })
    }

    pub fn update_uniform_buffer(
        &self,
        instance: &VkInstance,
        image_index: usize,
        time: f32,
        ratio: f32,
    ) -> Result<(), VkError> {
        let model = Mat4::from_axis_angle(vec3(0.0, 0.0, 1.0), Deg(90.0) * time);

        let view = Mat4::look_at_rh(
            point3::<f32>(2.0, 2.0, 2.0),
            point3::<f32>(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, 1.0),
        );

        let mut proj = cgmath::perspective(Deg(45.0), ratio, 0.1, 10.0);

        proj.y.y *= -1.0; // OGL legacy)

        let ubo = UniformBufferObject { model, view, proj };
        let buf_memory = self.uniform_buffers_memory[image_index];

        instance.copy_memory(
            buf_memory,
            0,
            size_of::<UniformBufferObject>() as vk::DeviceSize,
            vk::MemoryMapFlags::empty(),
            &ubo,
            1,
        )?;

        Ok(())
    }

    pub fn update_descriptor_sets(&mut self, instance: &VkInstance) -> Result<(), VkError> {
        if self.uniform_buffers.len() != instance.swapchain.images.len() {
            self.destroy_uniform_buffers(&instance.device);
            let (uniform_buffers, uniform_buffers_memory) = create_uniform_buffers(instance)?;
            self.uniform_buffers = uniform_buffers;
            self.uniform_buffers_memory = uniform_buffers_memory;
        }

        for i in 0..self.uniform_buffers.len() {
            let info = vk::DescriptorBufferInfo::default()
                .buffer(self.uniform_buffers[i])
                .offset(0)
                .range(size_of::<UniformBufferObject>() as u64);

            let descriptor_set = self.descriptor_sets[i];
            let buffer_info = &[info];
            let ubo_write = vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(buffer_info);

            let sampler_info = [vk::DescriptorImageInfo::default().sampler(instance.sampler)];
            let sampler_write = vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(&sampler_info);

            let image_info = [vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(self.texture.view)];
            let image_write = vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(&image_info);

            unsafe {
                instance.device.update_descriptor_sets(
                    &[ubo_write, sampler_write, image_write],
                    &[] as &[vk::CopyDescriptorSet],
                )
            };
        }

        Ok(())
    }

    pub fn draw_to_buffer(
        &mut self,
        instance: &VkInstance,
        image_index: usize,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), VkError> {
        let image = &instance.swapchain.images[image_index];
        let device = &instance.device;
        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            let buffers = [self.vertex_buffer];
            let offsets = [0];
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                buffers.as_slice(),
                offsets.as_slice(),
            );
            device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer,
                0,
                vk::IndexType::UINT16,
            );
            let descriptor_sets = [self.descriptor_sets[image_index]];
            let dyn_offsets = [];
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.layout,
                0,
                descriptor_sets.as_slice(),
                dyn_offsets.as_slice(),
            );

            device.cmd_draw_indexed(command_buffer, INDICES.len() as u32, 1, 0, 0, 0);
        }
        Ok(())
    }

    pub fn destroy_uniform_buffers(&mut self, device: &Device) {
        unsafe {
            self.uniform_buffers
                .iter()
                .for_each(|b| device.destroy_buffer(*b, None));
            self.uniform_buffers.clear();
            self.uniform_buffers_memory
                .iter()
                .for_each(|m| device.free_memory(*m, None));
            self.uniform_buffers_memory.clear();
        }
    }

    pub fn destroy(&mut self, device: &Device) {
        self.destroy_uniform_buffers(device);
        unsafe {
            self.texture.destroy(device);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            device.destroy_buffer(self.index_buffer, None);
            device.free_memory(self.index_buffer_memory, None);
            device.destroy_buffer(self.vertex_buffer, None);
            device.free_memory(self.vertex_buffer_memory, None);
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
    }
}

fn create_descriptor_set_layout(device: &Device) -> Result<vk::DescriptorSetLayout, VkError> {
    let ubo_binding = vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX);
    let texture_sampler_binding = vk::DescriptorSetLayoutBinding::default()
        .binding(1)
        .descriptor_type(vk::DescriptorType::SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);
    let texture_layout_binding = vk::DescriptorSetLayoutBinding::default()
        .binding(2)
        .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let bindings = &[ubo_binding, texture_sampler_binding, texture_layout_binding];
    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(bindings);
    let layout = unsafe { device.create_descriptor_set_layout(&info, None) }?;

    Ok(layout)
}

fn create_descriptor_pool(device: &Device, count: usize) -> Result<vk::DescriptorPool, VkError> {
    let ubo_size = vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(count as u32);
    let sampler_size = vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::SAMPLER)
        .descriptor_count(count as u32);
    let image_size = vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::SAMPLED_IMAGE)
        .descriptor_count(count as u32);

    let pool_sizes = &[ubo_size, sampler_size, image_size];
    let info = vk::DescriptorPoolCreateInfo::default()
        .pool_sizes(pool_sizes)
        .max_sets(count as u32);

    Ok(unsafe { device.create_descriptor_pool(&info, None) }?)
}

fn create_descriptor_sets(
    device: &Device,
    layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    count: usize,
) -> Result<Vec<vk::DescriptorSet>, VkError> {
    let layouts = vec![layout; count];
    let info = vk::DescriptorSetAllocateInfo::default()
        .descriptor_pool(pool)
        .set_layouts(&layouts);

    unsafe { device.allocate_descriptor_sets(&info) }.map_err(|e| VkError::VkErrorCode(e))
}

fn create_vertex_buffer(instance: &VkInstance) -> Result<(vk::Buffer, vk::DeviceMemory), VkError> {
    let size = (size_of::<Pos2Color3Tex2Vertex>() * VERTICES.len()) as u64;

    let (staging_buffer, staging_buffer_memory) = instance.create_buffer(
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    instance.copy_memory(
        staging_buffer_memory,
        0,
        size,
        vk::MemoryMapFlags::empty(),
        VERTICES.as_ptr(),
        VERTICES.len(),
    )?;

    let (vertex_buffer, vertex_buffer_memory) = instance.create_buffer(
        size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    instance.copy_buffer(staging_buffer, vertex_buffer, size)?;

    unsafe {
        instance.device.destroy_buffer(staging_buffer, None);
        instance.device.free_memory(staging_buffer_memory, None);
    }

    Ok((vertex_buffer, vertex_buffer_memory))
}

fn create_index_buffer(instance: &VkInstance) -> Result<(vk::Buffer, vk::DeviceMemory), VkError> {
    // Create (staging)

    let size = (size_of::<u16>() * INDICES.len()) as u64;

    let (staging_buffer, staging_buffer_memory) = instance.create_buffer(
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    // Copy (staging)
    instance.copy_memory(
        staging_buffer_memory,
        0,
        size,
        vk::MemoryMapFlags::empty(),
        INDICES.as_ptr(),
        INDICES.len(),
    )?;

    // Create (index)

    let (index_buffer, index_buffer_memory) = instance.create_buffer(
        size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    instance.copy_buffer(staging_buffer, index_buffer, size)?;

    unsafe {
        instance.device.destroy_buffer(staging_buffer, None);
        instance.device.free_memory(staging_buffer_memory, None);
    }

    Ok((index_buffer, index_buffer_memory))
}

fn create_uniform_buffers(
    instance: &VkInstance,
) -> Result<(Vec<vk::Buffer>, Vec<vk::DeviceMemory>), VkError> {
    let mut uniform_buffers = Vec::new();
    let mut uniform_buffers_memory = Vec::new();
    for _ in 0..instance.swapchain.images.len() {
        let (uniform_buffer, uniform_buffer_memory) = instance.create_buffer(
            size_of::<UniformBufferObject>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;
        uniform_buffers.push(uniform_buffer);
        uniform_buffers_memory.push(uniform_buffer_memory);
    }
    Ok((uniform_buffers, uniform_buffers_memory))
}
