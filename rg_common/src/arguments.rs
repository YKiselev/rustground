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
    pub dedicated: bool,

    /// run client in windowed mode
    #[argh(switch)]
    pub windowed: bool,

    /// v-sync mode
    #[argh(option, default = "VSyncMode::Triple")]
    pub v_sync: VSyncMode,

    /// log level filter
    #[argh(option, default = "LevelFilter::Debug")]
    pub log_level: LevelFilter,
}
