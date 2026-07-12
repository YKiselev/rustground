use rg_common::ui::canvas::WrapMode;
use rg_common::ui::color::Color;

use crate::error::VkError;
use crate::font::{GlyphInfo, VkFont};
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
/// Text layout scratch
///
#[derive(Default)]
pub(super) struct TextLayoutScratch {
    pub(super) glyphs: Vec<GlyphInstance>,
    pub(super) line_lengths: Vec<usize>,
}

///
/// Text layout
///
pub(super) trait TextLayout {
    fn layout<S>(
        &self,
        scratch: &mut TextLayoutScratch,
        x: i32,
        y: i32,
        width: u32,
        line_spacing: u32,
        font: &VkFont,
        color: Color,
        text: S,
    ) -> Result<(), VkError>
    where
        S: AsRef<str>;
}

impl TextLayout for WrapMode {
    fn layout<S>(
        &self,
        scratch: &mut TextLayoutScratch,
        x: i32,
        y: i32,
        width: u32,
        line_spacing: u32,
        font: &VkFont,
        color: Color,
        text: S,
    ) -> Result<(), VkError>
    where
        S: AsRef<str>,
    {
        match self {
            WrapMode::None => layout_no_wrap(scratch, x, y, width, line_spacing, font, color, text),
            WrapMode::Character => {
                layout_char_wrap(scratch, x, y, width, line_spacing, font, color, text)
            }
            WrapMode::Word => {
                layout_word_wrap(scratch, x, y, width, line_spacing, font, color, text)
            }
        }
    }
}

fn layout_no_wrap<S>(
    scratch: &mut TextLayoutScratch,
    x: i32,
    y: i32,
    width: u32,
    line_spacing: u32,
    font: &VkFont,
    color: Color,
    text: S,
) -> Result<(), VkError>
where
    S: AsRef<str>,
{
    scratch.glyphs.clear();
    scratch.line_lengths.clear();

    let mut color = color;
    let mut x = 0;
    let mut y = 0;

    for ch in text.as_ref().chars() {
        if let Some(glyph) = font.get(ch).or_else(|| font.get('?')) {
            let mut g = glyph.to_glyph_instance(x, y);
            g.color = color.into();
            scratch.glyphs.push(g);
            x += glyph.h_advance as i32;
        } else {
            return Err(VkError::GenericError(format!("No glyph for {}", ch)));
        }
    }

    scratch.line_lengths.push(scratch.glyphs.len());

    Ok(())
}

fn layout_char_wrap<S>(
    scratch: &mut TextLayoutScratch,
    x: i32,
    y: i32,
    width: u32,
    line_spacing: u32,
    font: &VkFont,
    color: Color,
    text: S,
) -> Result<(), VkError>
where
    S: AsRef<str>,
{
    scratch.glyphs.clear();
    scratch.line_lengths.clear();

    let mut color = color;
    let mut x = 0;
    let mut y = 0;
    let mut line_glyphs = 0;
    let line_height = (font.height + line_spacing) as i32;

    for ch in text.as_ref().chars() {
        if let Some(glyph) = font.get(ch).or_else(|| font.get('?')) {
            let mut g = glyph.to_glyph_instance(x, y);
            g.color = color.into();

            if x + glyph.h_advance as i32 > width as i32 && line_glyphs > 0 {
                scratch.line_lengths.push(line_glyphs);
                line_glyphs = 1;
                x = 0;
                y += line_height;
            } else {
                line_glyphs += 1;
                x += glyph.h_advance as i32;
            }

            scratch.glyphs.push(g);
        } else {
            return Err(VkError::GenericError(format!("No glyph for {}", ch)));
        }
    }

    if line_glyphs > 0 {
        scratch.line_lengths.push(line_glyphs);
    }

    Ok(())
}

fn layout_word_wrap<S>(
    scratch: &mut TextLayoutScratch,
    x: i32,
    y: i32,
    width: u32,
    line_spacing: u32,
    font: &VkFont,
    color: Color,
    text: S,
) -> Result<(), VkError>
where
    S: AsRef<str>,
{
    scratch.glyphs.clear();
    scratch.line_lengths.clear();

    let mut color = color;
    let line_height = (font.height + line_spacing) as i32;
    let mut x = 0;
    let mut y = 0;
    let mut line_start = 0;
    let mut word_start = 0;
    let mut is_whitespace = false;
    let mut i = 0;

    for ch in text.as_ref().chars() {
        if let Some(glyph) = font.get(ch).or_else(|| font.get('?')) {
            let mut g = glyph.to_glyph_instance(x, y);
            g.color = color.into();

            if ch.is_whitespace() {
                is_whitespace = true;
            } else if is_whitespace {
                word_start = i;
                is_whitespace = false
            }

            let mut fixup = false;
            if x + glyph.h_advance as i32 > width as i32 && word_start > line_start {
                let word_glyphs = i - word_start + 1;
                let line_length = i - line_start + 1 - word_glyphs;
                scratch.line_lengths.push(line_length);
                line_start = word_start;
                x = 0;
                y += line_height;
                fixup = word_glyphs > 0;
            } else {
                x += glyph.h_advance as i32;
            }

            scratch.glyphs.push(g);

            if fixup {
                let mut x = 0;
                for g in scratch.glyphs[word_start..].iter_mut() {
                    g.pos[0] -= old_x as i16;
                    g.pos[1] += line_height as i16;
                }
            }
        } else {
            return Err(VkError::GenericError(format!("No glyph for {}", ch)));
        }
        i += 1;
    }

    if i > line_start {
        scratch.line_lengths.push(i - line_start);
    }

    let a: usize = scratch.line_lengths.iter().sum();
    if a > scratch.glyphs.len() {
        println!("Oops: {} {}", a, scratch.glyphs.len())
    }

    Ok(())
}
