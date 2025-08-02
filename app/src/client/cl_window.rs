use std::sync::Arc;

use rg_common::App;
use winit::{
    application::ApplicationHandler, dpi::PhysicalSize, event::WindowEvent,
    platform::startup_notify::WindowAttributesExtStartupNotify, window::Window,
};

use crate::error::AppError;

#[derive(Default, Debug)]
pub(super) struct ClientWindow {
    window: Option<Window>,
}

impl ClientWindow {
    pub(super) fn new(app: &Arc<App>) -> Result<Self, AppError> {
        Ok(Self { window: None })
    }
}

impl ApplicationHandler for ClientWindow {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(PhysicalSize::new(800, 600))
                        .with_title("Rust Ground")
                        .with_decorations(true)
                        .with_fullscreen(None)
                        .with_resizable(true)
                        .with_visible(true)
                )
                .unwrap(),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                // Draw.

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}
