use ash::vk::{self};

use crate::types::{Vec2, Vec2i16, Vec2u16, Vec4, Vec4i16, Vec4u16};

///
/// Vertex trait
///
pub(crate) trait Vertex {
    fn input_binding_description() -> vk::VertexInputBindingDescription;
    fn input_attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription>;
    fn size_in_bytes() -> usize;
}

///
/// Pos2Color4Vertex
///
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Pos2Color4Vertex {
    pub pos: Vec2,
    pub color: Vec4,
}

impl Pos2Color4Vertex {
    pub const fn new(pos: Vec2, color: Vec4) -> Self {
        Self { pos, color }
    }
}

impl Vertex for Pos2Color4Vertex {
    fn input_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Pos2Color4Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    fn input_attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        let pos = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(std::mem::offset_of!(Pos2Color4Vertex, pos) as u32);
        let color = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(std::mem::offset_of!(Pos2Color4Vertex, color) as u32);
        vec![pos, color]
    }

    fn size_in_bytes() -> usize {
        size_of::<Pos2Color4Vertex>()
    }
}

///
/// Pos2Color4Tex2Vertex
///
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Pos2Color4Tex2Vertex {
    pub pos: Vec2,
    pub color: Vec4,
    pub tex: Vec2,
}

impl Pos2Color4Tex2Vertex {
    pub const fn new(pos: Vec2, color: Vec4, tex: Vec2) -> Self {
        Self { pos, color, tex }
    }
}

impl Vertex for Pos2Color4Tex2Vertex {
    fn input_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Pos2Color4Tex2Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    fn input_attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        let pos = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(std::mem::offset_of!(Pos2Color4Tex2Vertex, pos) as u32);
        let color = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(std::mem::offset_of!(Pos2Color4Tex2Vertex, color) as u32);
        let tex = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(std::mem::offset_of!(Pos2Color4Tex2Vertex, tex) as u32);
        vec![pos, color, tex]
    }

    fn size_in_bytes() -> usize {
        size_of::<Pos2Color4Tex2Vertex>()
    }
}

///
/// GlyphInstance
///
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GlyphInstance {
    pub pos: [i16; 2],
    pub size: [u16; 2],
    pub color: u32,
    pub uv_min: [u16; 2],
    pub uv_max: [u16; 2],
    pub layer_index: u32,
}

impl Vertex for GlyphInstance {
    fn input_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<GlyphInstance>() as u32)
            .input_rate(vk::VertexInputRate::INSTANCE)
    }

    fn input_attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        let pos = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(0)
            .format(vk::Format::R16G16_SINT)
            .offset(std::mem::offset_of!(GlyphInstance, pos) as u32);
        let size = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(1)
            .format(vk::Format::R16G16_UINT)
            .offset(std::mem::offset_of!(GlyphInstance, size) as u32);
        let color = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(2)
            .format(vk::Format::R8G8B8A8_UNORM)
            .offset(std::mem::offset_of!(GlyphInstance, color) as u32);
        let uv_min = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(3)
            .format(vk::Format::R16G16_UNORM)
            .offset(std::mem::offset_of!(GlyphInstance, uv_min) as u32);
        let uv_max = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(4)
            .format(vk::Format::R16G16_UNORM)
            .offset(std::mem::offset_of!(GlyphInstance, uv_max) as u32);
        let layer_index = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(5)
            .format(vk::Format::R32_UINT)
            .offset(std::mem::offset_of!(GlyphInstance, layer_index) as u32);
        vec![pos, size, color, uv_min, uv_max, layer_index]
    }

    fn size_in_bytes() -> usize {
        size_of::<GlyphInstance>()
    }
}

///
/// Helpers
///
pub(crate) fn vertex_input_descriptions<V>() -> (
    vk::VertexInputBindingDescription,
    Vec<vk::VertexInputAttributeDescription>,
)
where
    V: Vertex,
{
    let binding_descriptions = V::input_binding_description();
    let attribute_descriptions = V::input_attribute_descriptions();
    (binding_descriptions, attribute_descriptions)
}
