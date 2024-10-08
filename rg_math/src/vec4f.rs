#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vector4f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vector4f {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Vector4f { x, y, z, w }
    }
}
