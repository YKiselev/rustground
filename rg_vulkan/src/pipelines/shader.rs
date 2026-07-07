
use ash::{vk};
use std::io::Cursor;

use crate::error::{VkError, to_generic};

pub(crate) fn create_shader_module(
    device: &ash::Device,
    bytecode: &[u8],
) -> Result<vk::ShaderModule, VkError> {
    let mut cursor = Cursor::new(bytecode);
    let code = ash::util::read_spv(&mut cursor).map_err(|e| to_generic(e))?;
    let info = vk::ShaderModuleCreateInfo::default()
        .code(&code);
    Ok(unsafe { device.create_shader_module(&info, None) }?)
}