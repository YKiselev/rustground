use std::any::Any;
use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Error};
use log::{error, info};

use rg_common::AppFiles;
use rg_common::arguments::Arguments;
use rg_macros::VarBag;

use crate::config::Config;
use crate::state::{AppState, InitialState};

#[derive(Copy, Clone)]
enum Value {
    SocketAddr(SocketAddr)
}

enum PlayerMode {
    SinglePlayer,
    MultiPlayer,
}

impl TryFrom<Value> for SocketAddr {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::SocketAddr(addr) = value {
            return Ok(addr);
        }
        Err(Error::msg("Wrong type!"))
    }
}

impl From<SocketAddr> for Value {
    fn from(value: SocketAddr) -> Self {
        Value::SocketAddr(value)
    }
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

    pub(crate) fn config(&self) -> &Config {
        &self.config
    }

    pub(crate) fn exit_flag(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub(crate) fn run(&mut self) -> anyhow::Result<()> {
        let mut state: Box<dyn AppState> = Box::new(InitialState::default());
        info!("Entering main loop...");
        loop {
            match state.try_advance(self) {
                Ok(Some(s)) => {
                    state = s;
                }
                Ok(None) => {
                    break;
                }
                Err(e) => {
                    error!("Got error: {}", e);
                    break;
                }
            }
        }
        info!("Leaving main loop.");
        Ok(())
    }

    pub(crate) fn set_var<S, V>(&mut self, key: S, value: V)
        where S: Into<String>,
              V: Into<Value>
    {
        let mut lock = self.vars.write().expect("Lock poisoned!");
        lock.insert(key.into(), value.into());
    }

    pub(crate) fn get_var<S, V>(&self, key: S) -> anyhow::Result<V>
        where S: Into<String>,
              V: TryFrom<Value, Error=Error>
    {
        self.vars.read().unwrap().get(&key.into())
            .and_then(|v| V::try_from(*v).ok())
            .ok_or_else(|| Error::msg("Not found!"))
    }
}