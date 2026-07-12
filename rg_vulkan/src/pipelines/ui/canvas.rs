use log::warn;
use rg_common::ui::{
    canvas::{Canvas, FontId, WrapMode},
    color::Color,
};

use crate::pipelines::ui::{
    text::{TextLayout, ToGlyphInstance},
    ui::{MAX_GLYPHS_PER_FRAME, UiPipeline},
};

impl Canvas for UiPipeline {
    fn set_font(&mut self, id: FontId) {
        self.canvas_font = id;
    }

    fn set_color(&mut self, color: Color) {
        self.canvas_color = color;
    }

    fn set_wrap_mode(&mut self, mode: WrapMode) {
        self.canvas_wrap_mode = mode;
    }

    fn draw_text<S>(&mut self, x0: i32, y0: i32, width: u32, text: S)
    where
        S: AsRef<str>,
    {
        if let Some(font) = self.font_atlas.fonts.get(self.canvas_font.as_ref()) {
            match self.canvas_wrap_mode.layout(
                &mut self.text_layout_scratch,
                x0,
                y0,
                width,
                0,
                font,
                self.canvas_color,
                text,
            ) {
                Ok(_) => {
                    let scratch = &mut self.text_layout_scratch;
                    if !scratch.line_lengths.is_empty() {
                        let mut x = x0 as i16;
                        let mut y = y0 as i16;
                        let mut offset = 0;
                        for &line_length in scratch.line_lengths.iter() {
                            for _ in 0..line_length {
                                let g = &mut scratch.glyphs[offset];
                                g.pos[0] += x;
                                g.pos[1] += y;
                                offset += 1;
                            }
                            x = x0 as i16;
                        }

                        self.glyph_buffer.append(&mut scratch.glyphs);
                    }
                }
                Err(e) => warn!("Failed to draw text: {}", e.to_string()),
            }
        }
    }

    fn measure_text<S>(&self, x: i32, y: i32, width: u32, text: S) -> u32
    where
        S: AsRef<str>,
    {
        todo!()
    }

    fn draw_sprite(
        &mut self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        sprite_id: rg_common::ui::canvas::SpriteId,
    ) {
        todo!()
    }

    fn draw_rect(&mut self, x: i32, y: i32, width: u32, height: u32) {
        todo!()
    }
}
