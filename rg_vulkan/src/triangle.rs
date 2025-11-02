use cgmath::{Deg, point3, vec2, vec3};
use log::error;
use vulkanalia::{
    Device,
    vk::{
        self, CommandBuffer, DescriptorSet, DeviceSize, DeviceV1_0, Handle, HasBuilder,
    },
};

use crate::{
    error::{VkError, to_generic},
    instance::VkInstance,
    pipeline::create_shader_module,
    types::Mat4,
    uniform::UniformBufferObject,
    vertex::Vertex,
};

#[rustfmt::skip]
static VERTICES: [Vertex; 4] = [
    Vertex::new(vec2(-0.5, -0.5), vec3(1.0, 0.0, 0.0)),
    Vertex::new(vec2(0.5, -0.5), vec3(0.0, 1.0, 0.0)),
    Vertex::new(vec2(0.5, 0.5), vec3(0.0, 0.0, 1.0)),
    Vertex::new(vec2(-0.5, 0.5), vec3(1.0, 1.0, 1.0)),
];
const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

#[derive(Debug, Default)]
pub struct Triangle {
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    uniform_buffers: Vec<vk::Buffer>,
    uniform_buffers_memory: Vec<vk::DeviceMemory>,
}

impl Triangle {
    pub fn new(instance: &VkInstance) -> Result<Self, VkError> {
        let mut result = Self::default();
        result.init_pipeline(instance)?;
        result.init_vertex_buffer(instance)?;
        result.init_index_buffer(instance)?;
        result.init_uniform_buffers(instance)?;

        Ok(result)
    }

    fn init_pipeline(&mut self, instance: &VkInstance) -> Result<(), VkError> {
        let vert = include_bytes!("../../base/resources/shaders/shader.vert.spv");
        let frag = include_bytes!("../../base/resources/shaders/shader.frag.spv");

        let vert_shader_module = create_shader_module(&instance.device, &vert[..])?;
        let frag_shader_module = create_shader_module(&instance.device, &frag[..])?;

        let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_shader_module)
            .name(b"main\0");

        let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_shader_module)
            .name(b"main\0");

        let binding_descriptions = &[Vertex::binding_description()];
        let attribute_descriptions = Vertex::attribute_descriptions();
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(instance.swapchain.extent.width as f32)
            .height(instance.swapchain.extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(instance.swapchain.extent);

        let viewports = &[viewport];
        let scissors = &[scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(viewports)
            .scissors(scissors);

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::_1);

        let attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(false);

        let attachments = &[attachment];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let layouts = &[instance.descriptor_set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(layouts);
        let layout = unsafe { instance.device.create_pipeline_layout(&layout_info, None) }?;

        let stages = &[vert_stage, frag_stage];
        let info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .layout(layout)
            .render_pass(instance.swapchain.render_pass)
            .subpass(0);

        let result = unsafe {
            instance
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
        }?;

        if result.0.is_empty() {
            error!("No pipeline in result!");
            return Err(to_generic("No pipeline in result!"));
        }

        let pipeline = result.0[0];

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

        self.layout = layout;
        self.pipeline = pipeline;

        Ok(())
    }

    fn init_vertex_buffer(&mut self, instance: &VkInstance) -> Result<(), VkError> {
        let size = (size_of::<Vertex>() * VERTICES.len()) as u64;

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

        self.vertex_buffer = vertex_buffer;
        self.vertex_buffer_memory = vertex_buffer_memory;

        instance.copy_buffer(staging_buffer, vertex_buffer, size)?;

        unsafe {
            instance.device.destroy_buffer(staging_buffer, None);
            instance.device.free_memory(staging_buffer_memory, None);
        }

        Ok(())
    }

    fn init_index_buffer(&mut self, instance: &VkInstance) -> Result<(), VkError> {
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

        self.index_buffer = index_buffer;
        self.index_buffer_memory = index_buffer_memory;

        instance.copy_buffer(staging_buffer, index_buffer, size)?;

        unsafe {
            instance.device.destroy_buffer(staging_buffer, None);
            instance.device.free_memory(staging_buffer_memory, None);
        }

        Ok(())
    }

    fn init_uniform_buffers(&mut self, instance: &VkInstance) -> Result<(), VkError> {
        self.uniform_buffers.clear();
        self.uniform_buffers_memory.clear();

        for _ in 0..instance.swapchain.images.len() {
            let (uniform_buffer, uniform_buffer_memory) = instance.create_buffer(
                size_of::<UniformBufferObject>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            )?;

            self.uniform_buffers.push(uniform_buffer);
            self.uniform_buffers_memory.push(uniform_buffer_memory);
        }

        Ok(())
    }

    pub fn update_uniform_buffer(
        &self,
        instance: &VkInstance,
        image_index: usize,
        time: f32,
        ratio: f32,
    ) -> Result<(), VkError> {
        //let time = self.start.elapsed().as_secs_f32();

        let model = Mat4::from_axis_angle(vec3(0.0, 0.0, 1.0), Deg(90.0) * time);

        let view = Mat4::look_at_rh(
            point3::<f32>(2.0, 2.0, 2.0),
            point3::<f32>(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, 1.0),
        );

        let mut proj = cgmath::perspective(
            Deg(45.0),
            ratio, //self.swapchain.extent.width as f32 / self.swapchain.extent.height as f32,
            0.1,
            10.0,
        );

        proj[1][1] *= -1.0; // OGL legacy)

        let ubo = UniformBufferObject { model, view, proj };
        let buf_memory = self.uniform_buffers_memory[image_index];

        instance.copy_memory(
            buf_memory,
            0,
            size_of::<UniformBufferObject>() as DeviceSize,
            vk::MemoryMapFlags::empty(),
            &ubo,
            1,
        )?;

        Ok(())
    }

    pub fn update_descriptor_sets(&mut self, instance: &VkInstance) -> Result<(), VkError> {
        assert_eq!(self.uniform_buffers.len(), instance.descriptor_sets.len());

        for i in 0..self.uniform_buffers.len() {
            let info = vk::DescriptorBufferInfo::builder()
                .buffer(self.uniform_buffers[i])
                .offset(0)
                .range(size_of::<UniformBufferObject>() as u64);

            let buffer_info = &[info];
            let ubo_write = vk::WriteDescriptorSet::builder()
                .dst_set(instance.descriptor_sets[i])
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(buffer_info);

            unsafe {
                instance
                    .device
                    .update_descriptor_sets(&[ubo_write], &[] as &[vk::CopyDescriptorSet])
            };
        }

        Ok(())
    }

    pub fn draw_to_buffer(
        &mut self,
        instance: &VkInstance,
        image_index: usize,
        command_buffer: vk::CommandBuffer
    ) -> Result<(), VkError> {
        let info = vk::CommandBufferBeginInfo::builder();

        unsafe { instance.device.begin_command_buffer(command_buffer, &info) }?;

        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(instance.swapchain.extent);

        let color_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.1, 1.0],
            },
        };

        let clear_values = &[color_clear_value];
        let info = vk::RenderPassBeginInfo::builder()
            .render_pass(instance.swapchain.render_pass)
            .framebuffer(instance.swapchain.framebuffers[image_index])
            .render_area(render_area)
            .clear_values(clear_values);

        unsafe {
            instance.device.cmd_begin_render_pass(command_buffer, &info, vk::SubpassContents::INLINE);
            self.render(&instance.device, &command_buffer, instance.descriptor_sets[image_index])?;
            instance.device.cmd_end_render_pass(command_buffer);
            instance.device.end_command_buffer(command_buffer)?;
        }
        Ok(())
    }

    fn render(
        &mut self,
        device: &Device,
        command_buffer: &CommandBuffer,
        descriptor_set: DescriptorSet,
    ) -> Result<(), VkError> {
        unsafe {
            device.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
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
                self.layout,
                0,
                &[descriptor_set],
                &[],
            );

            device.cmd_draw_indexed(*command_buffer, INDICES.len() as u32, 1, 0, 0, 0);
        }
        Ok(())
    }

    pub fn destroy(&self, device: &Device) {
        unsafe {
            self.uniform_buffers
                .iter()
                .for_each(|b| device.destroy_buffer(*b, None));
            self.uniform_buffers_memory
                .iter()
                .for_each(|m| device.free_memory(*m, None));
            device.destroy_buffer(self.index_buffer, None);
            device.free_memory(self.index_buffer_memory, None);
            device.destroy_buffer(self.vertex_buffer, None);
            device.free_memory(self.vertex_buffer_memory, None);
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
    }
}
