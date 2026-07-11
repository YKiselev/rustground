use ash::Device;
use ash::vk;
use glam::Mat4;
use log::error;
use log::warn;
use rg_common::App;
use rg_common::Color;
use rg_common::load_bytes;
use rg_common::load_deserializable;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

use crate::buffer::VkBuffer;
use crate::context::MAX_FRAMES_IN_FLIGHT;
use crate::dyn_buffer::VkDynamicBuffer;
use crate::font::VkFontAtlas;
use crate::loaders::FontAtlasLoaderContext;
use crate::loaders::load_font_atlas;
use crate::pipelines::shader::ShaderStages;
use crate::pipelines::shader::ShaderStagesBuilder;
use crate::pipelines::ui::text::ToGlyphInstance;
use crate::renderer::create_default_viewport_and_scissor;
use crate::vertex::GlyphInstance;
use crate::vertex::vertex_input_descriptions;
use crate::{
    context::VkContext,
    error::{VkError, to_generic},
    pipelines::shader::create_shader_module,
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
    descriptor_set: vk::DescriptorSet,
}

const DEFAULT_GLYPH_BUFFER_SIZE: usize = 20_000;
const MAX_GLYPHS_PER_FRAME: usize = 100_000;

impl FrameObjects {
    fn new(instance: &VkContext, descriptor_set: vk::DescriptorSet) -> Result<Self, VkError> {
        let vertex_buffer =
            VkDynamicBuffer::vertex::<GlyphInstance>(instance, MAX_GLYPHS_PER_FRAME)?;
        let uniform_buffer = VkBuffer::uniform::<UniformBufferObject>(instance)?;
        Ok(Self {
            vertex_buffer,
            uniform_buffer,
            descriptor_set,
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
    font_atlas: VkFontAtlas,
    frame_index: Option<usize>,
    glyph_buffer: Vec<GlyphInstance>,
}

impl UiPipeline {
    pub fn new(context: &VkContext, app: &Arc<App>, scale_factor: f64) -> Result<Self, VkError> {
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
        let ctx = FontAtlasLoaderContext::new(context, app, atlas_size, scale_factor);
        let font_atlas = app.load_resource("configs/ui-pipeline.toml", &load_font_atlas, &ctx)?;

        let vert = app.load_resource(config.vertex_shader, &load_bytes, ())?;
        let frag = app.load_resource(config.fragment_schader, &load_bytes, ())?;
        let mut shader_stages = ShaderStages::builder()
            .with_vertex_shader(&vert)
            .with_fragment_shader(&frag)
            .build(context)?;

        let (binding_description, attribute_descriptions) =
            vertex_input_descriptions::<GlyphInstance>();
        let binding_descriptions = [binding_description];
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_STRIP)
            .primitive_restart_enable(false);

        let (viewport, scissor) = create_default_viewport_and_scissor(context.swapchain.extent);
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
            .front_face(vk::FrontFace::CLOCKWISE)
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

        let descriptor_set_layout = create_descriptor_set_layout(&context.device)?;
        let layouts = &[descriptor_set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(layouts);
        let layout = unsafe { context.device.create_pipeline_layout(&layout_info, None) }?;
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

        let info = vk::GraphicsPipelineCreateInfo::default()
            .stages(shader_stages.stages())
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .dynamic_state(&dynamic_state)
            .depth_stencil_state(&depth_stencil_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .layout(layout)
            .render_pass(context.swapchain.render_pass)
            .subpass(0);

        let infos = [info];
        let mut result = unsafe {
            context.device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                infos.as_slice(),
                None,
            )
        }
        .map_err(|(_, e)| VkError::VkErrorCode(e))?;

        if result.is_empty() {
            error!("No pipeline in result!");
            return Err(to_generic("No pipeline in result!"));
        }

        let pipeline = result.remove(0);

        shader_stages.destroy(&context.device);

        let descriptor_pool = create_descriptor_pool(&context.device, MAX_FRAMES_IN_FLIGHT)?;
        let descriptor_sets = create_descriptor_sets(
            &context.device,
            descriptor_set_layout,
            descriptor_pool,
            MAX_FRAMES_IN_FLIGHT,
        )?;

        let frame_objects = descriptor_sets
            .into_iter()
            .map(|ds| FrameObjects::new(context, ds))
            .collect::<Result<Vec<FrameObjects>, VkError>>()?;

        let mut result = Self {
            app: Arc::clone(app),
            layout,
            pipeline,
            frame_objects,
            descriptor_set_layout,
            descriptor_pool,
            font_atlas,
            frame_index: None,
            glyph_buffer: Vec::with_capacity(DEFAULT_GLYPH_BUFFER_SIZE),
        };
        
        result.update_descriptor_sets(context)?;

        Ok(result)
    }

    pub fn draw_text<S>(&mut self, x: i32, y: i32, text: S, color: Color)
    where
        S: AsRef<str>,
    {
        let font = "console";
        if let Some(font) = self.font_atlas.fonts.get(font) {
            let mut x = x;
            let mut y = y + font.height as i32;
            for ch in text.as_ref().chars() {
                if let Some(glyph) = font.get(ch) {
                    let mut g = glyph.to_glyph_instance(x, y);
                    g.color = color.into();
                    let buf = &mut self.glyph_buffer;
                    if buf.len() >= MAX_GLYPHS_PER_FRAME {
                        warn!("Maximim glyphs per frame reached ({})", buf.len());
                        return;
                    } else {
                        buf.push(g);
                    }
                    x += glyph.h_advance as i32;
                }
            }
        } else {
            warn!("Font not found: {}", font);
        }
    }

    fn update_uniform_buffer(
        &self,
        instance: &VkContext,
        frame_index: usize,
    ) -> Result<(), VkError> {
        let ext = instance.swapchain.extent;
        let proj = glam::camera::rh::proj::vulkan::orthographic(
            0.0,
            ext.width as f32,
            0.0,
            -(ext.height as f32),
            -1.0,
            1.0,
        );

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

    pub fn on_swapchain_recreated(&mut self, instance: &VkContext) -> Result<(), VkError> {
        self.update_descriptor_sets(instance)
    }

    fn update_descriptor_sets(&mut self, instance: &VkContext) -> Result<(), VkError> {
        for (i, frame_obj) in self.frame_objects.iter().enumerate() {
            let info = vk::DescriptorBufferInfo::default()
                .buffer(self.frame_objects[i].uniform_buffer.buffer)
                .offset(0)
                .range(size_of::<UniformBufferObject>() as u64);

            let descriptor_set = frame_obj.descriptor_set;
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

    pub fn begin_frame(
        &mut self,
        instance: &VkContext,
        frame_index: usize,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), VkError> {
        self.frame_index = Some(frame_index);

        let device = &instance.device;
        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            let descriptor_sets = [self.frame_objects[frame_index].descriptor_set];
            let dyn_offsets = [];

            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.layout,
                0,
                &descriptor_sets,
                &dyn_offsets,
            );
        }

        let _ = self.update_uniform_buffer(instance, frame_index)?;

        self.draw_text(0, 0, "Hello, Vulkan user!", Color::RED);
        self.draw_text(50, 50, "Hello, Vulkan user!", Color::LIGHT_BLUE);
        self.draw_text(100, 100, "Hello, Vulkan user!", Color::LIGHT_GREEN);

        Ok(())
    }

    pub fn end_frame(
        &mut self,
        instance: &VkContext,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), VkError> {
        if let Some(frame_index) = self.frame_index.take() {
            let frame_obj = &self.frame_objects[frame_index];
            frame_obj
                .vertex_buffer
                .copy_from(self.glyph_buffer.as_ptr(), self.glyph_buffer.len());
            let instance_count = self.glyph_buffer.len() as u32;
            let vertex_count = 4;

            let buffers = [frame_obj.vertex_buffer.buffer];
            let offsets = [0];

            unsafe {
                instance
                    .device
                    .cmd_bind_vertex_buffers(command_buffer, 0, &buffers, &offsets);

                if instance_count > 0 {
                    instance
                        .device
                        .cmd_draw(command_buffer, vertex_count, instance_count, 0, 0);
                }
            }
            self.glyph_buffer.clear();
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
