use argh::FromArgs;

/// Program arguments
#[derive(FromArgs)]
pub struct Arguments {
    /// run dedicated server
    #[argh(switch)]
    dedicated: bool,
    /// run client in windowed mode
    #[argh(switch)]
    windowed: bool,
}

impl Arguments {
    pub fn dedicated(&self) -> bool {
        self.dedicated
    }

    pub fn windowed(&self) -> bool {
        self.windowed
    }
}
