use crate::font::GlyphInfo;
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
            pos: [ gx, gy ],
            size: [ gw, gh ],
            color: 0xffffffff,
            uv_min: [ u0, v0 ],
            uv_max: [ u1, v1 ],
            layer_index: self.layer_index,
        }
    }
}
