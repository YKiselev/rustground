use std::borrow::Borrow;
use std::fs::File;
use std::io::{BufReader, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{info, warn};

use rg_common::arguments::Arguments;
use rg_common::{CommandRegistry, Files, VarRegistry};

use crate::asset::{AssetError, Assets, Loader};
use crate::config::load_config;
use crate::save_config;

pub struct App {
    pub arguments: Arguments,
    pub exit_flag: AtomicBool,
    pub started_at: Instant,
    pub files: Files,
    pub vars: VarRegistry,
    pub commands: CommandRegistry,
    pub assets: Assets,
}

impl App {
    pub fn new(args: Arguments) -> Self {
        let files = Files::new(&args);
        Self {
            arguments: args,
            exit_flag: AtomicBool::new(false),
            started_at: Instant::now(),
            files: files,
            vars: VarRegistry::new(None),
            commands: CommandRegistry::default(),
            assets: Assets::new(),
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

    pub fn load_asset<S, L, A, Rd>(&self, name: S, loader: &L) -> Result<Arc<A>, AssetError>
    where
        S: Into<Box<str>> + Borrow<str>,
        Rd: Read,
        L: Loader<A, BufReader<File>> + 'static,
        A: Send + Sync + 'static,
    {
        self.assets.load(
            name,
            |n| self.files.read(n).map(|f| BufReader::new(f)),
            loader,
        )
    }
}
