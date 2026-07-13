use log::warn;
use rg_common::ui::{
    canvas::{Canvas, FontId, WrapMode},
    color::Color,
};

use crate::{
    pipelines::ui::{
        text::{TextLayout, ToGlyphInstance},
        ui::UiPipeline,
    },
    vertex::GlyphInstance,
};

const FULL_BLOCK: char = unsafe { char::from_u32_unchecked(0x2588) };

impl Canvas for UiPipeline {
    fn set_font(&mut self, id: FontId) {
        self.canvas_context.font_id = id;
    }

    fn set_line_spacing(&mut self, spacing: usize) {
        self.canvas_context.line_spacing = spacing;
    }

    fn set_color(&mut self, color: Color) {
        self.canvas_context.color = color;
    }

    fn set_wrap_mode(&mut self, mode: WrapMode) {
        self.canvas_context.wrap_mode = mode;
    }

    fn draw_text<S>(&mut self, x0: i32, y0: i32, width: u32, text: S)
    where
        S: AsRef<str>,
    {
        let ctx = &mut self.canvas_context;
        if let Some(font) = self.font_atlas.fonts.get(ctx.font_id.as_ref()) {
            let wrap_mode = ctx.wrap_mode;
            if let Err(e) = wrap_mode.layout(ctx, x0, y0, width, font, text) {
                warn!("Failed to layout text: {}", e.to_string());
            }
        }
    }

    fn measure_text<S>(&self, width: u32, text: S) -> u32
    where
        S: AsRef<str>,
    {
        let mut height = 0;
        let ctx = &self.canvas_context;
        if let Some(font) = self.font_atlas.fonts.get(ctx.font_id.as_ref()) {
            let wrap_mode = ctx.wrap_mode;
            match wrap_mode.measure(ctx, width, font, text) {
                Ok(h) => height = h,
                Err(e) => warn!("Failed to measure text: {}", e.to_string()),
            }
        }
        height as u32
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
        let ctx = &mut self.canvas_context;
        if let Some(font) = self.font_atlas.fonts.get(ctx.font_id.as_ref()) {
            if let Some(glyph) = font.get(FULL_BLOCK) {
                let mut g = glyph.to_glyph_instance(0, 0);
                g.color = ctx.color.into();
                g.pos = [x as i16, y as i16];
                g.size = [width as u16, height as u16];
                ctx.glyphs.push(g);
            } else {
                warn!("Full block character (0x2588) is not mapped!");
            }
        }
    }
}
