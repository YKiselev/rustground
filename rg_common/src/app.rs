use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use log::info;

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
        let mut files = AppFiles::new(&args);
        let cfg = load_config("config.toml", &mut files);
        info!("Loaded config: {:?}", &cfg);
        Self {
            arguments: args,
            exit_flag: AtomicBool::new(false),
            started_at: Instant::now(),
            files: files,
            vars: VarRegistry::new(cfg),
            commands: CommandRegistry::default(),
        }
    }

    pub fn is_exit(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn save_config(&self, name: &str, value: String) {
        save_config(name, &self.files, value);
    }
}
