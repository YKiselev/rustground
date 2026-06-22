use ash::khr::surface;
use ash::{Entry, Instance, vk};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

use crate::error::VkError;

pub(crate) struct VkSurface {
    pub loader: surface::Instance,
    pub surface: vk::SurfaceKHR,
}

impl VkSurface {
    pub fn new(entry: &Entry, instance: &Instance, window: &Window) -> Result<Self, VkError> {
        let loader = ash::khr::surface::Instance::new(entry, instance);
        let display_handle = window.display_handle().unwrap().as_raw();
        let window_handle = window.window_handle().unwrap().as_raw();
        let surface = unsafe {
            ash_window::create_surface(entry, instance, display_handle, window_handle, None)?
        };
        Ok(Self { loader, surface })
    }

    pub fn get_support(
        &self,
        device: vk::PhysicalDevice,
        queue_family_index: u32,
    ) -> Result<bool, VkError> {
        Ok(unsafe {
            self.loader.get_physical_device_surface_support(
                device,
                queue_family_index,
                self.surface,
            )
        }?)
    }

    pub fn get_capabilities(
        &self,
        device: vk::PhysicalDevice,
    ) -> Result<vk::SurfaceCapabilitiesKHR, VkError> {
        Ok(unsafe {
            self.loader
                .get_physical_device_surface_capabilities(device, self.surface)?
        })
    }

    pub fn get_formats(
        &self,
        device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::SurfaceFormatKHR>, VkError> {
        Ok(unsafe {
            self.loader
                .get_physical_device_surface_formats(device, self.surface)?
        })
    }

    pub fn get_present_modes(
        &self,
        device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::PresentModeKHR>, VkError> {
        Ok(unsafe {
            self.loader
                .get_physical_device_surface_present_modes(device, self.surface)?
        })
    }
}

impl Drop for VkSurface {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_surface(self.surface, None);
        }
    }
}
