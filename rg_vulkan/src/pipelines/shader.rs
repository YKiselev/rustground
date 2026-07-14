use ash::vk;
use std::collections::{HashMap, hash_map::Entry};
use std::io::Cursor;

use crate::error::to_generic;
use crate::{error::VkError, misc::context::VkContext};

///
/// Shader stages
///
pub(crate) struct ShaderStages<'a> {
    shader_modules: HashMap<vk::ShaderStageFlags, vk::ShaderModule>,
    shader_stages: Vec<vk::PipelineShaderStageCreateInfo<'a>>,
}

impl<'a> ShaderStages<'a> {
    pub fn builder() -> ShaderStagesBuilder<'a> {
        ShaderStagesBuilder::new()
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            self.shader_modules
                .iter()
                .for_each(|(_, &sm)| device.destroy_shader_module(sm, None));
            self.shader_modules.clear();
        }
    }

    pub fn stages(&self) -> &[vk::PipelineShaderStageCreateInfo<'a>] {
        &self.shader_stages
    }
}

///
/// Shader stages builder
///
pub(crate) struct ShaderStagesBuilder<'a> {
    shader_codes: HashMap<vk::ShaderStageFlags, &'a [u8]>,
    shader_stages: HashMap<vk::ShaderStageFlags, vk::PipelineShaderStageCreateInfo<'a>>,
}

impl<'a> ShaderStagesBuilder<'a> {
    fn new() -> Self {
        Self {
            shader_codes: HashMap::default(),
            shader_stages: HashMap::default(),
        }
    }

    pub fn with_shader(mut self, stage_flag: vk::ShaderStageFlags, shader_code: &'a [u8]) -> Self {
        self.shader_codes.insert(stage_flag, shader_code);
        self
    }

    pub fn with_vertex_shader(mut self, shader_code: &'a [u8]) -> Self {
        self.with_shader(vk::ShaderStageFlags::VERTEX, shader_code)
    }

    pub fn with_fragment_shader(mut self, shader_code: &'a [u8]) -> Self {
        self.with_shader(vk::ShaderStageFlags::FRAGMENT, shader_code)
    }

    pub fn build(mut self, context: &'a VkContext) -> Result<ShaderStages, VkError> {
        if self.shader_codes.is_empty() {
            return Err(VkError::GenericError(
                "No shaders configured on pipeline!".to_string(),
            ));
        }
        // Create shader modules
        let shader_modules = self
            .shader_codes
            .iter()
            .map(|(&stage, &code)| {
                create_shader_module(&context.device, code).map(|sm| (stage, sm))
            })
            .collect::<Result<HashMap<_, _>, VkError>>()?;

        // Create shader stages
        for (&stage, &module) in shader_modules.iter() {
            if let Entry::Vacant(entry) = self.shader_stages.entry(stage) {
                let shader_stage = vk::PipelineShaderStageCreateInfo::default()
                    .stage(stage)
                    .module(module)
                    .name(c"main");
                entry.insert(shader_stage);
            }
        }

        Ok(ShaderStages {
            shader_modules,
            shader_stages: self.shader_stages.into_values().collect(),
        })
    }
}

///
/// Helpers
///
pub(crate) fn create_shader_module(
    device: &ash::Device,
    bytecode: &[u8],
) -> Result<vk::ShaderModule, VkError> {
    let mut cursor = Cursor::new(bytecode);
    let code = ash::util::read_spv(&mut cursor).map_err(|e| to_generic(e))?;
    let info = vk::ShaderModuleCreateInfo::default().code(&code);
    Ok(unsafe { device.create_shader_module(&info, None) }?)
}
