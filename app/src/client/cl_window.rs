use std::sync::Arc;

use log::{error, info};
use rg_common::App;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::ModifiersState,
    window::{Window, WindowAttributes, WindowId},
};

use crate::error::AppError;

#[derive(Debug)]
pub(super) struct ClientWindow {
    app: Arc<App>,
    window: Option<Window>,
    modifiers: ModifiersState,
}

impl ClientWindow {
    pub(super) fn new(app: &Arc<App>) -> Result<Self, AppError> {
        Ok(Self {
            app: Arc::clone(app),
            window: None,
            modifiers: ModifiersState::default(),
        })
    }

    fn create_window_attributes(&mut self, event_loop: &ActiveEventLoop) -> WindowAttributes {
        let mut attrs = Window::default_attributes()
            .with_inner_size(PhysicalSize::new(800, 600))
            .with_title("Rust Ground")
            .with_decorations(true)
            .with_fullscreen(None)
            .with_resizable(true)
            .with_visible(true);

        #[cfg(x11_platform)]
        if event_loop.is_x11() {
            window_attributes = window_attributes
                .with_platform_attributes(Box::new(window_attributes_x11(event_loop)?));
        }

        #[cfg(wayland_platform)]
        if event_loop.is_wayland() {
            window_attributes = window_attributes
                .with_platform_attributes(Box::new(window_attributes_wayland(event_loop)));
        }

        #[cfg(macos_platform)]
        if let Some(tab_id) = _tab_id {
            let window_attributes_macos =
                Box::new(WindowAttributesMacOS::default().with_tabbing_identifier(&tab_id));
            window_attributes = window_attributes.with_platform_attributes(window_attributes_macos);
        }

        #[cfg(web_platform)]
        {
            window_attributes =
                window_attributes.with_platform_attributes(Box::new(window_attributes_web()));
        }

        attrs
    }
}

impl ApplicationHandler for ClientWindow {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = self.create_window_attributes(event_loop);
        self.window = match event_loop.create_window(attrs) {
            Ok(wnd) => Some(wnd),
            Err(e) => {
                error!("Unable to create window: {e:?}");
                None
            }
        };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = if let Some(wnd) = self.window.as_ref() {
            wnd
        } else {
            return;
        };
        match event {
            WindowEvent::Focused(focused) => {
                if focused {
                    info!("Window={window_id:?} focused");
                } else {
                    info!("Window={window_id:?} unfocused");
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                info!("Window={window_id:?} changed scale to {scale_factor}");
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                info!("Keyboard modifiers changed to {:?}", self.modifiers);
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    info!("Mouse wheel Line Delta: ({x},{y})");
                }
                MouseScrollDelta::PixelDelta(px) => {
                    info!("Mouse wheel Pixel Delta: ({},{})", px.x, px.y);
                }
            },
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                let mods = self.modifiers;
                info!("Key input: {event:?}");
                // Dispatch actions only on press.
                // if event.state.is_pressed() {
                //     let action = if let Key::Character(ch) = event.key_without_modifiers.as_ref() {
                //         Self::process_key_binding(&ch.to_uppercase(), &mods)
                //     } else {
                //         None
                //     };

                //     // if let Some(action) = action {
                //     //     self.handle_action_with_window(event_loop, window_id, action);
                //     // }
                // }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                info!("Pointer button {button:?} {state:?}");
                let mods = self.modifiers;
                // if let Some(action) = state
                //     .is_pressed()
                //     .then(|| Self::process_mouse_binding(button.mouse_button(), &mods))
                //     .flatten()
                // {
                //     self.handle_action_with_window(event_loop, window_id, action);
                // }
            }
            WindowEvent::CursorLeft { .. } => {
                info!("Cursor left Window={window_id:?}");
                //window.cursor_left();
            }
            WindowEvent::CursorMoved { position, .. } => {
                info!("Moved cursor to {position:?}");
                //window.cursor_moved(position);
            }
            WindowEvent::ActivationTokenDone { token: _token, .. } => {
                #[cfg(any(x11_platform, wayland_platform))]
                {
                    startup_notify::set_activation_token_env(_token);
                    if let Err(err) = self.create_window(event_loop, None) {
                        error!("Error creating new window: {err}");
                    }
                }
            }
            WindowEvent::RedrawRequested => {               
                window.pre_present_notify();
                
                // Swap buffers, etc

                window.request_redraw();
            }
            _ => (),
        }
    }
}
