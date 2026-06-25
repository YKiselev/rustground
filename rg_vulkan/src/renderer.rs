use std::{sync::Arc, time::Instant};

use log::{info, warn};
use rg_common::{App, Plugin};
use winit::window::Window;

use crate::{error::VkError, instance::VkInstance, triangle::Triangle};

pub struct VulkanRenderer {
    instance: VkInstance,
    window_resized: bool,
    start: Instant,
    triangle: Triangle,
    app: Arc<App>,
}

impl VulkanRenderer {
    pub fn new(app: &Arc<App>, window: &Window) -> Result<Self, VkError> {
        let instance = VkInstance::new(window)?;
        let mut triangle = Triangle::new(&instance)?;
        triangle.update_descriptor_sets(&instance)?;
        info!("Vulkan renderer initialzied");
        Ok(Self {
            instance,
            window_resized: false,
            start: Instant::now(),
            triangle,
            app: Arc::clone(app),
        })
    }

    pub fn render(&mut self, window: &Window) {
        let mut recreate_swapchain = self.window_resized;
        if !recreate_swapchain {
            match self.instance.begin_frame() {
                Ok(image_index) => {
                    self.render_frame(image_index);

                    match self.instance.end_frame(image_index, window) {
                        Ok(changed) => recreate_swapchain = changed,
                        Err(e) => warn!("Failed to end frame: {:?}", e),
                    }
                }
                Err(VkError::SwapchainChanged) => recreate_swapchain = true,
                Err(e) => warn!("Failed to begin frame: {:?}", e),
            }
        }
        if recreate_swapchain {
            let _ = self
                .recreate_swapchain(window)
                .inspect_err(|e| warn!("Failed to recreate swapchain: {:?}", e));
        }
    }

    fn recreate_swapchain(&mut self, window: &Window) -> Result<(), VkError> {
        self.window_resized = false;
        self.instance.recreate_swapchain(window)?;

        self.triangle.update_descriptor_sets(&self.instance)?;

        info!("Swapchain recreated");
        Ok(())
    }

    fn render_frame(&mut self, image_index: usize) {
        match self.triangle.draw_to_buffer(
            &self.instance,
            image_index,
            self.instance.swapchain.frames_in_flight.frame().command_buffer,
        ) {
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
        self.window_resized = true;
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        info!("Destroing renderer");
        self.instance.wait_idle().unwrap();
        self.triangle.destroy(&self.instance.device);
    }
}
