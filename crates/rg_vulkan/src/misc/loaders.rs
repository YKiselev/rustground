use std::{collections::HashMap, sync::Arc};

use ab_glyph::FontVec;
use ash::vk;
use rg_common::{App, LoaderError, SeekAndRead, load_bytes};
use serde::{Deserialize, Serialize};

use crate::{
    misc::context::VkContext,
    misc::font::{FontAtlasBuilder, VkFont, VkFontAtlas, optimize_ranges, to_char_set},
};

#[derive(Serialize, Deserialize)]
struct CharacterRange(u32, u32);

#[derive(Serialize, Deserialize)]
struct Font {
    name: String,
    size: u32,
    char_ranges: Vec<CharacterRange>,
}

#[derive(Serialize, Deserialize)]
struct Config {
    fonts: HashMap<String, Font>,
}

pub(crate) struct FontAtlasLoaderContext<'a> {
    instance: &'a VkContext,
    app: &'a Arc<App>,
    atlas_size: vk::Extent2D,
    scale_factor: f64,
}

impl<'a> FontAtlasLoaderContext<'a> {
    pub fn new(
        instance: &'a VkContext,
        app: &'a Arc<App>,
        atlas_size: vk::Extent2D,
        scale_factor: f64,
    ) -> Self {
        Self {
            instance,
            app,
            atlas_size,
            scale_factor,
        }
    }
}

pub(crate) fn load_font_atlas(
    reader: &mut std::io::BufReader<SeekAndRead>,
    ctx: &FontAtlasLoaderContext,
) -> Result<VkFontAtlas, LoaderError> {
    let bytes = load_bytes(reader, ())?;
    let config: Config =
        toml::from_slice(&bytes).map_err(|e| LoaderError::Custom(e.to_string()))?;

    let font_vecs: HashMap<String, (FontVec, u32, Vec<CharacterRange>)> = config
        .fonts
        .into_iter()
        .map(|(key, font)| {
            let font_vec = ctx.app.load_resource(font.name, &load_font, ctx)?;
            Ok::<_, LoaderError>((key, (font_vec, font.size, font.char_ranges)))
        })
        .into_iter()
        .collect::<Result<_, _>>()?;

    let mut font_glyphs = HashMap::with_capacity(font_vecs.len());
    let mut builder = FontAtlasBuilder::new(
        ctx.atlas_size.width as usize,
        ctx.atlas_size.height as usize,
    );
    for (key, (font_vec, font_size, char_ranges)) in font_vecs {
        let char_ranges = char_ranges.iter().map(|r| r.0..=r.1).collect();
        let optimized = optimize_ranges(&char_ranges);
        let chars = to_char_set(&optimized);
        let scaled_font_size = font_size as f64 * ctx.scale_factor;
        let glyph_infos = builder.add_font(&font_vec, scaled_font_size as f32, &chars)?;
        let font = VkFont::new(glyph_infos, scaled_font_size as u32);
        font_glyphs.insert(key, font);
    }
    let atlas_layers = builder.build()?;

    let image = ctx
        .instance
        .create_texture_image_from_pixels(
            ctx.atlas_size.width,
            ctx.atlas_size.height,
            &atlas_layers,
            vk::Format::R8_UNORM,
            vk::ImageViewType::TYPE_2D_ARRAY,
        )
        .map_err(|e| LoaderError::Custom(e.to_string()))?;

    let font = VkFontAtlas::new(font_glyphs, image);

    Ok(font)
}

fn load_font(
    reader: &mut std::io::BufReader<SeekAndRead>,
    _: &FontAtlasLoaderContext,
) -> Result<FontVec, LoaderError> {
    let bytes = load_bytes(reader, ())?;

    let font = FontVec::try_from_vec(bytes).map_err(|e| LoaderError::Custom(e.to_string()))?;

    Ok(font)
}
