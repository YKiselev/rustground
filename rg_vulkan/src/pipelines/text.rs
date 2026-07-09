use crate::font::GlyphInfo;
use crate::types::Vec2i16;
use crate::types::Vec2u16;
use crate::types::Vec4u16;
use crate::vertex::GlyphInstance;

pub(crate) trait ToGlyphInstance {
    fn to_glyph_instance(&self, x: i32, y: i32) -> GlyphInstance;
}

impl ToGlyphInstance for GlyphInfo {
    fn to_glyph_instance(&self, x: i32, y: i32) -> GlyphInstance {
        assert!( self.uv_min.x >= 0.0 && self.uv_min.y >= 0.0);
        assert!( self.uv_max.x >= 0.0 && self.uv_max.y >= 0.0);
        assert!( self.width >= 0.0 && self.height >= 0.0 );

        let gx = (x + self.offset.x as i32) as i16;
        let gy = (y + self.offset.y as i32) as i16;
        let gw = self.width as u16;
        let gh = self.height as u16;
        let u0 = (self.uv_min.x * 65535.0) as u16;
        let v0 = (self.uv_min.y * 65535.0) as u16;
        let u1 = (self.uv_max.x * 65535.0) as u16;
        let v1 = (self.uv_max.y * 65535.0) as u16;

        GlyphInstance {
            pos: Vec2i16 { x: gx, y: gy },
            size: Vec2u16 { x: gw, y: gh },
            color: Vec4u16 {
                x: 65535,
                y: 65535,
                z: 65535,
                w: 65535,
            },
            uv_min: Vec2u16 { x: u0, y: v0 },
            uv_max: Vec2u16 { x: u1, y: v1 },
            layer_index: self.layer_index,
        }
    }
}
