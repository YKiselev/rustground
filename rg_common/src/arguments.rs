use std::default;

use argh::{FromArgValue, FromArgs};
use log::LevelFilter;

#[derive(FromArgValue, Debug)]
pub enum VSyncMode {
    Off,
    Triple,
    Adaptive,
    On,
}

fn def_width() -> u32 {
    1024
}

fn def_height() -> u32 {
    768
}

///
/// Program arguments
///
#[derive(FromArgs, Debug)]
pub struct Arguments {
    /// run dedicated server
    #[argh(switch)]
    pub dedicated: bool,

    /// run client in windowed mode
    #[argh(switch)]
    pub windowed: bool,

    /// window width
    #[argh(option, default = "def_width()")]
    pub width: u32,
    
    /// window height
    #[argh(option, default = "def_height()")]
    pub height: u32,

    /// v-sync mode
    #[argh(option, default = "VSyncMode::Triple")]
    pub v_sync: VSyncMode,

    /// log level filter
    #[argh(option, default = "LevelFilter::Debug")]
    pub log_level: LevelFilter,
}
