use rg_macros::VarBag;
use serde::{Deserialize, Serialize};

#[derive(Default, VarBag, Serialize, Deserialize)]
pub(crate) struct Config {
    pub windowed: bool,
    pub preferred_monitor: Option<String>,
    pub preferred_device: String,
    pub width: u32, // logical size
    pub height: u32, // logical size
    pub bit_depth: u16,
    pub refresh_rate: u32
}
