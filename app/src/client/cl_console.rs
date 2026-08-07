use std::{
    collections::VecDeque,
    fmt::Write,
    sync::{Arc, Mutex, atomic::Ordering},
    time::Instant,
};

use rg_common::{
    App, FromStrMutator,
    ui::{
        canvas::{Canvas, WrapMode},
        color::Color,
    },
};
use rg_vulkan::renderer::VulkanCanvas;
use tracing::warn;
use winit::{
    event::{ElementState, WindowEvent},
    keyboard::{Key, ModifiersState, NamedKey},
};

use crate::{
    app_logger::AppLoggerBuffer,
    client::{SharedState, cl_ui_layer::UiLayer},
};

struct CommandLine {
    buffer: String,
    caret_pos: i32, // in characters
    max_history: usize,
    history: VecDeque<String>,
    history_index: usize,
    free: Vec<String>,
}

pub struct Console {
    app: Arc<App>,
    app_log_buffer: AppLoggerBuffer,
    height: u32,
    scroll_offset: u32, // how many lines to show from start, not end
    autoscroll: bool,
    opened_at: Option<Instant>,
    line_buf: String,
    cmd_line: CommandLine,
    cmd_line_offset: i32,
    time: Instant,
    completion_buf: String,
    completion_index: usize,
}

const DEF_CMD_LINE: usize = 80;

impl Console {
    pub fn new(app: Arc<App>, app_log_buffer: AppLoggerBuffer) -> Self {
        Self {
            app,
            app_log_buffer,
            height: 0,
            scroll_offset: 0,
            autoscroll: true,
            opened_at: None,
            line_buf: String::with_capacity(200),
            cmd_line: CommandLine {
                buffer: String::with_capacity(DEF_CMD_LINE),
                caret_pos: 0,
                max_history: 10,
                history: VecDeque::default(),
                history_index: 0,
                free: Vec::default(),
            },
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

    fn dispatch_named_key(&mut self, key: &NamedKey) -> bool {
        match key {
            NamedKey::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
                self.autoscroll = false;
                true
            }
            NamedKey::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_add(10);
                self.autoscroll = false;
                true
            }
            NamedKey::ArrowUp => {
                self.cmd_line.prev_command();
                true
            }
            NamedKey::ArrowDown => {
                self.cmd_line.next_command();
                true
            }
            NamedKey::ArrowLeft => {
                self.cmd_line.move_caret(-1);
                true
            }
            NamedKey::ArrowRight => {
                self.cmd_line.move_caret(1);
                true
            }
            NamedKey::Enter => {
                self.execute();
                true
            }
            NamedKey::Tab => {
                self.complete_command();
                true
            }
            NamedKey::Backspace => {
                self.cmd_line.delete_char_before_cursor();
                true
            }
            NamedKey::Delete => {
                self.cmd_line.delete_char_at_cursor();
                true
            }
            NamedKey::Home => {
                self.cmd_line.move_caret_to_start();
                true
            }
            NamedKey::End => {
                self.cmd_line.move_caret_to_end();
                true
            }
            NamedKey::Space => {
                self.push_at_caret(&" ");
                true
            }
            _ => false,
        }
    }

    fn execute(&mut self) {
        if let Some(cmd_line) = self.cmd_line.remember() {
            if let Err(e) = self.app.commands.execute(cmd_line) {
                warn!("{}", e);
            }
            self.cmd_line_offset = 0;
        }
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

    fn is_opening(&self) -> bool {
        self.opened_at.is_some()
    }

    fn is_input_ready(&self) -> bool {
        self.opened_at
            .map_or(false, |t| t.elapsed().as_millis() > 300)
    }
}

impl UiLayer for Console {
    fn window_event(
        &mut self,
        event: &winit::event::WindowEvent,
        modifiers: ModifiersState,
    ) -> bool {
        if !self.is_input_ready() || self.height == 0 {
            return false;
        }

        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    match &event.logical_key.as_ref() {
                        Key::Named(named_key) => {
                            return self.dispatch_named_key(named_key);
                        }
                        Key::Character("c") if modifiers.control_key() => {
                            // todo - process copy to clipboard
                        }
                        Key::Character("v") if modifiers.control_key() => {
                            // todo - process paste from clipboard
                        }
                        Key::Character(s) => {
                            if modifiers.control_key() || modifiers.alt_key() {
                            } else {
                                self.push_at_caret(s);
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        false
    }

    fn draw(&mut self, canvas: &mut VulkanCanvas) {
        if self.is_opening() {
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
        self.opened_at = if self.opened_at.is_none() {
            Some(Instant::now())
        } else {
            None
        }
    }

    fn is_visible(&self) -> bool {
        self.height > 0 || self.is_opening()
    }
}

impl CommandLine {
    fn push_at_caret(&mut self, ch: &str) {
        if let Some(idx) = self
            .buffer
            .char_indices()
            .nth(self.caret_pos as usize)
            .map(|(i, _)| i)
            .or_else(|| {
                if self.buffer.len() == self.caret_pos as usize {
                    Some(self.buffer.len())
                } else {
                    None
                }
            })
        {
            self.buffer.insert_str(idx, ch);
            self.move_caret(ch.chars().count() as i32);
        }
    }

    fn prev_command(&mut self) {
        self.history_index = self.history_index.saturating_sub(1);
        self.set_from_history();
    }

    fn next_command(&mut self) {
        self.history_index = (self.history_index + 1).clamp(0, self.history.len());
        self.set_from_history();
    }

    fn set_from_history(&mut self) {
        self.clear();
        if let Some(s) = self.history.get(self.history_index) {
            self.buffer.push_str(s);
            self.caret_pos = self.buffer.len() as i32;
        }
    }

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

    fn remember(&mut self) -> Option<&String> {
        if let Some(index) = self.history.iter().position(|s| *s == self.buffer) {
            self.history.remove(index);
        }
        if self.history.len() >= self.max_history {
            if let Some(mut s) = self.history.pop_front() {
                s.clear();
                self.free.push(s);
            }
        }
        let new_buf = self.free.pop().unwrap_or_else(|| String::with_capacity(80));
        let buf = std::mem::replace(&mut self.buffer, new_buf);
        self.clear();
        self.history.push_back(buf);
        self.history_index = self.history.len();
        self.history.back()
    }
}
