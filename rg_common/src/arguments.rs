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

///
/// Program arguments
///
#[derive(FromArgs, Debug)]
pub struct Arguments {
    /// run dedicated server
    #[argh(switch)]
    pub dedicated: Option<bool>,

    /// run client in windowed mode
    #[argh(switch)]
    pub windowed: Option<bool>,

    /// window width
    #[argh(option)]
    pub width: Option<u32>,

    /// window height
    #[argh(option)]
    pub height: Option<u32>,

    /// bit depth (bpp)
    #[argh(option)]
    pub bit_depth: Option<u16>,

    /// refresh rate (Hz)
    #[argh(option)]
    pub refresh_rate: Option<u32>,

    /// v-sync mode
    #[argh(option)]
    pub v_sync: Option<VSyncMode>,

    /// log level filter
    #[argh(option)]
    pub log_level: Option<LevelFilter>,
}
