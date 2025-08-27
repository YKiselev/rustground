use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use log::info;

use rg_common::arguments::Arguments;
use rg_common::{AppFiles, CommandRegistry, VarRegistry};

use rg_common::config::Config;

#[derive(Debug)]
pub struct App {
    pub arguments: Arguments,
    pub exit_flag: AtomicBool,
    pub started_at: Instant,
    pub config: Arc<Mutex<Config>>,
    pub files: Arc<Mutex<AppFiles>>,
    pub vars: VarRegistry,
    pub commands: CommandRegistry,
}

impl App {
    pub fn new(args: Arguments) -> Self {
        let mut files = AppFiles::new(&args);
        let cfg = Arc::new(Mutex::new(Config::load("config.toml", &mut files)));
        info!("Loaded config: {:?}", cfg.lock().unwrap());
        Self {
            arguments: args,
            exit_flag: AtomicBool::new(false),
            started_at: Instant::now(),
            config: cfg.clone(),
            files: Arc::new(Mutex::new(files)),
            vars: VarRegistry::new(),
            commands: CommandRegistry::default(),
        }
    }

    pub fn is_exit(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}
