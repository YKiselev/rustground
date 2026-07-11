use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use ash::vk;
use log::{info, warn};
use rg_common::{App, wrap_var_bag};
use winit::{event_loop::ActiveEventLoop, window::Window};

use crate::{
    config::Config,
    context::VkContext,
    create_instance::create_instance,
    debug::DebugUtils,
    error::{VkError, to_generic},
    pipelines::{cube::CubePipeline, textured_triangle::TexturedTriangle, ui::UiPipeline},
    window::{MAX_VIDEO_MODE, create_window},
};

pub struct VulkanRenderer {
    app: Arc<App>,
    config: Arc<RwLock<Config>>,
    entry: ash::Entry,
    window: Window,
    instance: ash::Instance,
    debug_utils: Option<DebugUtils>,
    context: VkContext,
    window_resized_at: Option<Instant>,
    recreate_swapchain: bool,
    start: Instant,
    tex_triangle: TexturedTriangle,
    cube: CubePipeline,
    ui: UiPipeline,
    image_index: Option<usize>,
    command_buffer: Option<vk::CommandBuffer>,
}

const RESIZE_DEBOUNCE_DURATION: Duration = Duration::from_millis(200);

impl VulkanRenderer {
    pub fn new(app: &Arc<App>, event_loop: &ActiveEventLoop) -> Result<Self, VkError> {
        let config = prepare_config(app)?;

        info!("Loading Vulkan entry...");
        let entry = unsafe { ash::Entry::load().map_err(to_generic)? };

        info!("Creating window...");
        let window = create_window(app, &config, event_loop)?;

        info!("Creating Vulkan instance...");
        let (instance, debug_utils) = create_instance(app, &window, &entry)?;

        info!("Creating Vulkan device...");
        let context = VkContext::new(app, &config, &entry, &instance, &window)?;

        info!("Creating pipelines...");
        let tex_triangle = TexturedTriangle::new(&context, app)?;
        let ui = UiPipeline::new(&context, app, window.scale_factor())?;
        let cube = CubePipeline::new(&context, app)?;

        info!("Vulkan renderer initialzied");
        window.set_visible(true);

        Ok(Self {
            app: Arc::clone(app),
            config,
            entry,
            window,
            instance,
            debug_utils,
            context,
            window_resized_at: None,
            recreate_swapchain: false,
            start: Instant::now(),
            tex_triangle,
            cube,
            ui,
            image_index: None,
            command_buffer: None,
        })
    }

    pub fn begin_frame(&mut self) {
        self.image_index = None;
        self.command_buffer = None;

        if let Some(resize_time) = self.window_resized_at.as_ref() {
            if resize_time.elapsed() >= RESIZE_DEBOUNCE_DURATION {
                self.recreate_swapchain = true;
            }
        }

        if !self.recreate_swapchain {
            match self.context.begin_frame() {
                Ok(image_index) => {
                    self.image_index = Some(image_index);
                }
                Err(VkError::SwapchainChanged) => {
                    if self.window_resized_at.is_none() {
                        self.recreate_swapchain = true
                    }
                }
                Err(e) => warn!("Failed to begin frame: {:?}", e),
            }
        }
        if let Some(image_index) = self.image_index {
            self.command_buffer = match self.begin_render_pass(image_index) {
                Ok(buf) => Some(buf),
                Err(e) => {
                    warn!("Failed to begin frame: {:?}", e);
                    return;
                }
            };
        }
    }

    pub fn render(&mut self) {
        if let Some(command_buffer) = self.command_buffer {
            let frame_index = self.context.swapchain.frames_in_flight.current_frame;

            self.draw_frame(frame_index, command_buffer);
        }
    }

    pub fn end_frame(&mut self) -> bool {
        if let Some(command_buffer) = self.command_buffer {
            match self.end_render_pass(command_buffer) {
                Ok(_) => {}
                Err(e) => warn!("Failed to end render pass: {:?}", e),
            }
        }
        if let Some(image_index) = self.image_index {
            match self.context.end_frame(image_index, &self.window) {
                Ok(changed) => {
                    if self.window_resized_at.is_none() {
                        self.recreate_swapchain = changed
                    }
                }
                Err(e) => {
                    warn!("Failed to end frame: {:?}", e);
                    return false;
                }
            }
            self.image_index = None;
        }
        if self.recreate_swapchain {
            match self.config.write() {
                Ok(mut cfg) => {
                    let scale_factor = self.window.scale_factor();
                    let new_size = self.window.inner_size().to_logical::<u32>(scale_factor);
                    cfg.width = new_size.width;
                    cfg.height = new_size.height;
                }
                Err(_) => warn!("Unable to update window size in config - lock is poisoned!"),
            }
            match self
                .recreate_swapchain()
                .inspect_err(|e| warn!("Failed to recreate swapchain: {:?}", e))
            {
                Ok(_) => self.recreate_swapchain = false,
                Err(e) => {
                    warn!("Failed to recreate swapchain: {:?}", e);
                }
            }
        }
        self.window.request_redraw();
        true
    }

    fn recreate_swapchain(&mut self) -> Result<(), VkError> {
        // Do not recreate swapchain on window minimize
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        self.window_resized_at = None;
        self.context
            .recreate_swapchain(&self.instance, &self.window)?;

        self.tex_triangle.on_swapchain_recreated(&self.context)?;

        info!("Swapchain recreated");
        Ok(())
    }

    fn draw_frame(&mut self, frame_index: usize, command_buffer: vk::CommandBuffer) {
        let time = self.start.elapsed().as_secs_f32();
        let extent = self.context.swapchain.extent;
        let ratio = extent.width as f32 / extent.height as f32;

        // Cube
        let _ = self
            .cube
            .update_uniform_buffer(&self.context, frame_index, time, ratio);
        
        if let Err(e) = self.cube.draw_to_buffer(&self.context, frame_index, command_buffer) {
             warn!("Failed to draw cubes to command buffer: {:?}", e);
        }

        // Triangle
        // let time1 = 0.98 * time;
        // let _ = self
        //     .tex_triangle
        //     .update_uniform_buffer(&self.context, frame_index, time1, ratio);
        // if let Err(e) = self
        //     .tex_triangle
        //     .draw_to_buffer(&self.context, frame_index, command_buffer)
        // {
        //     warn!("Failed to draw to command buffer: {:?}", e);
        // }

        // UI
        if let Err(e) = self
            .ui
            .begin_frame(&self.context, frame_index, command_buffer)
        {
            warn!("Failed to begin ui frame: {}", e);
        }
        match self.ui.end_frame(&self.context, command_buffer) {
            Ok(_) => {}
            Err(e) => warn!("Failed to draw ui to command buffer: {:?}", e),
        }
    }

    fn begin_render_pass(&self, image_index: usize) -> Result<vk::CommandBuffer, VkError> {
        let info = vk::CommandBufferBeginInfo::default();
        let instance = &self.context;
        let command_buffer = self
            .context
            .swapchain
            .frames_in_flight
            .frame()
            .command_buffer;

        unsafe { instance.device.begin_command_buffer(command_buffer, &info) }?;

        let render_area = vk::Rect2D::default().extent(instance.swapchain.extent);

        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.1, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let image = &instance.swapchain.images[image_index];
        let info = vk::RenderPassBeginInfo::default()
            .render_pass(instance.swapchain.render_pass)
            .framebuffer(image.framebuffer)
            .render_area(render_area)
            .clear_values(&clear_values);

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

    fn end_render_pass(&self, command_buffer: vk::CommandBuffer) -> Result<(), VkError> {
        unsafe {
            self.context.device.cmd_end_render_pass(command_buffer);
            self.context.device.end_command_buffer(command_buffer)?;
        }
        Ok(())
    }

    pub fn mark_resized(&mut self) {
        self.window_resized_at = Some(Instant::now());
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        info!("Destroing renderer");
        self.context.wait_idle().unwrap();
        self.ui.destroy(&self.context.device);
        self.tex_triangle.destroy(&self.context.device);
        self.cube.destroy(&self.context.device);
        self.context.destroy();
        if let Some(debug_utils) = self.debug_utils.as_mut() {
            debug_utils.destroy();
        }
        unsafe {
            self.instance.destroy_instance(None);
        }
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

fn prepare_config(app: &Arc<App>) -> Result<Arc<RwLock<Config>>, VkError> {
    let mut config = Config::default();

    // Sane defaults
    config.windowed = true;
    config.width = 800;
    config.height = 600;
    config.bit_depth = 24;
    config.refresh_rate = 60;

    let config = wrap_var_bag(config);
    app.vars.add("gfx", &config).map_err(|e| to_generic(e))?;

    // Command line arguments takes precedence over config values
    let mut cfg = config.write()?;
    let args = &app.arguments;

    if let Some(windowed) = args.windowed {
        cfg.windowed = windowed;
    }
    if let Some(width) = args.width {
        cfg.width = width;
    }
    if let Some(height) = args.height {
        cfg.height = height;
    }
    if let Some(bit_depth) = args.bit_depth {
        cfg.bit_depth = bit_depth;
    }
    if let Some(refresh_rate) = args.refresh_rate {
        cfg.refresh_rate = refresh_rate;
    }

    cfg.width = cfg.width.clamp(400, MAX_VIDEO_MODE.width);
    cfg.height = cfg.height.clamp(200, MAX_VIDEO_MODE.height);
    cfg.bit_depth = cfg.bit_depth.clamp(24, 32);
    cfg.refresh_rate = cfg.refresh_rate.clamp(50, 200);

    std::mem::drop(cfg);

    Ok(config)
}
