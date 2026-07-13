use rg_common::ui::canvas::{FontId, WrapMode};
use rg_common::ui::color::Color;

use crate::error::VkError;
use crate::font::{GlyphInfo, VkFont};
use crate::pipelines::ui::ui::DEFAULT_GLYPH_BUFFER_SIZE;
use crate::vertex::GlyphInstance;

pub(crate) trait ToGlyphInstance {
    fn to_glyph_instance(&self, x: i32, y: i32) -> GlyphInstance;
}

impl ToGlyphInstance for GlyphInfo {
    fn to_glyph_instance(&self, x: i32, y: i32) -> GlyphInstance {
        let gx = (x + self.offset.x as i32) as i16;
        let gy = (y + self.offset.y as i32) as i16;
        let gw = self.width as u16;
        let gh = self.height as u16;
        let u0 = (self.uv_min.x * 65535.0) as u16;
        let v0 = (self.uv_min.y * 65535.0) as u16;
        let u1 = (self.uv_max.x * 65535.0) as u16;
        let v1 = (self.uv_max.y * 65535.0) as u16;

        GlyphInstance {
            pos: [gx, gy],
            size: [gw, gh],
            color: 0xffffffff,
            uv_min: [u0, v0],
            uv_max: [u1, v1],
            layer_index: self.layer_index,
        }
    }
}

///
/// Canvas context
///
pub(super) struct CanvasContext {
    pub font_id: FontId,
    pub color: Color,
    pub wrap_mode: WrapMode,
    pub line_spacing: usize,
    pub glyphs: Vec<GlyphInstance>,
    pub line_lengths: Vec<usize>,
}

impl CanvasContext {
    pub fn new() -> Self {
        Self {
            font_id: FontId::DEFAULT,
            color: Color::WHITE,
            wrap_mode: WrapMode::None,
            line_spacing: 0,
            glyphs: Vec::with_capacity(DEFAULT_GLYPH_BUFFER_SIZE),
            line_lengths: Vec::new(),
        }
    }
}

///
/// Text layout
///
pub(super) trait TextLayout {
    fn layout<S>(
        &self,
        context: &mut CanvasContext,
        x: i32,
        y: i32,
        width: u32,
        font: &VkFont,
        text: S,
    ) -> Result<(), VkError>
    where
        S: AsRef<str>;

    fn measure<S>(
        &self,
        context: &CanvasContext,
        width: u32,
        font: &VkFont,
        text: S,
    ) -> Result<i32, VkError>
    where
        S: AsRef<str>;
}

impl TextLayout for WrapMode {
    fn layout<S>(
        &self,
        context: &mut CanvasContext,
        x: i32,
        y: i32,
        width: u32,
        font: &VkFont,
        text: S,
    ) -> Result<(), VkError>
    where
        S: AsRef<str>,
    {
        match self {
            WrapMode::None => layout_no_wrap(context, x, y, font, text.as_ref()),
            WrapMode::Character => layout_char_wrap(context, x, y, width, font, text.as_ref()),
            WrapMode::Word => layout_word_wrap(context, x, y, width, font, text.as_ref()),
        }
    }

    fn measure<S>(
        &self,
        context: &CanvasContext,
        width: u32,
        font: &VkFont,
        text: S,
    ) -> Result<i32, VkError>
    where
        S: AsRef<str>,
    {
        match self {
            WrapMode::None => measure_no_wrap(context, font, text.as_ref()),
            WrapMode::Character => measure_char_wrap(context, width, font, text.as_ref()),
            WrapMode::Word => measure_word_wrap(context, width, font, text.as_ref()),
        }
    }
}

fn layout_no_wrap(
    context: &mut CanvasContext,
    x0: i32,
    y0: i32,
    font: &VkFont,
    text: &str,
) -> Result<(), VkError> {
    context.line_lengths.clear();

    let color = context.color;
    let mut x = x0;

    for ch in text.chars() {
        if let Some(glyph) = font.get(ch).or_else(|| font.get('?')) {
            let mut g = glyph.to_glyph_instance(x, y0);
            g.color = color.into();
            context.glyphs.push(g);
            x += glyph.h_advance as i32;
        } else {
            return Err(VkError::GenericError(format!("No glyph for {}", ch)));
        }
    }

    context.line_lengths.push(context.glyphs.len());

    Ok(())
}

fn measure_no_wrap(context: &CanvasContext, font: &VkFont, text: &str) -> Result<i32, VkError> {
    let line_height = (font.height + context.line_spacing as u32) as i32;

    Ok(line_height)
}

fn layout_char_wrap(
    context: &mut CanvasContext,
    x0: i32,
    y0: i32,
    width: u32,
    font: &VkFont,
    text: &str,
) -> Result<(), VkError> {
    context.line_lengths.clear();

    let color = context.color;
    let line_height = (font.height + context.line_spacing as u32) as i32;
    let right_margin = x0 + width as i32;
    let mut x = x0;
    let mut y = y0;
    let mut line_glyphs = 0;
    let mut slice = text;
    let mut it = slice.char_indices();

    while let Some((idx, ch)) = it.next() {
        if let Some(glyph) = font.get(ch).or_else(|| font.get('?')) {
            let mut g = glyph.to_glyph_instance(x, y);
            g.color = color.into();

            if x + glyph.h_advance as i32 > right_margin as i32 && line_glyphs > 0 {
                context.line_lengths.push(line_glyphs);
                line_glyphs = 1;
                x = x0;
                y += line_height;
                slice = &slice[idx..];
                it = slice.char_indices();
                continue;
            } else {
                line_glyphs += 1;
                x += glyph.h_advance as i32;
            }

            context.glyphs.push(g);
        } else {
            return Err(VkError::GenericError(format!("No glyph for {}", ch)));
        }
    }

    if line_glyphs > 0 {
        context.line_lengths.push(line_glyphs);
    }

    Ok(())
}

fn measure_char_wrap(
    context: &CanvasContext,
    width: u32,
    font: &VkFont,
    text: &str,
) -> Result<i32, VkError> {
    let line_height = (font.height + context.line_spacing as u32) as i32;
    let right_margin = width as i32;
    let mut x = 0;
    let mut y = 0;
    let mut line_glyphs = 0;
    let mut slice = text;
    let mut it = slice.char_indices();

    while let Some((idx, ch)) = it.next() {
        if let Some(glyph) = font.get(ch).or_else(|| font.get('?')) {
            if x + glyph.h_advance as i32 > right_margin as i32 && line_glyphs > 0 {
                line_glyphs = 1;
                x = 0;
                y += line_height;
                slice = &slice[idx..];
                it = slice.char_indices();
                continue;
            } else {
                line_glyphs += 1;
                x += glyph.h_advance as i32;
            }
        } else {
            return Err(VkError::GenericError(format!("No glyph for {}", ch)));
        }
    }

    if line_glyphs > 0 {
        y += line_height;
    }

    Ok(y)
}

fn layout_word_wrap(
    context: &mut CanvasContext,
    x0: i32,
    y0: i32,
    width: u32,
    font: &VkFont,
    text: &str,
) -> Result<(), VkError> {
    context.line_lengths.clear();

    let line_height = (font.height + context.line_spacing as u32) as i32;
    let right_margin = x0 + width as i32;
    let mut slice = text;
    let mut color = context.color;
    let mut x = x0;
    let mut y = y0;
    let mut line_start = 0;
    let mut word_start = 0;
    let mut is_whitespace = false;
    let mut i = 0;

    let mut it = slice.char_indices();
    let mut word_start_idx = 0;
    while let Some((idx, ch)) = it.next() {
        if let Some(glyph) = font.get(ch).or_else(|| font.get('?')) {
            let mut g = glyph.to_glyph_instance(x, y);
            g.color = color.into();

            if ch.is_whitespace() {
                is_whitespace = true;
            } else if is_whitespace {
                word_start = i;
                word_start_idx = idx;
                is_whitespace = false
            }

            if x + glyph.h_advance as i32 > right_margin as i32 && word_start > line_start {
                let word_glyphs = i - word_start; // not counting this char, because we won't put its glyph into buffer!
                let line_length = word_start - line_start;
                context.line_lengths.push(line_length);
                line_start = word_start;
                x = x0;
                y += line_height;
                is_whitespace = false;
                slice = &slice[word_start_idx..];
                it = slice.char_indices();
                context.glyphs.truncate(context.glyphs.len() - word_glyphs);
                continue;
            } else {
                x += glyph.h_advance as i32;
            }

            context.glyphs.push(g);
        } else {
            return Err(VkError::GenericError(format!("No glyph for {}", ch)));
        }
        i += 1;
    }

    if i > line_start {
        context.line_lengths.push(i - line_start);
    }

    Ok(())
}

fn measure_word_wrap(
    context: &CanvasContext,
    width: u32,
    font: &VkFont,
    text: &str,
) -> Result<i32, VkError> {
    let line_height = (font.height + context.line_spacing as u32) as i32;
    let right_margin = width as i32;
    let mut slice = text;
    let mut x = 0;
    let mut y = 0;
    let mut line_start = 0;
    let mut word_start = 0;
    let mut is_whitespace = false;
    let mut i = 0;

    let mut it = slice.char_indices();
    let mut word_start_idx = 0;
    while let Some((idx, ch)) = it.next() {
        if let Some(glyph) = font.get(ch).or_else(|| font.get('?')) {

            if ch.is_whitespace() {
                is_whitespace = true;
            } else if is_whitespace {
                word_start = i;
                word_start_idx = idx;
                is_whitespace = false
            }

            if x + glyph.h_advance as i32 > right_margin as i32 && word_start > line_start {
                line_start = word_start;
                x = 0;
                y += line_height;
                is_whitespace = false;
                slice = &slice[word_start_idx..];
                it = slice.char_indices();
                continue;
            } else {
                x += glyph.h_advance as i32;
            }
        } else {
            return Err(VkError::GenericError(format!("No glyph for {}", ch)));
        }
        i += 1;
    }

    if i > line_start {
        y += line_height;
    }

    Ok(y)
}
