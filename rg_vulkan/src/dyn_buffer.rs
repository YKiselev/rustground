use std::os::raw::c_void;

use ash::{Device, vk};

use crate::{error::VkError, instance::VkInstance};

pub struct VkDynamicBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub mapped_ptr: *mut c_void,
    pub capacity_bytes: vk::DeviceSize,
}

impl VkDynamicBuffer {
    pub fn vertex<V: Sized>(instance: &VkInstance, max_vertices: usize) -> Result<Self, VkError> {
        create_dynamic_buffer::<V>(instance, max_vertices, vk::BufferUsageFlags::VERTEX_BUFFER)
    }

    pub fn index<I: Sized>(instance: &VkInstance, max_indices: usize) -> Result<Self, VkError> {
        create_dynamic_buffer::<I>(instance, max_indices, vk::BufferUsageFlags::INDEX_BUFFER)
    }

    pub fn uniform<U: Sized>(instance: &VkInstance) -> Result<Self, VkError> {
        create_dynamic_buffer::<U>(instance, 1, vk::BufferUsageFlags::UNIFORM_BUFFER)
    }

    pub fn destroy(&self, device: &Device) {
        unsafe {
            device.unmap_memory(self.memory);
            device.destroy_buffer(self.buffer, None);
            device.free_memory(self.memory, None);
        }
    }
}

fn create_dynamic_buffer<T: Sized>(
    instance: &VkInstance,
    max_items: usize,
    usage_flags: vk::BufferUsageFlags,
) -> Result<VkDynamicBuffer, VkError> {
    let capacity_bytes = (size_of::<T>() * max_items) as vk::DeviceSize;

    let (buffer, memory) = instance.create_buffer(
        capacity_bytes,
        usage_flags,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    let mapped_ptr = unsafe {
        instance
            .device
            .map_memory(memory, 0, capacity_bytes, vk::MemoryMapFlags::empty())?
    };

    Ok(VkDynamicBuffer {
        buffer,
        memory,
        mapped_ptr,
        capacity_bytes,
    })
}
