///
/// Color (AABBGGRR)
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Color(u32);

impl Color {
    ///
    /// Standard colors
    ///
    pub const BLACK: Color = Color::from_rgb(0, 0, 0);

    pub const RED: Color = Color::from_rgb(0xA8, 0, 0);

    pub const GREEN: Color = Color::from_rgb(0, 0xA8, 0);

    pub const YELLOW: Color = Color::from_rgb(0xA8, 0xA8, 0);

    pub const BLUE: Color = Color::from_rgb(0, 0, 0xA8);

    pub const MAGENTA: Color = Color::from_rgb(0xA8, 0, 0xA8);

    pub const CYAN: Color = Color::from_rgb(0, 0xA8, 0xA8);

    pub const WHITE: Color = Color::from_rgb(0xA8, 0xA8, 0xA8);

    ///
    /// Bright (Light) Colors
    ///
    pub const GRAY: Color = Color::from_rgb(0x54, 0x54, 0x54);

    pub const LIGHT_RED: Color = Color::from_rgb(0xFC, 0x54, 0x54);

    pub const LIGHT_GREEN: Color = Color::from_rgb(0x54, 0xFC, 0x54);

    pub const LIGHT_YELLOW: Color = Color::from_rgb(0xFC, 0xFC, 0x54);

    pub const LIGHT_BLUE: Color = Color::from_rgb(0x54, 0x54, 0xFC);

    pub const LIGHT_MAGENTA: Color = Color::from_rgb(0xFC, 0x54, 0xFC);

    pub const LIGHT_CYAN: Color = Color::from_rgb(0x54, 0xFC, 0xFC);

    pub const LIGHT_WHITE: Color = Color::from_rgb(0xFC, 0xFC, 0xFC);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(u32::from_le_bytes([r, g, b, a]))
    }

    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 0xFF)
    }
}

impl From<Color> for u32 {
    fn from(value: Color) -> Self {
        value.0
    }
}

const STANDARD_COLORS: [Color; 16] = [
    Color::BLACK,
    Color::RED,
    Color::GREEN,
    Color::YELLOW,
    Color::BLUE,
    Color::MAGENTA,
    Color::CYAN,
    Color::WHITE,
    Color::GRAY,
    Color::LIGHT_RED,
    Color::LIGHT_GREEN,
    Color::LIGHT_YELLOW,
    Color::LIGHT_BLUE,
    Color::LIGHT_MAGENTA,
    Color::LIGHT_CYAN,
    Color::LIGHT_YELLOW,
];
