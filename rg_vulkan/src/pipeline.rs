
use ash::{Device, vk};
use std::io::Cursor;

use crate::error::{VkError, to_generic};

pub(crate) fn create_shader_module(
    device: &Device,
    bytecode: &[u8],
) -> Result<vk::ShaderModule, VkError> {
    let mut cursor = Cursor::new(bytecode);
    let code = ash::util::read_spv(&mut cursor).map_err(|e| to_generic(e))?;
    let info = vk::ShaderModuleCreateInfo::default()
        .code(&code);
    Ok(unsafe { device.create_shader_module(&info, None) }?)
}

pub(crate) fn create_render_pass(device: &Device, format: vk::Format) -> Result<vk::RenderPass, VkError> {
    let color_attachment = vk::AttachmentDescription::default()
        .format(format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    let color_attachment_ref = vk::AttachmentReference::default()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let color_attachments = &[color_attachment_ref];
    let subpass = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(color_attachments);

    let attachments = &[color_attachment];
    let subpasses = &[subpass];
    let info = vk::RenderPassCreateInfo::default()
        .attachments(attachments)
        .subpasses(subpasses);

    Ok(unsafe { device.create_render_pass(&info, None)? })
}
