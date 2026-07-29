use std::fmt::Write;
use std::rc::Rc;

use rg_common::ui::{canvas::Canvas, color::Color};

use crate::{app_logger::AppLoggerBuffer, client::cl_ui_layer::UiLayer};

pub struct Console {
    app_log_buffer: AppLoggerBuffer,
    height: u32,
    opening: bool,
    line_buf: String
}

impl Console {
    pub fn new(app_log_buffer: AppLoggerBuffer) -> Self {
        Self {
            app_log_buffer,
            height: 0,
            opening: false,
            line_buf: String::with_capacity(200)
        }
    }

    pub fn update(&mut self) {
        self.app_log_buffer.update();
    }
}

impl UiLayer for Console {
    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _event: &winit::event::DeviceEvent,
    ) -> bool {
        false
    }

    fn draw(&mut self, canvas: &mut rg_vulkan::renderer::VulkanCanvas) {
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
        let x = margin as i32;
        let mut y = self.height as i32;
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

                canvas.draw_text(x, y, line_width, &self.line_buf);
            } else {
                break;
            }

            if y < 0 {
                break;
            }
        }
    }

    fn toggle(&mut self) {
        self.opening = !self.opening;
    }

    fn is_visible(&self) -> bool {
        self.height > 0 || self.opening
    }
}
