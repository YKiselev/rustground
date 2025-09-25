use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use log::{info, warn};

use rg_common::arguments::Arguments;
use rg_common::{AppFiles, CommandRegistry, VarRegistry};

use crate::config::load_config;
use crate::save_config;

#[derive(Debug)]
pub struct App {
    pub arguments: Arguments,
    pub exit_flag: AtomicBool,
    pub started_at: Instant,
    pub files: AppFiles,
    pub vars: VarRegistry,
    pub commands: CommandRegistry,
}

impl App {
    pub fn new(args: Arguments) -> Self {
        let files = AppFiles::new(&args);
        Self {
            arguments: args,
            exit_flag: AtomicBool::new(false),
            started_at: Instant::now(),
            files: files,
            vars: VarRegistry::new(None),
            commands: CommandRegistry::default(),
        }
    }

    pub fn is_exit(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn load_config<S>(&self, name: S)
    where
        S: AsRef<str>,
    {
        if let Some(cfg) = load_config(name.as_ref(), &self.files) {
            info!("Loaded config: {:?}", &cfg);
            let _ = self
                .vars
                .set_table(cfg)
                .inspect_err(|e| warn!("Unable to load {}: {:?}", name.as_ref(), e));
        }
    }

    pub fn save_config(&self, name: &str, value: String) {
        save_config(name, &self.files, value);
    }
}
