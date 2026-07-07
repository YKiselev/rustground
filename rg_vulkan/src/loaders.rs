use ab_glyph::FontVec;
use rg_common::{LoaderError, SeekAndRead, load_bytes};
use serde::Deserialize;

fn load_font(reader: &mut std::io::BufReader<SeekAndRead>) -> Result<Vec<u8>, LoaderError> {
    let bytes = load_bytes(reader)?;
    let font_vec = FontVec::try_from_vec(bytes).map_err(|e| LoaderError::Custom(e.to_string()))?;
    todo!()
}
