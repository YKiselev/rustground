use ash::Device;
use ash::vk;
use log::error;
use rg_common::App;
use rg_common::load_bytes;
use rg_common::load_deserializable;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

use crate::buffer::VkBuffer;
use crate::dyn_buffer::VkDynamicBuffer;
use crate::font::VkFontAtlas;
use crate::loaders::FontAtlasLoaderContext;
use crate::loaders::load_font_atlas;
use crate::renderer::create_default_viewport_and_scissor;
use crate::types::Vec2i16;
use crate::types::Vec4i16;
use crate::vertex::GlyphInstance;
use crate::vertex::vertex_input_descriptions;
use crate::{
    error::{VkError, to_generic},
    instance::VkInstance,
    pipelines::shader::create_shader_module,
    types::Mat4,
};

///
/// UI pipeline config
///
#[derive(Serialize, Deserialize)]
struct Config {
    atlas_width: u32,
    atlas_height: u32,
    vertex_shader: String,
    fragment_schader: String,
}

///
///
///
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UniformBufferObject {
    pub proj: Mat4,
}

///
/// Frame objects
///
struct FrameObjects {
    vertex_buffer: VkDynamicBuffer,
    uniform_buffer: VkBuffer,
    total_glyph_count: u32,
    glyph_buffer: Vec<GlyphInstance>,
}

const MAX_GLYPH_BATCH: usize = 200;

impl FrameObjects {
    fn new(instance: &VkInstance) -> Result<Self, VkError> {
        let vertex_buffer = VkDynamicBuffer::vertex::<GlyphInstance>(instance, 2048)?;
        let uniform_buffer = VkBuffer::uniform::<UniformBufferObject>(instance)?;
        Ok(Self {
            vertex_buffer,
            uniform_buffer,
            total_glyph_count: 0,
            glyph_buffer: Vec::with_capacity(MAX_GLYPH_BATCH),
        })
    }

    fn destroy(&self, device: &ash::Device) {
        self.vertex_buffer.destroy(device);
        self.uniform_buffer.destroy(device);
    }
}
///
/// UI pipeline
///
#[derive()]
pub struct UiPipeline {
    app: Arc<App>,
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    frame_objects: Vec<FrameObjects>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    font_atlas: VkFontAtlas,
}

impl UiPipeline {
    pub fn new(instance: &VkInstance, app: &Arc<App>) -> Result<Self, VkError> {
        let config = app.load_resource(
            "configs/ui-pipeline.toml",
            &load_deserializable::<Config>,
            (),
        )?;

        // Load fonts
        let atlas_size = vk::Extent2D {
            width: config.atlas_width,
            height: config.atlas_height,
        };
        let ctx = FontAtlasLoaderContext::new(instance, app, atlas_size);
        let font_atlas = app.load_resource("configs/ui-pipeline.toml", &load_font_atlas, &ctx)?;

        let vert = app.load_resource(config.vertex_shader, &load_bytes, ())?;
        let frag = app.load_resource(config.fragment_schader, &load_bytes, ())?;
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
            vertex_input_descriptions::<GlyphInstance>();
        let binding_descriptions = [binding_description];
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_STRIP)
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
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_write_mask(vk::ColorComponentFlags::RGBA);

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

        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(vk::CompareOp::ALWAYS);

        let stages = &[vert_stage, frag_stage];
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
                .destroy_shader_module(vert_shader_module, None);
            instance
                .device
                .destroy_shader_module(frag_shader_module, None);
        }

        let descriptor_set_count = instance.swapchain.images.len();
        let descriptor_pool = create_descriptor_pool(&instance.device, descriptor_set_count)?;
        let descriptor_sets = create_descriptor_sets(
            &instance.device,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_set_count,
        )?;

        let frame_objects = instance
            .swapchain
            .images
            .iter()
            .map(|_| FrameObjects::new(instance))
            .collect::<Result<Vec<FrameObjects>, VkError>>()?;
        let mut result = Self {
            app: Arc::clone(app),
            layout,
            pipeline,
            frame_objects,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_sets,
            font_atlas,
        };
        result.update_descriptor_sets(instance)?;

        Ok(result)
    }

    pub fn draw_text<S>(&mut self, x: i32, y: i32, text: S, font: S)
    where
        S: AsRef<str>,
    {
        if let Some(font) = self.font_atlas.fonts.get(font.as_ref()) {
            let mut x = x;
            let mut y = y;
            for ch in text.as_ref().chars() {
                if let Some(glyph) = font.get(&ch) {
                    let gx = (x + glyph.offset.x as i32) as i16;
                    let gy = (y + glyph.offset.y as i32) as i16;
                    let gw = glyph.width as i16;
                    let gh = glyph.height as i16;
                    let u = (glyph.uv_min.x * 32767.0) as i16;
                    let v = (glyph.uv_min.y * 32767.0) as i16;
                    let size = glyph.uv_max - glyph.uv_min;
                    let uw = (size.x * 32767.0) as i16;
                    let vh = (size.y * 32767.0) as i16;

                    let g = GlyphInstance {
                        pos: Vec2i16 { x: gx, y: gy },
                        size: Vec2i16 { x: gw, y: gh },
                        color: Vec4i16 {
                            x: 32767,
                            y: 32767,
                            z: 32767,
                            w: 32767,
                        },
                        uv: Vec2i16 { x: u, y: v },
                        uv_size: Vec2i16 { x: uw, y: vh },
                        layer_index: glyph.layer_index,
                    };

                    x += glyph.h_advance as i32;
                }
            }
        }
    }

    pub fn update_uniform_buffer(
        &self,
        instance: &VkInstance,
        frame_index: usize,
    ) -> Result<(), VkError> {
        let mut proj = cgmath::ortho(0.0, 800.0, 600.0, 0.0, -1.0, 1.0);

        proj.y.y *= -1.0; // OGL legacy)

        let ubo = UniformBufferObject { proj };
        let buf_memory = self.frame_objects[frame_index].uniform_buffer.memory;

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
        self.update_descriptor_sets(instance)
    }

    fn update_descriptor_sets(&mut self, instance: &VkInstance) -> Result<(), VkError> {
        for i in 0..self.frame_objects.len() {
            let info = vk::DescriptorBufferInfo::default()
                .buffer(self.frame_objects[i].uniform_buffer.buffer)
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

            let sampler_info = [vk::DescriptorImageInfo::default()
                .sampler(instance.sampler)
                .image_view(self.font_atlas.image.view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
            let sampler_write = vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&sampler_info);

            let writes = [ubo_write, sampler_write];
            unsafe { instance.device.update_descriptor_sets(&writes, &[]) };
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

            let buffers = [self.frame_objects[frame_index].vertex_buffer.buffer];
            let offsets = [0];
            device.cmd_bind_vertex_buffers(command_buffer, 0, &buffers, &offsets);
            let descriptor_sets = [self.descriptor_sets[frame_index]];
            let dyn_offsets = [];
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.layout,
                0,
                &descriptor_sets,
                &dyn_offsets,
            );

            device.cmd_draw(command_buffer, 4, 1, 0, 0);
        }
        Ok(())
    }

    pub fn destroy(&mut self, device: &Device) {
        self.frame_objects
            .iter()
            .for_each(|obj| obj.destroy(device));
        self.frame_objects.clear();
        unsafe {
            self.font_atlas.destroy(device);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
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
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let bindings = &[ubo_binding, texture_sampler_binding];
    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(bindings);
    let layout = unsafe { device.create_descriptor_set_layout(&info, None) }?;

    Ok(layout)
}

fn create_descriptor_pool(device: &Device, count: usize) -> Result<vk::DescriptorPool, VkError> {
    let ubo_size = vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(count as u32);
    let sampler_size = vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(count as u32);

    let pool_sizes = &[ubo_size, sampler_size];
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
