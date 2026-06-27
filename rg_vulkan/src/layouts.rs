use ash::{Device, vk};

use crate::error::VkError;

pub(crate) struct VkDescriptorSetLayouts {
    pub ubo_only: vk::DescriptorSetLayout,
    pub ubo_sampler_texture: vk::DescriptorSetLayout,
}

impl VkDescriptorSetLayouts {
    pub fn new(device: &Device) -> Result<Self, VkError> {
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

        let bindings = &[ubo_binding];
        let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(bindings);
        let ubo_only = unsafe { device.create_descriptor_set_layout(&info, None) }?;

        let bindings = &[ubo_binding, texture_sampler_binding, texture_layout_binding];
        let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(bindings);
        let ubo_sampler_texture = unsafe { device.create_descriptor_set_layout(&info, None) }?;

        Ok(Self {
            ubo_only,
            ubo_sampler_texture,
        })
    }
}
