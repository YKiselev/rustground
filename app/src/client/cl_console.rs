use std::{fmt::Write, sync::Arc, time::Instant};

use rg_common::{
    App,
    ui::{
        canvas::{Canvas, WrapMode},
        color::Color,
    },
};
use rg_vulkan::renderer::VulkanCanvas;
use tracing::warn;
use winit::{
    event::ElementState,
    keyboard::{Key, ModifiersState, NamedKey},
};

use crate::{app_logger::AppLoggerBuffer, client::cl_ui_layer::UiLayer};

#[derive(Default)]
struct CommandLine {
    buffer: String,
    caret_pos: i32, // in characters
}

pub struct Console {
    app: Arc<App>,
    app_log_buffer: AppLoggerBuffer,
    height: u32,
    scroll_offset: u32, // how many lines to show from start not end
    autoscroll: bool,
    opening: bool,
    line_buf: String,
    cmd_line: CommandLine,
    cmd_line_offset: i32,
    time: Instant,
    completion_buf: String,
    completion_index: usize,
}

impl Console {
    pub fn new(app: Arc<App>, app_log_buffer: AppLoggerBuffer) -> Self {
        Self {
            app,
            app_log_buffer,
            height: 0,
            scroll_offset: 0,
            autoscroll: true,
            opening: false,
            line_buf: String::with_capacity(200),
            cmd_line: CommandLine::default(),
            cmd_line_offset: 0,
            time: Instant::now(),
            completion_buf: String::with_capacity(200),
            completion_index: 0,
        }
    }

    pub fn update(&mut self) {
        self.app_log_buffer.update();
    }

    fn draw_lines(
        &mut self,
        x0: i32,
        y0: i32,
        line_width: u32,
        line_height: u32,
        canvas: &mut VulkanCanvas,
    ) {
        canvas.set_scissor(x0, 0, line_width, y0 as u32);

        let line_count = self.app_log_buffer.iter().count() as u32;
        if !self.autoscroll && self.scroll_offset == line_count {
            self.autoscroll = true;
        }
        if self.scroll_offset > line_count || self.autoscroll {
            self.scroll_offset = line_count;
        }
        if self.scroll_offset < 5 && line_count >= 5 {
            self.scroll_offset = 5;
        }

        let x = x0;
        let mut y = y0.saturating_add_unsigned((line_count - self.scroll_offset) * line_height);

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

    fn draw_command_line(
        &mut self,
        x0: i32,
        y0: i32,
        line_width: u32,
        line_spacing: u32,
        canvas: &mut VulkanCanvas,
    ) -> u32 {
        let font_height = canvas.get_font_height();
        let char_width = canvas.get_char_width('_');
        let cmd_line_height = font_height + line_spacing;
        let mut x = x0;
        let y = y0.saturating_sub(cmd_line_height as i32);
        let caret_offset = self.cmd_line.caret_pos * char_width as i32;

        // Draw prompt
        canvas.draw_text(x, y, line_width, ">");
        x += (3 * char_width as i32) / 2;

        // Clip command line
        canvas.set_scissor(x, y, line_width, cmd_line_height + 8);

        // Check that caret position is in bounds
        if caret_offset as u32 + char_width > line_width + self.cmd_line_offset as u32 {
            self.cmd_line_offset = caret_offset + char_width as i32 - line_width as i32;
        } else if x + caret_offset < self.cmd_line_offset {
            self.cmd_line_offset = caret_offset;
        }
        x -= self.cmd_line_offset;

        canvas.set_wrap_mode(WrapMode::None);

        canvas.draw_text(x, y, line_width, &self.cmd_line.buffer);

        // Draw caret
        let is_visible = (self.time.elapsed().as_millis() >> 9) & 1 != 0;
        if is_visible {
            canvas.draw_text(x + caret_offset, y + 4, line_width, "\u{005F}");
        }

        cmd_line_height
    }

    fn dispatch_named_key(&mut self, key: &NamedKey) {
        match key {
            NamedKey::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
                self.autoscroll = false;
            }
            NamedKey::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_add(10);
                self.autoscroll = false;
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
                self.execute();
            }
            NamedKey::Tab => {
                self.complete_command();
            }
            NamedKey::Backspace => {
                self.cmd_line.delete_char_before_cursor();
            }
            NamedKey::Delete => {
                self.cmd_line.delete_char_at_cursor();
            }
            NamedKey::Home => {
                self.cmd_line.move_caret_to_start();
            }
            NamedKey::End => {
                self.cmd_line.move_caret_to_end();
            }
            NamedKey::Space => {
                self.push_at_caret(&" ");
            }
            _ => {}
        }
    }

    fn execute(&mut self) {
        if let Err(e) = self.app.commands.execute(&self.cmd_line.buffer) {
            warn!("{}", e);
        }
        self.cmd_line.clear();
        self.cmd_line_offset = 0;
    }

    fn complete_command(&mut self) {
        if self.completion_index == 0 && self.completion_buf.is_empty() {
            self.completion_buf.clear();

            self.app
                .commands
                .complete(&self.cmd_line.buffer, &mut self.completion_buf);

            let _ = self
                .app
                .vars
                .complete(&self.cmd_line.buffer, &mut self.completion_buf);
        }

        let comp_lines = self.completion_buf.lines().count();

        if comp_lines > 0 {
            self.cmd_line.clear();
            if let Some(completion) = self.completion_buf.lines().nth(self.completion_index) {
                self.cmd_line.push_at_caret(completion);
            }
        }

        if self.completion_index + 1 < comp_lines {
            self.completion_index += 1;
        } else {
            self.completion_index = 0;
        }
    }

    fn push_at_caret(&mut self, ch: &str) {
        self.cmd_line.push_at_caret(ch);
        self.completion_buf.clear();
        self.completion_index = 0;
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
                winit::event::MouseScrollDelta::PixelDelta(_physical_position) => {}
            },
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
                            self.push_at_caret(s);
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

        canvas.set_color(Color::BLACK.with_alpha(230));
        canvas.draw_rect(0, 0, canvas.width(), self.height);

        let line_spacing = 1u32;
        canvas.set_font(rg_common::ui::canvas::FontId::CONSOLE);
        canvas.set_color(Color::WHITE);
        canvas.set_line_spacing(line_spacing);
        canvas.set_wrap_mode(rg_common::ui::canvas::WrapMode::Word);

        // Check offset (should be multiple of font height to prevent text jumps)
        //let extra = self.scroll_offset % (canvas.get_font_height() + line_spacing as u32);
        //self.scroll_offset = self.scroll_offset.saturating_sub(extra);

        let font_height = canvas.get_font_height();
        let margin = 4;
        let line_width = canvas.width().saturating_sub(2 * margin);
        let line_height = font_height + line_spacing;
        let x = margin as i32;
        let mut y = self.height.saturating_sub(font_height) as i32;

        self.draw_command_line(x, y, line_width, line_spacing, canvas) as i32;

        y = y.saturating_sub_unsigned(line_height);

        self.draw_lines(x, y, line_width, line_height, canvas);
    }

    fn toggle(&mut self) {
        self.opening = !self.opening;
    }

    fn is_visible(&self) -> bool {
        self.height > 0 || self.opening
    }
}

impl CommandLine {
    fn push_at_caret(&mut self, ch: &str) {
        self.buffer.insert_str(self.caret_pos as usize, ch);
        self.move_caret(ch.chars().count() as i32);
    }

    fn prev_command(&mut self) {}

    fn next_command(&mut self) {}

    fn move_caret(&mut self, delta: i32) {
        self.caret_pos = self
            .caret_pos
            .saturating_add(delta)
            .clamp(0, self.buffer.len() as i32)
    }

    fn move_caret_to_start(&mut self) {
        self.caret_pos = 0;
    }

    fn move_caret_to_end(&mut self) {
        self.caret_pos = self.buffer.len() as i32;
    }

    fn delete_char_before_cursor(&mut self) {
        if self.caret_pos > 0 {
            self.move_caret(-1);
            self.delete_char_at_cursor();
        }
    }

    fn delete_char_at_cursor(&mut self) {
        if self.caret_pos < self.buffer.len() as i32 {
            self.buffer.remove(self.caret_pos as usize);
        }
    }

    fn clear(&mut self) {
        self.buffer.clear();
        self.caret_pos = 0;
    }
}
