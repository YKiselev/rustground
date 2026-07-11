use ash::{Device, vk};

use crate::{error::VkError, context::VkContext};

pub struct VkBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
}

impl VkBuffer {
    pub fn vertex<V: Sized>(
        instance: &VkContext,
        data: *const V,
        count: usize,
    ) -> Result<Self, VkError> {
        let size = (size_of::<V>() * count) as u64;

        let (staging_buffer, staging_buffer_memory) = instance.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;

        instance.copy_memory(
            staging_buffer_memory,
            0,
            size,
            vk::MemoryMapFlags::empty(),
            data,
            count,
        )?;

        let (buffer, memory) = instance.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        instance.copy_buffer(staging_buffer, buffer, size)?;

        unsafe {
            instance.device.destroy_buffer(staging_buffer, None);
            instance.device.free_memory(staging_buffer_memory, None);
        }

        Ok(Self { buffer, memory })
    }

    pub fn index<I: Sized>(
        instance: &VkContext,
        data: *const I,
        count: usize,
    ) -> Result<Self, VkError> {
        let size = (size_of::<I>() * count) as u64;

        let (staging_buffer, staging_buffer_memory) = instance.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;

        instance.copy_memory(
            staging_buffer_memory,
            0,
            size,
            vk::MemoryMapFlags::empty(),
            data,
            count,
        )?;

        let (buffer, memory) = instance.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        instance.copy_buffer(staging_buffer, buffer, size)?;

        unsafe {
            instance.device.destroy_buffer(staging_buffer, None);
            instance.device.free_memory(staging_buffer_memory, None);
        }

        Ok(Self { buffer, memory })
    }

    pub fn uniform<I: Sized>(instance: &VkContext) -> Result<Self, VkError> {
        let (buffer, memory) = instance.create_buffer(
            size_of::<I>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;
        Ok(Self { buffer, memory })
    }

    pub fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_buffer(self.buffer, None);
            device.free_memory(self.memory, None);
        }
    }
}
