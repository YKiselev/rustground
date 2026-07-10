use ash::Device;
use ash::vk;
use log::error;
use rg_common::App;
use rg_common::load_bytes;
use std::sync::Arc;

use crate::buffer::VkBuffer;
use crate::instance::MAX_FRAMES_IN_FLIGHT;
use crate::renderer::create_default_viewport_and_scissor;
use crate::types::Vec2;
use crate::types::Vec3;
use crate::types::Vec4;
use crate::vertex::vertex_input_descriptions;
use crate::{
    error::{VkError, to_generic},
    instance::VkInstance,
    pipelines::shader::create_shader_module,
    types::Mat4,
    uniform::UniformBufferObject,
    vertex::Pos2Color4Vertex,
};

#[rustfmt::skip]
static VERTICES: [Pos2Color4Vertex; 4] = [
    Pos2Color4Vertex::new(Vec2::new(-0.5, -0.5), Vec4::new(1.0, 0.0, 0.0,1.0)),
    Pos2Color4Vertex::new(Vec2::new(0.5, -0.5), Vec4::new(0.0, 1.0, 0.0,1.0)),
    Pos2Color4Vertex::new(Vec2::new(0.5, 0.5), Vec4::new(0.0, 0.0, 1.0,1.0)),
    Pos2Color4Vertex::new(Vec2::new(-0.5, 0.5), Vec4::new(1.0, 1.0, 1.0,1.0)),
];
const INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

#[derive()]
pub struct Triangle {
    app: Arc<App>,
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub vertex_buffer: VkBuffer,
    pub index_buffer: VkBuffer,
    uniform_buffers: Vec<VkBuffer>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
}

impl Triangle {
    pub fn new(instance: &VkInstance, app: &Arc<App>) -> Result<Self, VkError> {
        let vert = app.load_resource("shaders/shader.vert.spv", &load_bytes, ())?;
        let frag = app.load_resource("shaders/shader.frag.spv", &load_bytes, ())?;

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

        let (binding_description, attribute_descriptions) =
            vertex_input_descriptions::<Pos2Color4Vertex>();
        let binding_descriptions = [binding_description];
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&binding_descriptions)
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

        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS);

        let info = vk::GraphicsPipelineCreateInfo::default()
            .stages(stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .dynamic_state(&dynamic_state)
            .depth_stencil_state(&depth_stencil_state)
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

        let descriptor_set_count = MAX_FRAMES_IN_FLIGHT;
        let descriptor_pool = create_descriptor_pool(&instance.device, descriptor_set_count)?;
        let descriptor_sets = create_descriptor_sets(
            &instance.device,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_set_count,
        )?;
        let vertex_buffer = VkBuffer::vertex(instance, VERTICES.as_ptr(), VERTICES.len())?;
        let index_buffer = VkBuffer::index(instance, INDICES.as_ptr(), INDICES.len())?;
        let uniform_buffers = create_uniform_buffers(instance)?;
        let mut result = Self {
            app: Arc::clone(app),
            layout,
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffers,
            descriptor_sets,
            descriptor_pool,
            descriptor_set_layout,
        };
        result.update_descriptor_sets(instance)?;
        Ok(result)
    }

    pub fn update_uniform_buffer(
        &self,
        instance: &VkInstance,
        image_index: usize,
        time: f32,
        ratio: f32,
    ) -> Result<(), VkError> {
        let mut model = Mat4::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 90.0f32.to_radians() * time);
        let trans = Mat4::from_translation(Vec3::new(0.0, 0.0, -0.15));

        model = trans * model;

        let view = glam::camera::lh::view::look_at_mat4(
            Vec3::new(2.0, 2.0, 2.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        );

        let mut proj = glam::camera::lh::proj::vulkan::perspective(45.0f32.to_radians(), ratio, 0.1, 10.0);

        //proj.y.y *= -1.0; // OGL legacy)

        let ubo = UniformBufferObject { model, view, proj };
        let buf_memory = self.uniform_buffers[image_index].memory;

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

    pub fn on_swapchain_recreated(&mut self, instance: &VkInstance) -> Result<(), VkError> {
        Ok(())
    }

    fn update_descriptor_sets(&mut self, instance: &VkInstance) -> Result<(), VkError> {
        for i in 0..self.uniform_buffers.len() {
            let info = vk::DescriptorBufferInfo::default()
                .buffer(self.uniform_buffers[i].buffer)
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
        frame_index: usize,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), VkError> {
        let device = &instance.device;
        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            let buffers = [self.vertex_buffer.buffer];
            let offsets = [0];
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                buffers.as_slice(),
                offsets.as_slice(),
            );
            device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer.buffer,
                0,
                vk::IndexType::UINT16,
            );
            let descriptor_sets = [self.descriptor_sets[frame_index]];
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
        self.uniform_buffers.iter().for_each(|b| b.destroy(device));
        self.uniform_buffers.clear();
    }

    pub fn destroy(&mut self, device: &Device) {
        self.destroy_uniform_buffers(device);
        unsafe {
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.index_buffer.destroy(device);
            self.vertex_buffer.destroy(device);
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

    let bindings = &[ubo_binding];
    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(bindings);
    let layout = unsafe { device.create_descriptor_set_layout(&info, None) }?;
    Ok(layout)
}

fn create_descriptor_pool(device: &Device, count: usize) -> Result<vk::DescriptorPool, VkError> {
    let ubo_size = vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(count as u32);

    let pool_sizes = &[ubo_size];
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

fn create_uniform_buffers(instance: &VkInstance) -> Result<Vec<VkBuffer>, VkError> {
    let uniform_buffers = (0..MAX_FRAMES_IN_FLIGHT)
        .into_iter()
        .map(|_| VkBuffer::uniform::<UniformBufferObject>(instance))
        .collect::<Result<Vec<VkBuffer>, VkError>>()?;

    Ok(uniform_buffers)
}
