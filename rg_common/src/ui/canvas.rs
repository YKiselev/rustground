use crate::Color;


pub enum WrapMode {
    None,
    Character,
    Word
}

pub struct StandardFont(&'static str);

impl StandardFont {
    pub const CONSOLE: StandardFont = StandardFont("console");
}

impl AsRef<str> for StandardFont {
    fn as_ref(&self) -> &str {
        self.0
    }
}

///
/// Canvas
/// 
pub trait Canvas {
    fn set_font<S>(name: S) where S: AsRef<str>;
    fn draw_text<S>(x: u32, y: u32, text: S, color: Color, wrap_mode: WrapMode) where S: AsRef<str>;
}