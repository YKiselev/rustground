use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use log::{error, info};

use log4rs::Handle;
use rg_common::arguments::Arguments;
use rg_common::{AppFiles, VarRegistry};

use crate::app_logger::AppLoggerBuffer;
use crate::error::AppError;
use crate::state::{AppState, InitialState};
use rg_common::config::Config;

pub(crate) struct App {
    arguments: Arguments,
    log_handle: Handle,
    log_buffer: Option<AppLoggerBuffer>,
    exit_flag: AtomicBool,
    started_at: Instant,
    config: Arc<Mutex<Config>>,
    files: Arc<Mutex<AppFiles>>,
    vars: VarRegistry<Config>,
}

impl App {
    pub(crate) fn new(
        args: Arguments,
        log_handle: Handle,
        log_buffer: Option<AppLoggerBuffer>,
    ) -> Self {
        let mut files = AppFiles::new(&args);
        let cfg = Arc::new(Mutex::new(Config::load("config.toml", &mut files)));
        info!("Loaded config: {:?}", cfg.lock().unwrap());
        App {
            arguments: args,
            log_handle,
            log_buffer,
            exit_flag: AtomicBool::new(false),
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

    pub(crate) fn exit_flag(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    // pub(crate) fn run(&mut self) -> Result<(), AppError> {
    //     let mut state: Box<dyn AppState> = Box::new(InitialState::default());
    //     info!("Entering main loop...");
    //     loop {
    //         match state.try_advance(self) {
    //             Ok(Some(s)) => {
    //                 state = s;
    //             }
    //             Ok(None) => {
    //                 info!("No state to transition to, exiting...");
    //                 break;
    //             }
    //             Err(e) => {
    //                 error!("Got error: {}", e);
    //                 break;
    //             }
    //         }
    //     }
    //     info!("Leaving main loop.");
    //     Ok(())
    // }
}
