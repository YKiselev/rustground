use ash::vk;

use crate::types::{Vec2, Vec4};

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

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Pos2Color4Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
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
        [pos, color]
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

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Pos2Color4Tex2Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
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
        [pos, color, tex]
    }
}
