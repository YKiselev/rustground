use std::{fmt::Display, sync::Arc};

use crate::ui::color::Color;

pub enum WrapMode {
    None,
    Character,
    Word,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum FontId {
    Static(&'static str),
    Dynamic(Arc<str>),
}

impl FontId {
    pub const CONSOLE: FontId = FontId::from_str("console");

    pub const DEFAULT: FontId = FontId::from_str("default");

    pub const SMALL: FontId = FontId::from_str("small");

    pub const LARGE: FontId = FontId::from_str("large");

    pub const fn from_str(s: &'static str) -> Self {
        Self::Static(s)
    }

    pub fn new<S>(s: S) -> Self
    where
        S: AsRef<str>,
    {
        Self::Dynamic(Arc::from(s.as_ref()))
    }
}

impl AsRef<str> for FontId {
    fn as_ref(&self) -> &str {
        match self {
            FontId::Static(s) => *s,
            FontId::Dynamic(s) => s.as_ref(),
        }
    }
}

impl Display for FontId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontId::Static(s) => write!(f, "{}", s),
            FontId::Dynamic(s) => write!(f, "{}", s.as_ref()),
        }
    }
}

///
/// Sprite ID
///
pub struct SpriteId(u32);

///
/// Canvas
///
pub trait Canvas {
    fn set_font(&mut self, id: FontId);

    fn set_color(&mut self, color: Color);

    fn set_wrap_mode(&mut self, mode: WrapMode);

    fn draw_text<S>(&mut self, x: i32, y: i32, width: u32, text: S)
    where
        S: AsRef<str>;
    
    fn measure_text<S>(&self, x: i32, y: i32, width: u32, text: S) -> u32
    where
        S: AsRef<str>;
    
    fn draw_sprite(&mut self, x: i32, y: i32, width: u32, height: u32, sprite_id: SpriteId);
    
    fn draw_rect(&mut self, x: i32, y: i32, width: u32, height: u32);
}
