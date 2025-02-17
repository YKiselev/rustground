use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use log::info;

use rg_common::arguments::Arguments;
use rg_common::{AppFiles, VarRegistry};

use rg_common::config::Config;

pub(crate) struct ExitFlag(Arc<AtomicBool>);

impl ExitFlag {
    pub fn load(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

pub(crate) struct App {
    arguments: Arguments,
    exit_flag: Arc<AtomicBool>,
    started_at: Instant,
    config: Arc<Mutex<Config>>,
    files: Arc<Mutex<AppFiles>>,
    vars: VarRegistry<Config>,
}

impl App {
    pub(crate) fn new(args: Arguments) -> Self {
        let mut files = AppFiles::new(&args);
        let cfg = Arc::new(Mutex::new(Config::load("config.toml", &mut files)));
        info!("Loaded config: {:?}", cfg.lock().unwrap());
        App {
            arguments: args,
            exit_flag: Arc::new(AtomicBool::new(false)),
            started_at: Instant::now(),
            config: cfg.clone(),
            files: Arc::new(Mutex::new(files)),
            vars: VarRegistry::new(cfg),
        }
    }

    pub(crate) fn args(&self) -> &Arguments {
        &self.arguments
    }

    pub(crate) fn config(&self) -> &Arc<Mutex<Config>> {
        &self.config
    }

    pub(crate) fn is_exit(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    pub(crate) fn exit_flag(&self) -> ExitFlag {
        ExitFlag(Arc::clone(&self.exit_flag))
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}
