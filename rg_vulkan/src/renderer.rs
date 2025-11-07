use std::time::Instant;

use log::{info, warn};
use vulkanalia::{
    Entry,
    loader::{LIBRARY, LibloadingLoader},
};
use winit::window::Window;

use crate::{
    error::{VkError, to_generic},
    instance::VkInstance,
    triangle::Triangle,
};

#[derive(Debug)]
pub struct VulkanRenderer {
    entry: Entry,
    instance: VkInstance,
    resized: bool,
    start: Instant,
    triangle: Triangle,
}

impl VulkanRenderer {
    pub fn new(window: &Window) -> Result<Self, VkError> {
        unsafe {
            let loader = LibloadingLoader::new(LIBRARY).map_err(to_generic)?;
            let entry = Entry::new(loader).map_err(to_generic)?;
            let instance = VkInstance::new(window, &entry)?;
            let mut triangle = Triangle::new(&instance)?;
            triangle.update_descriptor_sets(&instance)?;
            info!("Vulkan renderer initialzied");
            Ok(Self {
                entry,
                instance,
                resized: false,
                start: Instant::now(),
                triangle,
            })
        }
    }

    pub fn render(&mut self, window: &Window) {
        let mut recreate_swapchain = self.resized;
        if !recreate_swapchain {
            match self.instance.begin_frame() {
                Ok(image_index) => {
                    self.render_frame(image_index);

                    match self.instance.end_frame(image_index) {
                        Ok(changed) => recreate_swapchain = changed,
                        Err(e) => warn!("Failed to end frame: {:?}", e),
                    }
                }
                Err(VkError::SwapchainChanged) => recreate_swapchain = true,
                Err(e) => warn!("Failed to begin frame: {:?}", e),
            }
        }
        if recreate_swapchain {
            let _ = self.recreate_swapchain(window)
                .inspect_err(|e| warn!("Failed to recreate swapchain: {:?}", e));
        }
    }

    fn recreate_swapchain(&mut self, window: &Window) -> Result<(), VkError> {
        self.resized = false;
        self.instance.recreate_swapchain(window)?;
        self.triangle.destroy(&self.instance.device);
        self.triangle = Triangle::new(&self.instance)?;
        self.triangle.update_descriptor_sets(&self.instance)?;
        Ok(())
    }

    fn render_frame(&mut self, image_index: usize) {
        match self.triangle.draw_to_buffer(&self.instance, image_index, self.instance.command_buffer()) {
            Ok(_) => {
                let time = self.start.elapsed().as_secs_f32();
                let extent = self.instance.swapchain.extent;
                let ratio = extent.width as f32 / extent.height as f32;
                let _ =
                    self.triangle
                        .update_uniform_buffer(&self.instance, image_index, time, ratio);
            }
            Err(e) => warn!("Failed to draw to command buffer: {:?}", e),
        }
    }

    pub fn mark_resized(&mut self) {
        self.resized = true;
    }

    pub fn destroy(&mut self) {
        info!("Destroing renderer");
        self.instance.wait_idle().unwrap();
        self.triangle.destroy(&self.instance.device);
        self.instance.destroy();
    }
}
