use std::ops::RangeInclusive;

use ab_glyph::FontVec;
use ash::vk;
use rg_common::{LoaderError, SeekAndRead, load_bytes};

use crate::{
    font::{VkFont, create_font_atlas, optimize_ranges, to_char_set},
    instance::VkInstance,
};

pub(crate) struct FontLoaderContext<'a> {
    instance: &'a VkInstance,
    size: u32,
    char_set: Vec<char>,
    atlas_size: vk::Extent2D,
}

impl<'a> FontLoaderContext<'a> {
    pub fn new(
        instance: &'a VkInstance,
        size: u32,
        ranges: Vec<RangeInclusive<u32>>,
        atlas_size: vk::Extent2D,
    ) -> Self {
        let optimized = optimize_ranges(&ranges);
        let char_set = to_char_set(&optimized);
        Self {
            instance,
            size,
            char_set,
            atlas_size,
        }
    }
}

pub(crate) fn load_font(
    reader: &mut std::io::BufReader<SeekAndRead>,
    ctx: &FontLoaderContext,
) -> Result<VkFont, LoaderError> {
    let bytes = load_bytes(reader, ())?;

    let font = FontVec::try_from_vec(bytes).map_err(|e| LoaderError::Custom(e.to_string()))?;

    let (pixels, glyphs) =
        create_font_atlas(&font, ctx.size as f32, ctx.atlas_size, &ctx.char_set)?;

    let image = ctx
        .instance
        .create_texture_image_from_pixels(
            ctx.atlas_size.width,
            ctx.atlas_size.height,
            pixels,
            vk::Format::R8_UNORM,
        )
        .map_err(|e| LoaderError::Custom(e.to_string()))?;

    let font = VkFont::new(glyphs, image);
    
    Ok(font)
}
