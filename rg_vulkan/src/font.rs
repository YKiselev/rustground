use ab_glyph::{Font, Glyph, PxScale, ScaleFont};
use ash::vk;
use cgmath::vec2;
use guillotiere::{AtlasAllocator, size2};
use rg_common::LoaderError;
use std::collections::HashMap;
use std::{cmp::max, ops::RangeInclusive};

use crate::image::VkImage;
use crate::types::Vec2;

use ab_glyph::FontVec;

///
/// Information about glyph on font atlas(нужна для генерации Vertex Buffer)
///
pub(crate) struct GlyphInfo {
    pub uv_min: Vec2,   // Top left corner in texture (0.0 - 1.0)
    pub uv_max: Vec2,   // Low right corner in texture (0.0 - 1.0)
    pub width: f32,     // Width (px)
    pub height: f32,    // Height (px)
    pub h_advance: f32, // Horizontal advance to next character in line
    pub offset: Vec2,   // Drawing offset
}

///
/// Font
///
pub(crate) struct VkFont {
    glyphs: HashMap<char, GlyphInfo>,
    image: VkImage,
}

impl VkFont {
    pub fn new(glyphs: HashMap<char, GlyphInfo>, image: VkImage) -> Self {
        Self { glyphs, image }
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        self.image.destroy(device);
    }
}

///
/// Helpres
///
pub(crate) fn optimize_ranges(source: &Vec<RangeInclusive<u32>>) -> Vec<RangeInclusive<u32>> {
    let mut sorted = Vec::clone(source);
    sorted.sort_by_key(|v| v.start().clone());

    let mut previous = sorted[0].clone();
    let mut result = Vec::with_capacity(source.len());

    for range in sorted.into_iter().skip(1) {
        if range.start() <= &(*previous.end() + 1) {
            previous = (*previous.start()..=max(*previous.end(), *range.end()));
        } else {
            result.push(previous);
            previous = range;
        }
    }
    result.push(previous);
    result
}

pub(crate) fn to_char_set(source: &Vec<RangeInclusive<u32>>) -> Vec<char> {
    let mut required_size = 0;
    for r in source.iter() {
        if r.is_empty() {
            continue;
        }
        required_size += r.end() - r.start() + 1;
    }

    let mut result = Vec::with_capacity(required_size as usize);

    for range in source.iter() {
        for k in range.clone() {
            if let Some(ch) = char::from_u32(k) {
                result.push(ch);
            }
        }
    }

    result
}

pub(crate) fn create_font_atlas(
    font: &FontVec,
    font_size: f32,
    atlas_size: vk::Extent2D,
    chars_to_pack: &Vec<char>,
) -> Result<(Vec<u8>, HashMap<char, GlyphInfo>), LoaderError> {
    let scale = PxScale::from(font_size);
    let scaled_font = font.as_scaled(scale);
    let data_size = atlas_size.width as usize * atlas_size.height as usize;
    let mut atlas_data = vec![0u8; data_size];

    // Init allocator
    let mut allocator =
        AtlasAllocator::new(size2(atlas_size.width as i32, atlas_size.height as i32));
    let mut glyph_map = HashMap::with_capacity(chars_to_pack.len());

    // Render and pack each character
    for (index, &ch) in chars_to_pack.iter().enumerate() {
        let glyph_id = font.glyph_id(ch);
        let glyph: Glyph = glyph_id.with_scale(scale);
        let h_advance = scaled_font.h_advance(glyph_id);

        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            let w = bounds.width() as u32;
            let h = bounds.height() as u32;

            // Asking allocator for space for new glyph (adding 1px offset to separate glyphs)
            if let Some(allocation) = allocator.allocate(size2(w as i32 + 1, h as i32 + 1)) {
                let rect = allocation.rectangle;

                // Rasterizing glyph in place
                outlined.draw(|x, y, coverage| {
                    let pixel_x = rect.min.x as u32 + x;
                    let pixel_y = rect.min.y as u32 + y;
                    if pixel_x < atlas_size.width as u32 && pixel_y < atlas_size.height as u32 {
                        let idx = (pixel_y * atlas_size.width as u32 + pixel_x) as usize;
                        atlas_data[idx] = (coverage * 255.0).round() as u8;
                    }
                });

                // Calculate normalized UV coordinates (0.0 - 1.0) for Vulkan shader
                let uv_min = vec2(
                    rect.min.x as f32 / atlas_size.width as f32,
                    rect.min.y as f32 / atlas_size.height as f32,
                );
                let uv_max = vec2(
                    (rect.min.x as u32 + w) as f32 / atlas_size.width as f32,
                    (rect.min.y as u32 + h) as f32 / atlas_size.height as f32,
                );

                glyph_map.insert(
                    ch,
                    GlyphInfo {
                        uv_min,
                        uv_max,
                        width: w as f32,
                        height: h as f32,
                        h_advance,
                        offset: vec2(bounds.min.x, bounds.min.y),
                    },
                );
            } else {
                return Err(LoaderError::Custom(format!(
                    "Font atlas overflow on char #{}/{}!",
                    index,
                    chars_to_pack.len()
                )));
            }
        } else {
            // Save horizontal advance for non-printable character
            glyph_map.insert(
                ch,
                GlyphInfo {
                    uv_min: vec2(0.0, 0.0),
                    uv_max: vec2(0.0, 0.0),
                    width: 0.0,
                    height: 0.0,
                    h_advance,
                    offset: vec2(0.0, 0.0),
                },
            );
        }
    }

    Ok((atlas_data, glyph_map))
}

#[cfg(test)]
mod tests {
    use crate::font::*;

    #[test]
    fn should_optimize() {
        let res = optimize_ranges(&vec![(2..=9), (0..=4), (5..=11), (100..=122)]);
        assert_eq!(vec![(0..=11), (100..=122)], res)
    }

    #[test]
    fn should_iterate_lazily() {
        let ranges = vec![(33..=45), (77..=82)];
        let result = to_char_set(&ranges);
        assert_eq!(
            vec![
                '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', 'M', 'N', 'O',
                'P', 'Q', 'R'
            ],
            result
        );
    }
}
