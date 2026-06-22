use ash::{
    Device,
    vk::{self, CommandBuffer, CommandPool, Fence, Semaphore},
};

use crate::{error::VkError, instance::MAX_FRAMES_IN_FLIGHT};

#[derive(Debug, Default)]
struct FrameInFlight {
    in_flight_fence: Fence,
    image_available: Semaphore,
    command_buffer: CommandBuffer,
}

impl FrameInFlight {
    fn destroy(&self, device: &Device, pool: CommandPool) {
        unsafe {
            device.free_command_buffers(pool, std::slice::from_ref(&self.command_buffer));
            device.destroy_semaphore(self.image_available, None);
            device.destroy_fence(self.in_flight_fence, None);
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct FramesInFlight {
    frame: usize,
    frames: Vec<FrameInFlight>,
}

impl FramesInFlight {
    pub(crate) fn new(device: &Device, pool: CommandPool) -> Result<FramesInFlight, VkError> {
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let semaphore_info = vk::SemaphoreCreateInfo::default();

        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32);
        let command_buffers = unsafe { device.allocate_command_buffers(&allocate_info)? };
        let frames = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|i| {
                Ok(FrameInFlight {
                    in_flight_fence: unsafe { device.create_fence(&fence_info, None) }?,
                    image_available: unsafe { device.create_semaphore(&semaphore_info, None) }?,
                    command_buffer: command_buffers[i],
                })
            })
            .into_iter()
            .collect::<Result<Vec<FrameInFlight>, vk::Result>>()?;
        Ok(Self {
            frame: 0,
            frames: frames,
        })
    }

    pub fn destroy(&mut self, device: &Device, pool: CommandPool) {
        self.frames.iter().for_each(|f| f.destroy(device, pool));
        self.frames.clear();
    }

    pub fn advance_frame_index(&mut self) {
        self.frame = (self.frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    pub fn command_buffer(&self) -> CommandBuffer {
        self.frames[self.frame].command_buffer
    }

    pub fn frence(&self) -> Fence {
        self.frames[self.frame].in_flight_fence
    }

    pub fn image_available_semaphore(&self) -> vk::Semaphore {
        self.frames[self.frame].image_available
    }
}
