use vulkanalia::{
    Device,
    vk::{self, DeviceV1_0, Fence, HasBuilder},
};

use crate::error::VkError;

#[derive(Debug)]
pub(crate) struct Frame {
    pub in_flight_fence: Fence,
    pub image_available: vk::Semaphore,
    pub command_buffer: vk::CommandBuffer,
}

impl Frame {
    pub(crate) fn new(
        device: &Device,
        command_buffer: vk::CommandBuffer,
    ) -> Result<Frame, VkError> {
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let in_flight_fence = unsafe { device.create_fence(&fence_info, None) }?;
        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let image_available = unsafe { device.create_semaphore(&semaphore_info, None) }?;
        Ok(Self {
            in_flight_fence,
            image_available,
            command_buffer,
        })
    }

    pub fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_semaphore(self.image_available, None);
            device.destroy_fence(self.in_flight_fence, None);
        }
    }
}
