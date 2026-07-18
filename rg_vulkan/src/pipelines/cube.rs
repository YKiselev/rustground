use ash::Device;
use ash::vk;
use core::slice;
use log::error;
use log::warn;
use rand::Rng;
use rg_common::App;
use rg_common::load_bytes;
use rg_common::world::HyperCube;
use std::sync::Arc;

use crate::misc::buffer::VkBuffer;
use crate::misc::context::MAX_FRAMES_IN_FLIGHT;
use crate::misc::dyn_buffer::VkDynamicBuffer;
use crate::misc::image::VkImage;
use crate::misc::vertex::Vertex;
use crate::renderer::create_default_viewport_and_scissor;
use crate::types::Vec3;
use crate::{
    error::{VkError, to_generic},
    misc::context::VkContext,
    misc::uniform::UniformBufferObject,
    pipelines::shader::create_shader_module,
    types::Mat4,
};

///
/// Cube vertex
///
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Pos4Norm4Uv2Vertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub uv: [f32; 2],
}

impl Pos4Norm4Uv2Vertex {
    pub const fn new(x: f32, y: f32, z: f32, nx: f32, ny: f32, nz: f32, u: f32, v: f32) -> Self {
        Self {
            position: [x, y, z, 1.0],
            normal: [nx, ny, nz, 1.0],
            uv: [u, v],
        }
    }
}

impl Vertex for Pos4Norm4Uv2Vertex {
    fn input_binding_description(binding: u32) -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(binding)
            .stride(Self::size_in_bytes() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    fn input_attribute_descriptions(
        binding: u32,
        location: u32,
    ) -> Vec<vk::VertexInputAttributeDescription> {
        let pos = vk::VertexInputAttributeDescription::default()
            .binding(binding)
            .location(location)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(std::mem::offset_of!(Self, position) as u32);
        let normal = vk::VertexInputAttributeDescription::default()
            .binding(binding)
            .location(location + 1)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(std::mem::offset_of!(Self, normal) as u32);
        let uv = vk::VertexInputAttributeDescription::default()
            .binding(binding)
            .location(location + 2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(std::mem::offset_of!(Self, uv) as u32);
        vec![pos, normal, uv]
    }
}

///
/// 
/// 
#[rustfmt::skip]
static VERTICES: [Pos4Norm4Uv2Vertex; 24] = [
    // --- Передняя грань (нормаль Z+) ---
    Pos4Norm4Uv2Vertex::new(-0.5, -0.5,  0.5, 0.0,  0.0,  1.0,0.0, 0.0),
    Pos4Norm4Uv2Vertex::new( 0.5, -0.5,  0.5, 0.0,  0.0,  1.0,1.0, 0.0),
    Pos4Norm4Uv2Vertex::new( 0.5,  0.5,  0.5, 0.0,  0.0,  1.0,1.0, 1.0),
    Pos4Norm4Uv2Vertex::new(-0.5,  0.5,  0.5, 0.0,  0.0,  1.0,0.0, 1.0),

    // --- Задняя грань (нормаль Z-) ---
    Pos4Norm4Uv2Vertex::new( 0.5, -0.5, -0.5, 0.0,  0.0, -1.0,0.0, 0.0),
    Pos4Norm4Uv2Vertex::new(-0.5, -0.5, -0.5, 0.0,  0.0, -1.0,1.0, 0.0),
    Pos4Norm4Uv2Vertex::new(-0.5,  0.5, -0.5, 0.0,  0.0, -1.0,1.0, 1.0),
    Pos4Norm4Uv2Vertex::new( 0.5,  0.5, -0.5, 0.0,  0.0, -1.0,0.0, 1.0),

    // --- Левая грань (нормаль X-) ---
    Pos4Norm4Uv2Vertex::new(-0.5, -0.5, -0.5,-1.0,  0.0,  0.0,0.0, 0.0),
    Pos4Norm4Uv2Vertex::new(-0.5, -0.5,  0.5,-1.0,  0.0,  0.0,1.0, 0.0),
    Pos4Norm4Uv2Vertex::new(-0.5,  0.5,  0.5,-1.0,  0.0,  0.0,1.0, 1.0),
    Pos4Norm4Uv2Vertex::new(-0.5,  0.5, -0.5,-1.0,  0.0,  0.0,0.0, 1.0),

    // --- Правая грань (нормаль X+) ---
    Pos4Norm4Uv2Vertex::new( 0.5, -0.5,  0.5, 1.0,  0.0,  0.0,0.0, 0.0),
    Pos4Norm4Uv2Vertex::new( 0.5, -0.5, -0.5, 1.0,  0.0,  0.0,1.0, 0.0),
    Pos4Norm4Uv2Vertex::new( 0.5,  0.5, -0.5, 1.0,  0.0,  0.0,1.0, 1.0),
    Pos4Norm4Uv2Vertex::new( 0.5,  0.5,  0.5, 1.0,  0.0,  0.0,0.0, 1.0),

    // --- Верхняя грань (нормаль Y+) ---
    Pos4Norm4Uv2Vertex::new(-0.5,  0.5,  0.5, 0.0,  1.0,  0.0,0.0, 0.0),
    Pos4Norm4Uv2Vertex::new( 0.5,  0.5,  0.5, 0.0,  1.0,  0.0,1.0, 0.0),
    Pos4Norm4Uv2Vertex::new( 0.5,  0.5, -0.5, 0.0,  1.0,  0.0,1.0, 1.0),
    Pos4Norm4Uv2Vertex::new(-0.5,  0.5, -0.5, 0.0,  1.0, -0.0,0.0, 1.0),

    // --- Нижняя грань (нормаль Y-) ---
    Pos4Norm4Uv2Vertex::new(-0.5, -0.5, -0.5, 0.0, -1.0,  0.0,0.0, 0.0),
    Pos4Norm4Uv2Vertex::new( 0.5, -0.5, -0.5, 0.0, -1.0,  0.0,1.0, 0.0),
    Pos4Norm4Uv2Vertex::new( 0.5, -0.5,  0.5, 0.0, -1.0,  0.0,1.0, 1.0),
    Pos4Norm4Uv2Vertex::new(-0.5, -0.5,  0.5, 0.0, -1.0,  0.0,0.0, 1.0),
];

///
/// 
/// 
#[rustfmt::skip]
const INDICES: [u16; 36] = [
    0,  1,  2,     2,  3,  0,  // Передняя
    4,  5,  6,     6,  7,  4,  // Задняя
    8,  9,  10,    10, 11, 8,  // Левая
    12, 13, 14,    14, 15, 12, // Правая
    16, 17, 18,    18, 19, 16, // Верхняя
    20, 21, 22,    22, 23, 20, // Нижняя
];

///
/// Constants
///
const MAX_CUBES_PER_FRAME: usize = 100_000;
const MAX_HYPER_CUBES_PER_FRAME: usize = MAX_CUBES_PER_FRAME / 4096;

///
/// Cube instance vertex
///
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CubeInstance {
    pub data: [u16; 2], // Index in hyper-cube and material
}

impl CubeInstance {
    pub fn new(index: u16, material: u8) -> Self {
        Self {
            data: [index, material as u16],
        }
    }
}

impl Vertex for CubeInstance {
    fn input_binding_description(binding: u32) -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(binding)
            .stride(Self::size_in_bytes() as u32)
            .input_rate(vk::VertexInputRate::INSTANCE)
    }

    fn input_attribute_descriptions(
        binding: u32,
        location: u32,
    ) -> Vec<vk::VertexInputAttributeDescription> {
        let offset = vk::VertexInputAttributeDescription::default()
            .binding(binding)
            .location(location)
            .format(vk::Format::R16G16_UINT)
            .offset(std::mem::offset_of!(Self, data) as u32);
        vec![offset]
    }
}

///
/// Hyper cube instance
///
struct HyperCubeInstance {
    position: [f32; 3],
    first_cube: usize,
    cube_count: usize,
}

impl HyperCubeInstance {
    pub fn new(x: f32, y: f32, z: f32, first_cube: usize, cube_count: usize) -> Self {
        Self {
            position: [x, y, z],
            first_cube,
            cube_count,
        }
    }
}

///
///
///
///
#[repr(C)]
#[derive(Clone, Copy)]
struct PushConstants {
    position: [f32; 3],
}

impl PushConstants {
    fn as_bytes(&self) -> &[u8] {
        let ptr = self as *const Self as *const u8;
        unsafe { slice::from_raw_parts(ptr, Self::size_in_bytes()) }
    }

    fn size_in_bytes() -> usize {
        std::mem::size_of::<PushConstants>()
    }
}

///
/// Frame objects
///
struct FrameObjects {
    vertex_buffer: VkBuffer,
    index_buffer: VkBuffer,
    instance_buffer: VkDynamicBuffer,
    uniform_buffer: VkBuffer,
    descriptor_set: vk::DescriptorSet,
}

impl FrameObjects {
    fn new(instance: &VkContext, descriptor_set: vk::DescriptorSet) -> Result<Self, VkError> {
        let instance_buffer =
            VkDynamicBuffer::vertex::<CubeInstance>(instance, MAX_CUBES_PER_FRAME)?;
        let vertex_buffer = VkBuffer::vertex(instance, VERTICES.as_ptr(), VERTICES.len())?;
        let index_buffer = VkBuffer::index(instance, INDICES.as_ptr(), INDICES.len())?;
        let uniform_buffer = VkBuffer::uniform::<UniformBufferObject>(instance)?;
        Ok(Self {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            uniform_buffer,
            descriptor_set,
        })
    }

    fn destroy(&self, device: &ash::Device) {
        self.uniform_buffer.destroy(device);
        self.index_buffer.destroy(device);
        self.vertex_buffer.destroy(device);
        self.instance_buffer.destroy(device);
    }
}

#[derive()]
pub struct CubePipeline {
    app: Arc<App>,
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    frame_objects: [FrameObjects; MAX_FRAMES_IN_FLIGHT],
    texture: VkImage,
    cubes: Vec<CubeInstance>,
    hyper_cubes: Vec<HyperCubeInstance>,
}

impl CubePipeline {
    pub fn new(instance: &VkContext, app: &Arc<App>) -> Result<Self, VkError> {
        let vert = app.load_resource("shaders/cube.vert.spv", &load_bytes, ())?;
        let frag = app.load_resource("shaders/cube.frag.spv", &load_bytes, ())?;

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

        let vertex_binding = Pos4Norm4Uv2Vertex::input_binding_description(0);
        let instance_binding = CubeInstance::input_binding_description(1);
        let vertex_attrs = Pos4Norm4Uv2Vertex::input_attribute_descriptions(0, 0);
        let instance_attrs =
            CubeInstance::input_attribute_descriptions(1, vertex_attrs.len() as u32);
        let attribute_descriptions: Vec<vk::VertexInputAttributeDescription> =
            vertex_attrs.into_iter().chain(instance_attrs).collect();

        let bindings = [vertex_binding, instance_binding];
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&attribute_descriptions[..]);

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
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
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
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<PushConstants>() as u32);
        let push_constant_ranges = [push_constant_range];
        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(layouts)
            .push_constant_ranges(&push_constant_ranges);
        let layout = unsafe { instance.device.create_pipeline_layout(&layout_info, None) }?;
        let dynamic_states = [
            ash::vk::DynamicState::VIEWPORT,
            ash::vk::DynamicState::SCISSOR,
        ];
        let dynamic_state =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS);

        let stages = &[vert_stage, frag_stage];
        let info = vk::GraphicsPipelineCreateInfo::default()
            .stages(stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .dynamic_state(&dynamic_state)
            .rasterization_state(&rasterization_state)
            .depth_stencil_state(&depth_stencil_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .layout(layout)
            .render_pass(instance.swapchain.render_pass)
            .subpass(0);

        let infos = [info];
        let mut result = unsafe {
            instance
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), &infos, None)
        }
        .map_err(|(_, e)| VkError::VkErrorCode(e))?;

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
        let texture = instance.create_texture_image(&app.files)?;
        let frame_objects = descriptor_sets
            .into_iter()
            .map(|ds| FrameObjects::new(instance, ds))
            .collect::<Result<Vec<_>, VkError>>()?
            .try_into()
            .map_err(|_| VkError::GenericError("Array size mismatch!".to_string()))?;

        let mut result = Self {
            app: Arc::clone(app),
            layout,
            pipeline,
            descriptor_set_layout,
            descriptor_pool,
            frame_objects,
            texture,
            cubes: Vec::with_capacity(MAX_CUBES_PER_FRAME),
            hyper_cubes: Vec::with_capacity(MAX_HYPER_CUBES_PER_FRAME),
        };
        result.update_descriptor_sets(instance)?;

        Ok(result)
    }

    pub fn update_uniform_buffer(
        &self,
        instance: &VkContext,
        frame_index: usize,
        time: f32,
        ratio: f32,
    ) -> Result<(), VkError> {
        let model = Mat4::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 90.0f32.to_radians() * time);

        let view = glam::camera::lh::view::look_at_mat4(
            Vec3::new(18.0, 18.0, 24.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        );

        let proj =
            glam::camera::lh::proj::vulkan::perspective(45.0f32.to_radians(), ratio, 0.1, 100.0);

        let frame_obj = &self.frame_objects[frame_index];
        let ubo = UniformBufferObject { model, view, proj };
        let buf_memory = frame_obj.uniform_buffer.memory;

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
        //self.update_descriptor_sets(instance)
        Ok(())
    }

    fn update_descriptor_sets(&mut self, instance: &VkContext) -> Result<(), VkError> {
        for frame_obj in self.frame_objects.iter() {
            let info = vk::DescriptorBufferInfo::default()
                .buffer(frame_obj.uniform_buffer.buffer)
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

    pub fn draw_hyper_cube(&mut self, cube: &HyperCube) {
        if self.cubes.len() >= MAX_CUBES_PER_FRAME {
            warn!("Max cubes per frame reached: {}", self.cubes.len());
            return;
        }
        if self.hyper_cubes.len() >= MAX_HYPER_CUBES_PER_FRAME {
            warn!(
                "Max hyper cubes per frame reached: {}",
                self.hyper_cubes.len()
            );
            return;
        }
        let first_cube = self.cubes.len();
        cube.pvs.ones().for_each(|index| {
            let material = cube.cubes[index];
            if material > 0 {
                self.cubes.push(CubeInstance::new(index as u16, material));
            }
        });
        let count = self.cubes.len() - first_cube;
        self.hyper_cubes.push(HyperCubeInstance::new(
            cube.origin.x,
            cube.origin.y,
            cube.origin.z,
            first_cube,
            count,
        ));
    }

    pub fn draw_to_buffer(
        &mut self,
        instance: &VkContext,
        frame_index: usize,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), VkError> {
        let device = &instance.device;
        let frame_obj = &self.frame_objects[frame_index];

        let cubes = &mut self.cubes;
        frame_obj
            .instance_buffer
            .copy_from(cubes.as_ptr(), cubes.len());

        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            let buffers = [
                frame_obj.vertex_buffer.buffer,
                frame_obj.instance_buffer.buffer,
            ];
            let offsets = [0, 0];
            device.cmd_bind_vertex_buffers(command_buffer, 0, &buffers, &offsets);
            device.cmd_bind_index_buffer(
                command_buffer,
                frame_obj.index_buffer.buffer,
                0,
                vk::IndexType::UINT16,
            );
            let descriptor_sets = [frame_obj.descriptor_set];
            let dyn_offsets = [];
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.layout,
                0,
                &descriptor_sets,
                &dyn_offsets,
            );

            for hc in self.hyper_cubes.iter() {
                let push = PushConstants {
                    position: hc.position,
                };
                device.cmd_push_constants(
                    command_buffer,
                    self.layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    push.as_bytes(),
                );

                device.cmd_draw_indexed(
                    command_buffer,
                    INDICES.len() as u32,
                    hc.cube_count as u32,
                    0,
                    0,
                    hc.first_cube as u32,
                );
            }
        }
        cubes.clear();
        self.hyper_cubes.clear();
        Ok(())
    }

    pub fn destroy(&mut self, device: &Device) {
        unsafe {
            self.texture.destroy(device);
            self.frame_objects.iter().for_each(|fo| fo.destroy(device));
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
