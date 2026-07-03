use std::sync::{Arc, RwLock};

use log::info;
use rg_common::App;
use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    event_loop::ActiveEventLoop,
    monitor::{MonitorHandle, VideoModeHandle},
    window::{Fullscreen, Window, WindowAttributes},
};

use crate::{config::Config, error::VkError};

pub(crate) const MAX_VIDEO_MODE: LogicalSize<u32> = LogicalSize::new(3840, 2160);

pub(crate) fn create_window(
    app: &Arc<App>,
    config: &Arc<RwLock<Config>>,
    event_loop: &ActiveEventLoop,
) -> Result<Window, VkError> {
    print_available_monitors(event_loop);

    let cfg = config.read()?;
    let monitor = select_best_monitor(event_loop, &cfg.preferred_monitor);
    if let Some(m) = monitor.as_ref() {
        info!(
            "Using {:?} {}x{}@{} Hz",
            m.name(),
            m.size().width,
            m.size().height,
            m.refresh_rate_millihertz().unwrap_or_default() / 1000
        );
    }
    let attributes = prepare_window_attributes(&cfg, monitor).with_title(&app.name);
    let window = event_loop.create_window(attributes)?;

    return Ok(window);
}

pub(crate) fn prepare_window_attributes(
    config: &Config,
    monitor: Option<MonitorHandle>,
) -> WindowAttributes {
    let mut attrs = Window::default_attributes();

    let max_logical_size = if let Some(monitor) = monitor.as_ref() {
        monitor.size().to_logical(monitor.scale_factor())
    } else {
        MAX_VIDEO_MODE
    };
    let width = config.width.clamp(400, max_logical_size.width);
    let height = config.height.clamp(200, max_logical_size.height);
    let size = LogicalSize::new(width, height);

    if config.windowed {
        info!("Using windowed mode");
        attrs = attrs
            .with_inner_size(size)
            .with_resizable(true)
            .with_decorations(true);
        if let Some(monitor) = monitor.as_ref() {
            let win_size = size.to_physical::<i32>(monitor.scale_factor());
            let pos = monitor.position();
            if pos.x != 0 || pos.y != 0 {
                let monitor_size = monitor.size();
                let x = pos.x + (monitor_size.width as i32 / 2) - (win_size.width / 2);
                let y = pos.y + (monitor_size.height as i32 / 2) - (win_size.height / 2);
                attrs = attrs.with_position(PhysicalPosition::new(x, y));
            }
        }
    } else {
        let is_exclusive = false;
        if is_exclusive
            && let Some(video_mode) =
                find_best_video_mode(&monitor, size, config.bit_depth, config.refresh_rate)
        {
            info!("Using exclusive fullscreen mode {:?}", &video_mode);
            attrs = attrs.with_fullscreen(Some(Fullscreen::Exclusive(video_mode)));
        } else {
            info!("Using maximized fullscreen mode");
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(monitor)));
        }
    }

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

pub(crate) fn select_best_monitor(
    event_loop: &ActiveEventLoop,
    preferred_monitor_name: &Option<String>,
) -> Option<MonitorHandle> {
    let selected = if let Some(preferred_name) = preferred_monitor_name {
        event_loop.available_monitors().find(|m| {
            if let Some(name) = m.name() {
                name.contains(preferred_name)
            } else {
                false
            }
        })
    } else {
        None
    };
    return selected.or_else(|| event_loop.primary_monitor());
}

fn find_best_video_mode(
    monitor: &Option<MonitorHandle>,
    size: LogicalSize<u32>,
    bit_depth: u16,
    refresh_rate: u32,
) -> Option<VideoModeHandle> {
    if let Some(monitor) = monitor.as_ref() {
        let target = size.to_physical(monitor.scale_factor());
        monitor.video_modes().max_by_key(|mode| {
            let m_size = mode.size();
            let m_refresh_rate = mode.refresh_rate_millihertz() / 1000;
            let m_bit_depth = mode.bit_depth();

            let w_diff = m_size.width.abs_diff(target.width) as i32;
            let h_diff = m_size.height.abs_diff(target.height) as i32;
            let rr_diff = m_refresh_rate.abs_diff(refresh_rate) as i32;
            let bd_diff = m_bit_depth.abs_diff(bit_depth) as i32;

            (-w_diff, -h_diff, -rr_diff, -bd_diff)
        })
    } else {
        None
    }
}

fn print_available_monitors(event_loop: &ActiveEventLoop) {
    for (index, monitor) in event_loop.available_monitors().enumerate() {
        info!(
            "Monitor#{:?}: {:?}, x={:?}, y={:?}, {:?} Hz, {:?}x{:?}, x{:?}",
            index,
            monitor.name().unwrap_or("unknown".to_string()),
            monitor.position().x,
            monitor.position().y,
            monitor
                .refresh_rate_millihertz()
                .map_or(0.0, |v| { v as f32 / 1000.0f32 }),
            monitor.size().width,
            monitor.size().height,
            monitor.scale_factor()
        );
        for (index, mode) in monitor.video_modes().enumerate() {
            info!(
                "  {:?}={:?}x{:?}x{:?}@{:?}",
                index,
                mode.size().width,
                mode.size().height,
                mode.bit_depth(),
                mode.refresh_rate_millihertz() as f32 / 1000.0f32
            );
        }
    }
}
