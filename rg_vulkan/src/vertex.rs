
use ash::vk;

use crate::types::{Vec2, Vec3};

///
/// Pos2Color3Vertex
/// 
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Pos2Color3Vertex {
    pub pos: Vec2,
    pub color: Vec3,
}

impl Pos2Color3Vertex {
    pub const fn new(pos: Vec2, color: Vec3) -> Self {
        Self { pos, color }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Pos2Color3Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        let pos = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0);
        let color = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec2>() as u32);
        [pos, color]
    }
}

///
/// Pos2Color3Tex2Vertex
/// 
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Pos2Color3Tex2Vertex {
    pub pos: Vec2,
    pub color: Vec3,
    pub tex: Vec2
}

impl Pos2Color3Tex2Vertex {
    pub const fn new(pos: Vec2, color: Vec3, tex: Vec2) -> Self {
        Self { pos, color, tex }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Pos2Color3Tex2Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        let pos = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0);
        let color = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec2>() as u32);
        let tex = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(size_of::<Vec2>() as u32);
        [pos, color, tex]
    }
}
