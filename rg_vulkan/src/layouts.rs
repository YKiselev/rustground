use ash::{Device, vk};

use crate::error::VkError;



pub(crate) fn create_descriptor_set_layout(device: &Device) -> Result<vk::DescriptorSetLayout, VkError> {
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

    let descriptor_set_layout = unsafe { device.create_descriptor_set_layout(&info, None) }?;

    Ok(descriptor_set_layout)
}
