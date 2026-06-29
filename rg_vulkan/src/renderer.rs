use std::{sync::Arc, time::Instant};

use ash::{Entry, Instance, vk};
use log::{info, warn};
use rg_common::{App, Plugin};
use winit::window::Window;

use crate::{
    error::{VkError, to_generic},
    instance::VkInstance,
    textured_triangle::TexturedTriangle,
    triangle::Triangle,
};

pub struct VulkanRenderer {
    entry: Entry,
    instance: VkInstance,
    window_resized: bool,
    start: Instant,
    triangle: Triangle,
    tex_triangle: TexturedTriangle,
    app: Arc<App>,
}

impl VulkanRenderer {
    pub fn new(app: &Arc<App>, window: &Window) -> Result<Self, VkError> {
        let entry = unsafe { Entry::load().map_err(to_generic)? };
        let instance = VkInstance::new(&entry, window, app)?;
        let mut triangle = Triangle::new(&instance, app)?;
        let mut tex_triangle = TexturedTriangle::new(&instance, app)?;
        info!("Vulkan renderer initialzied");
        Ok(Self {
            entry,
            instance,
            window_resized: false,
            start: Instant::now(),
            triangle,
            tex_triangle,
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
        // Do not recreate swapchain on window minimize
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        self.window_resized = false;
        self.instance.recreate_swapchain(window)?;

        self.triangle.on_swapchain_recreated(&self.instance)?;
        self.tex_triangle.on_swapchain_recreated(&self.instance)?;

        info!("Swapchain recreated");
        Ok(())
    }

    fn render_frame(&mut self, image_index: usize) {
        let command_buffer = match self.begin_frame(image_index) {
            Ok(buf) => buf,
            Err(e) => {
                warn!("Failed to begin frame: {:?}", e);
                return;
            }
        };
        let time = self.start.elapsed().as_secs_f32();
        let extent = self.instance.swapchain.extent;
        let ratio = extent.width as f32 / extent.height as f32;

        match self
            .triangle
            .draw_to_buffer(&self.instance, image_index, command_buffer)
        {
            Ok(_) => {
                let _ =
                    self.triangle
                        .update_uniform_buffer(&self.instance, image_index, time, ratio);
            }
            Err(e) => warn!("Failed to draw to command buffer: {:?}", e),
        }

        match self
            .tex_triangle
            .draw_to_buffer(&self.instance, image_index, command_buffer)
        {
            Ok(_) => {
                let time = 0.98 * time;
                let _ = self.tex_triangle.update_uniform_buffer(
                    &self.instance,
                    image_index,
                    time,
                    ratio,
                );
            }
            Err(e) => warn!("Failed to draw to command buffer: {:?}", e),
        }

        match self.end_frame(command_buffer) {
            Ok(_) => {}
            Err(e) => warn!("Failed to end frame: {:?}", e),
        }
    }

    fn begin_frame(&self, image_index: usize) -> Result<vk::CommandBuffer, VkError> {
        let info = vk::CommandBufferBeginInfo::default();
        let instance = &self.instance;
        let command_buffer = self
            .instance
            .swapchain
            .frames_in_flight
            .frame()
            .command_buffer;

        unsafe { instance.device.begin_command_buffer(command_buffer, &info) }?;

        let render_area = vk::Rect2D::default()
            .offset(vk::Offset2D::default())
            .extent(instance.swapchain.extent);

        let color_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.1, 1.0],
            },
        };

        let image = &instance.swapchain.images[image_index];
        let clear_values = &[color_clear_value];
        let info = vk::RenderPassBeginInfo::default()
            .render_pass(instance.swapchain.render_pass)
            .framebuffer(image.framebuffer)
            .render_area(render_area)
            .clear_values(clear_values);

        unsafe {
            instance.device.cmd_begin_render_pass(
                command_buffer,
                &info,
                vk::SubpassContents::INLINE,
            );
            let scissors = [render_area];
            instance
                .device
                .cmd_set_scissor(command_buffer, 0, scissors.as_slice());
            let viewport = create_viewport_from_extent(instance.swapchain.extent);
            let viewports = [viewport];
            instance
                .device
                .cmd_set_viewport(command_buffer, 0, viewports.as_slice());
        }
        Ok(command_buffer)
    }

    fn end_frame(&self, command_buffer: vk::CommandBuffer) -> Result<(), VkError> {
        unsafe {
            self.instance.device.cmd_end_render_pass(command_buffer);
            self.instance.device.end_command_buffer(command_buffer)?;
        }
        Ok(())
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
        self.tex_triangle.destroy(&self.instance.device);
        self.instance.destroy();
    }
}

fn create_viewport(width: u32, height: u32) -> vk::Viewport {
    vk::Viewport::default()
        .x(0.0)
        .y(0.0)
        .width(width as f32)
        .height(height as f32)
        .min_depth(0.0)
        .max_depth(1.0)
}

fn create_viewport_from_extent(extent: vk::Extent2D) -> vk::Viewport {
    create_viewport(extent.width, extent.height)
}

fn create_scissor_from_extent(extent: vk::Extent2D) -> vk::Rect2D {
    vk::Rect2D::default()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(extent)
}

pub(crate) fn create_default_viewport_and_scissor(
    extent: vk::Extent2D,
) -> (vk::Viewport, vk::Rect2D) {
    let viewport = create_viewport_from_extent(extent);
    let scissor = vk::Rect2D::default()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(extent);

    (viewport, scissor)
}
