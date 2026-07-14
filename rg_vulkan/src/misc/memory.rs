use ash::vk;

use crate::error::VkError;

pub(crate) trait VkMemoryProperties {
    fn get_memory_type_index(
        &self,
        properties: vk::MemoryPropertyFlags,
        requirements: vk::MemoryRequirements,
    ) -> Result<u32, VkError>;
}

impl VkMemoryProperties for vk::PhysicalDeviceMemoryProperties {
    fn get_memory_type_index(
        &self,
        properties: vk::MemoryPropertyFlags,
        requirements: vk::MemoryRequirements,
    ) -> Result<u32, VkError> {
        (0..self.memory_type_count)
            .filter(|i| (requirements.memory_type_bits & (1 << i)) != 0)
            .find(|i| {
                let memory_type = self.memory_types.as_slice()[*i as usize];
                memory_type.property_flags.contains(properties)
            })
            .ok_or_else(|| VkError::SuitabilityError("Failed to find appropriate memory type!"))
    }
}
