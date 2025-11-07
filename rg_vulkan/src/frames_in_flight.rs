use vulkanalia::{
    Device,
    vk::{self, CommandBuffer, CommandPool, DeviceV1_0, Fence, HasBuilder},
};

use crate::{error::VkError, instance::MAX_FRAMES_IN_FLIGHT};

#[derive(Debug, Default)]
pub(crate) struct FramesInFlight {
    frame: usize,
    in_flight_fences: [Fence; MAX_FRAMES_IN_FLIGHT],
    image_available: [vk::Semaphore; MAX_FRAMES_IN_FLIGHT],
    command_buffers: [vk::CommandBuffer; MAX_FRAMES_IN_FLIGHT],
}

impl FramesInFlight {
    pub(crate) fn new(device: &Device, pool: CommandPool) -> Result<FramesInFlight, VkError> {
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let in_flight_fences = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| unsafe { device.create_fence(&fence_info, None) })
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let semaphore_info = vk::SemaphoreCreateInfo::builder().build();
        let image_available = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| unsafe { device.create_semaphore(&semaphore_info, None) })
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32);
        let command_buffers = unsafe { device.allocate_command_buffers(&allocate_info)? };
        Ok(Self {
            frame: 0,
            in_flight_fences: in_flight_fences.try_into().unwrap(),
            image_available: image_available.try_into().unwrap(),
            command_buffers: command_buffers.try_into().unwrap(),
        })
    }

    pub fn destroy(&self, device: &Device, pool: CommandPool) {
        unsafe {
            device.free_command_buffers(pool, &self.command_buffers[..]);
            self.image_available
                .iter()
                .for_each(|s| device.destroy_semaphore(*s, None));
            self.in_flight_fences
                .iter()
                .for_each(|f| device.destroy_fence(*f, None));
        }
    }

    pub fn next_frame(&mut self) {
        self.frame = (self.frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    pub fn command_buffer(&self) -> CommandBuffer {
        self.command_buffers[self.frame]
    }

    pub fn frence(&self) -> Fence {
        self.in_flight_fences[self.frame]
    }

    pub fn image_available_semaphore(&self) -> vk::Semaphore {
        self.image_available[self.frame]
    }
}
