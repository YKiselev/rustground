use ash::{
    Device,
    vk::{self},
};

use crate::{error::VkError, instance::MAX_FRAMES_IN_FLIGHT};

#[derive(Debug, Default)]
pub(crate) struct FrameObjects {
    pub in_flight_fence: vk::Fence,
    pub image_available: vk::Semaphore, // present semaphore
    command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,
}

impl FrameObjects {
    fn new(device: &Device, queue_family_index: u32) -> Result<Self, VkError> {
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_index);
        let command_pool = unsafe { device.create_command_pool(&info, None) }?;
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let command_buffers = unsafe { device.allocate_command_buffers(&allocate_info)? };
        Ok(FrameObjects {
            in_flight_fence: unsafe { device.create_fence(&fence_info, None) }?,
            image_available: unsafe { device.create_semaphore(&semaphore_info, None) }?,
            command_pool: command_pool,
            command_buffer: command_buffers[0],
        })
    }

    pub fn reset_buffers(&self, device: &Device) -> Result<(), VkError> {
        unsafe { device.reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::empty())? }
        Ok(())
    }
}

impl FrameObjects {
    fn destroy(&self, device: &Device) {
        unsafe {
            device.free_command_buffers(
                self.command_pool,
                std::slice::from_ref(&self.command_buffer),
            );
            device.destroy_command_pool(self.command_pool, None);
            device.destroy_semaphore(self.image_available, None);
            device.destroy_fence(self.in_flight_fence, None);
        }
    }
}

///
/// Frames in flight
///
#[derive(Debug, Default)]
pub(crate) struct FramesInFlight {
    current_frame: usize,
    frames: Vec<FrameObjects>,
}

impl FramesInFlight {
    pub(crate) fn new(device: &Device, queue_family_index: u32) -> Result<FramesInFlight, VkError> {
        let frames = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| FrameObjects::new(device, queue_family_index))
            .into_iter()
            .collect::<Result<Vec<FrameObjects>, VkError>>()?;
        Ok(Self {
            current_frame: 0,
            frames: frames,
        })
    }

    pub fn destroy(&mut self, device: &Device) {
        self.frames.iter().for_each(|f| f.destroy(device));
        self.frames.clear();
    }

    pub fn advance_frame_index(&mut self) {
        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    pub fn frame(&self) -> &FrameObjects {
        &self.frames[self.current_frame]
    }
}
