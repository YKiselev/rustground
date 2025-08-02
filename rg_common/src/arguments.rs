
use argh::FromArgs;
use log::LevelFilter;

/// Program arguments
#[derive(FromArgs, Debug)]
pub struct Arguments {
    /// run dedicated server
    #[argh(switch)]
    dedicated: bool,
    /// run client in windowed mode
    #[argh(switch)]
    windowed: bool,

    /// log level filter
    #[argh(option, default="LevelFilter::Debug")]
    log_level: LevelFilter
}

impl Arguments {
    pub fn dedicated(&self) -> bool {
        self.dedicated
    }

    pub fn windowed(&self) -> bool {
        self.windowed
    }

    pub fn log_level(&self) -> LevelFilter {
        self.log_level
    }
}
