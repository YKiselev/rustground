
use winit::{
    dpi::PhysicalSize,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};

pub(crate) fn create_window_attributes(event_loop: &ActiveEventLoop) -> WindowAttributes {
    let mut attrs = Window::default_attributes()
        .with_inner_size(PhysicalSize::new(800, 600))
        .with_title("Rust Ground")
        .with_decorations(true)
        .with_fullscreen(None)
        .with_resizable(true)
        .with_visible(true);

    #[cfg(x11_platform)]
    if event_loop.is_x11() {
        attrs = attrs.with_platform_attributes(Box::new(window_attributes_x11(event_loop)?));
    }

    #[cfg(wayland_platform)]
    if event_loop.is_wayland() {
        attrs = attrs.with_platform_attributes(Box::new(window_attributes_wayland(event_loop)));
    }

    #[cfg(macos_platform)]
    if let Some(tab_id) = _tab_id {
        let window_attributes_macos =
            Box::new(WindowAttributesMacOS::default().with_tabbing_identifier(&tab_id));
        attrs = attrs.with_platform_attributes(window_attributes_macos);
    }

    #[cfg(web_platform)]
    {
        attrs = attrs.with_platform_attributes(Box::new(window_attributes_web()));
    }

    attrs
}
