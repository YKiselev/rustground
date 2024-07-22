use std::any::Any;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use log::info;

use common::AppFiles;
use common::arguments::Arguments;

use crate::as_init::InitialState;
use crate::config::Config;

enum Value {
    SocketAddr(SocketAddr)
}

pub(crate) struct App {
    arguments: Arguments,
    exit_flag: AtomicBool,
    started_at: Instant,
    config: Config,
    files: Arc<RwLock<AppFiles>>,
    vars: Arc<RwLock<HashMap<String, Value>>>,
}

impl App {
    pub(crate) fn new(args: &Arguments) -> Self {
        let mut files = AppFiles::new(&args);
        let cfg = Config::load("config.toml", &mut files);
        info!("Loaded config: {cfg:?}");
        App {
            arguments: *args,
            exit_flag: AtomicBool::new(false),
            started_at: Instant::now(),
            config: cfg,
            files: Arc::new(RwLock::new(files)),
            vars: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub(crate) fn args(&self) -> &Arguments {
        &self.arguments
    }

    pub(crate) fn run(&mut self) -> anyhow::Result<()> {
        let mut state: Box<dyn crate::app_state::AppState> = Box::new(InitialState::default());
        info!("Entering main loop...");
        while !self.exit_flag.load(Ordering::Relaxed) {
            match state.try_advance(self) {
                Ok(Some(s)) => {
                    println!("State transition");
                    state = s;
                }
                Ok(None) => {
                    // not ready yet
                }
                Err(e) => {
                    println!("Got error: {}", e);
                    break;
                }
            }
            thread::sleep(Duration::from_millis(5));
            // debug
            if self.started_at.elapsed() > Duration::from_secs(7) {
                self.exit_flag.store(true, Ordering::Release);
            }
        }
        info!("Leaving main loop.");
        Ok(())
    }

    pub(crate) fn set_var<S: Into<String>, V: Into<Value>>(&mut self, key: S, value: V) {
        self.vars.write().unwrap().insert(key.into(), value.into());
    }

    pub(crate) fn get_var<S: Into<String>, V: From<Value>>(&self, key: S) -> Option<&V> {
        None//self.vars.read().unwrap().get(&key.into()).map(|v| v.into())
    }
}