use std::fmt::Write;

use rg_common::ui::{canvas::Canvas, color::Color};
use rg_vulkan::renderer::VulkanCanvas;
use winit::keyboard::{
    KeyCode::{PageDown, PageUp},
    NamedKey, SmolStr,
};

use crate::{app_logger::AppLoggerBuffer, client::cl_ui_layer::UiLayer};

#[derive(Default)]
struct CommandLine {
    buffer: String,
    caret_pos: u32,
    scroll_offset: u32,
}

pub struct Console {
    app_log_buffer: AppLoggerBuffer,
    height: u32,
    scroll_offset: u32,
    opening: bool,
    line_buf: String,
    cmd_line: CommandLine,
}

impl Console {
    pub fn new(app_log_buffer: AppLoggerBuffer) -> Self {
        Self {
            app_log_buffer,
            height: 0,
            scroll_offset: 0,
            opening: false,
            line_buf: String::with_capacity(200),
            cmd_line: CommandLine::default(),
        }
    }

    pub fn update(&mut self) {
        self.app_log_buffer.update();
    }

    fn draw_lines(&mut self, y: i32, margin: u32, canvas: &mut VulkanCanvas) {
        let max_y = y;
        let x = margin as i32;
        let mut y = y + self.scroll_offset as i32;
        let line_width = canvas.width() - 2 * margin;

        for record in self.app_log_buffer.iter() {
            self.line_buf.clear();

            if let Ok(_) = write!(
                self.line_buf,
                "{} {:>6} {}",
                record.time.format("%H:%M:%S%.3f"),
                record.level,
                record.message
            ) {
                let line_height = canvas.measure_text(line_width, &self.line_buf);

                y -= line_height as i32;

                if y <= max_y {
                    canvas.draw_text(x, y, line_width, &self.line_buf);
                }
            } else {
                break;
            }

            if y < 0 {
                break;
            }
        }
    }

    fn draw_command_line(&mut self, y: i32, margin: u32, canvas: &mut VulkanCanvas) -> u32 {
        0
    }
}

impl UiLayer for Console {
    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        event: &winit::event::DeviceEvent,
    ) -> bool {
        match event {
            winit::event::DeviceEvent::MouseWheel { delta } => match delta {
                winit::event::MouseScrollDelta::LineDelta(_, _) => {}
                winit::event::MouseScrollDelta::PixelDelta(physical_position) => {}
            },
            winit::event::DeviceEvent::Key(raw_key_event) => {}
            _ => {}
        }
        {}
        false
    }

    fn keyboard_input(&mut self, event: &winit::event::KeyEvent) -> bool {
        if self.opening && self.height > 0 {
            match &event.logical_key {
                winit::keyboard::Key::Named(named_key) => match named_key {
                    NamedKey::PageUp => {
                        self.scroll_offset = self.scroll_offset.saturating_add(100);
                    }
                    NamedKey::PageDown => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(100);
                    }
                    _ => {}
                },
                winit::keyboard::Key::Character(s) => {
                    self.cmd_line.push_at_caret(s);
                }
                _ => {}
            }
        }
        false
    }

    fn draw(&mut self, canvas: &mut VulkanCanvas) {
        if self.opening {
            self.height = canvas.height();
        } else {
            self.height = 0;
        }

        if self.height < 1 {
            return;
        }

        canvas.set_font(rg_common::ui::canvas::FontId::CONSOLE);
        canvas.set_color(Color::WHITE);
        canvas.set_line_spacing(1);
        canvas.set_wrap_mode(rg_common::ui::canvas::WrapMode::Word);

        let margin = 4;
        let mut y = self.height as i32;

        y -= self.draw_command_line(y, margin, canvas) as i32;

        self.draw_lines(y, margin, canvas);
    }

    fn toggle(&mut self) {
        self.opening = !self.opening;
    }

    fn is_visible(&self) -> bool {
        self.height > 0 || self.opening
    }
}

impl CommandLine {
    pub fn push_at_caret(&mut self, ch: &SmolStr) {}
}
