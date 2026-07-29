use std::fmt::Write;

use rg_common::ui::{
    canvas::{Canvas, WrapMode},
    color::Color,
};
use rg_vulkan::renderer::VulkanCanvas;
use winit::{
    event::ElementState,
    keyboard::{
        Key,
        KeyCode::{PageDown, PageUp},
        ModifiersState, NamedKey, SmolStr,
    },
};

use crate::{app_logger::AppLoggerBuffer, client::cl_ui_layer::UiLayer};

#[derive(Default)]
struct CommandLine {
    buffer: String,
    caret_pos: i32,
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

    fn draw_lines(&mut self, y0: i32, margin: u32, canvas: &mut VulkanCanvas) {
        let x = margin as i32;
        let mut y = y0 + self.scroll_offset as i32;
        let line_width = canvas.width() - 2 * margin;

        canvas.set_scissor(x, 0, line_width, y0 as u32);

        for record in self.app_log_buffer.iter() {
            self.line_buf.clear();

            if let Ok(_) = write!(
                self.line_buf,
                "{} {:>6} {}",
                record.time.format("%H:%M:%S%.3f"),
                record.level,
                record.message
            ) {
                let text_height = canvas.measure_text(line_width, &self.line_buf);

                y -= text_height as i32;

                if y <= y0 {
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

    fn draw_command_line(&mut self, y0: i32, margin: u32, canvas: &mut VulkanCanvas) -> u32 {
        let x = margin as i32;
        let line_width = canvas.width() - 2 * margin;
        let font_height = canvas.get_font_height();
        let cmd_line_height = font_height + 2 * 2;

        canvas.set_scissor(x, y0 - cmd_line_height as i32, line_width, cmd_line_height);
        canvas.set_wrap_mode(WrapMode::None);

        canvas.draw_text(x, y0 - 2, line_width, &self.cmd_line.buffer);

        cmd_line_height
    }

    fn dispatch_named_key(&mut self, key: &NamedKey) {
        match key {
            NamedKey::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_add(100);
            }
            NamedKey::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(100);
            }
            NamedKey::ArrowUp => {
                self.cmd_line.prev_command();
            }
            NamedKey::ArrowDown => {
                self.cmd_line.next_command();
            }
            NamedKey::ArrowLeft => {
                self.cmd_line.move_caret(-1);
            }
            NamedKey::ArrowRight => {
                self.cmd_line.move_caret(1);
            }
            NamedKey::Enter => {
                self.cmd_line.execute();
            }
            NamedKey::Tab => {
                self.cmd_line.complete_command();
            }
            NamedKey::Backspace => {
                self.cmd_line.delete_char_before_cursor();
            }
            NamedKey::Delete => {
                self.cmd_line.delete_char_at_cursor();
            }
            _ => {}
        }
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
        false
    }

    fn keyboard_input(
        &mut self,
        event: &winit::event::KeyEvent,
        modifiers: ModifiersState,
    ) -> bool {
        if self.opening && self.height > 0 {
            if event.state == ElementState::Pressed {
                match &event.logical_key.as_ref() {
                    Key::Named(named_key) => {
                        self.dispatch_named_key(named_key);
                    }
                    Key::Character("c") if modifiers.control_key() => {
                        // todo - process copy to clipboard
                    }
                    Key::Character("v") if modifiers.control_key() => {
                        // todo - process paste from clipboard
                    }
                    Key::Character(s) => {
                        if modifiers.control_key() {
                        } else {
                            self.cmd_line.push_at_caret(s);
                        }
                    }
                    _ => {}
                }
            }
            true
        } else {
            false
        }
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

        let line_spacing = 1;
        canvas.set_font(rg_common::ui::canvas::FontId::CONSOLE);
        canvas.set_color(Color::WHITE);
        canvas.set_line_spacing(line_spacing);
        canvas.set_wrap_mode(rg_common::ui::canvas::WrapMode::Word);

        // Check offset (should be multiple of font height to prevent text jumps)
        let extra = self.scroll_offset % (canvas.get_font_height() + line_spacing as u32);
        self.scroll_offset = self.scroll_offset.saturating_sub(extra);

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
    pub fn push_at_caret(&mut self, ch: &str) {
        self.buffer.insert_str(self.caret_pos as usize, ch);
        self.move_caret(ch.chars().count() as i32);
    }

    pub fn prev_command(&mut self) {}

    pub fn next_command(&mut self) {}

    pub fn move_caret(&mut self, delta: i32) {
        self.caret_pos = self
            .caret_pos
            .saturating_add(delta)
            .clamp(0, self.buffer.len() as i32)
    }

    pub fn execute(&mut self) {}

    pub fn complete_command(&mut self) {}

    pub fn delete_char_before_cursor(&mut self) {}

    pub fn delete_char_at_cursor(&mut self) {}
}
