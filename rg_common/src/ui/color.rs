
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Color(u32);

impl Color {
    ///
    /// Standard colors
    /// 
    pub const BLACK: Color = Color(0x000000ff);
    
    pub const RED: Color = Color(0xA80000ff);
    
    pub const GREEN: Color = Color(0x00A800ff);
    
    pub const YELLOW: Color = Color(0xA85400ff);
    
    pub const BLUE: Color = Color(0x0000A8ff);
    
    pub const MAGENTA: Color = Color(0xA800A8ff);
    
    pub const CYAN: Color = Color(0x00A8A8ff);
    
    pub const WHITE: Color = Color(0xA8A8A8ff);
    
    ///
    /// Bright (Light) Colors
    /// 
    pub const GRAY: Color = Color(0x545454ff);
    
    pub const LIGHT_RED: Color = Color(0xFC5454ff);
    
    pub const LIGHT_GREEN: Color = Color(0x54FC54ff);
    
    pub const LIGHT_YELLOW: Color = Color(0xFCFC54ff);
    
    pub const LIGHT_BLUE: Color = Color(0x5454FCff);
    
    pub const LIGHT_MAGENTA: Color = Color(0xFC54FCff);
    
    pub const LIGHT_CYAN: Color = Color(0x54FCFCff);
    
    pub const LIGHT_WHITE: Color = Color(0xFCFCFCff);
}

impl From<Color> for u32 {
    fn from(value: Color) -> Self {
        value.0
    }
}