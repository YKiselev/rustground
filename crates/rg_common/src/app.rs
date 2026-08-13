use std::borrow::Borrow;
use std::sync::Arc;
use std::time::{Duration, Instant};

use toml::Table;
use tracing::{info, warn};

use rg_common::arguments::Arguments;
use rg_common::{CommandRegistry, Files, VarRegistry};

use crate::asset::{AssetError, Assets};
use crate::commands::{CmdError, CommandBuilder};
use crate::config::read_config;
use crate::{Loader, LoaderError, save_config};

pub struct App {
    pub name: String,
    pub arguments: Arguments,
    pub started_at: Instant,
    pub files: Files,
    pub vars: VarRegistry,
    pub commands: CommandRegistry,
    pub assets: Assets,
}

impl App {
    pub fn new(args: Arguments) -> Self {
        let files = Files::new(&args);
        let vars = VarRegistry::new(None);
        let commands = CommandRegistry::default();
        Self {
            name: "Rust Ground".to_string(),
            arguments: args,
            started_at: Instant::now(),
            files: files,
            vars,
            commands,
            assets: Assets::new(),
        }
    }

    pub fn command_builder<'a>(&'a self) -> CommandBuilder<'a> {
        CommandBuilder::new(&self.commands)
    }

    pub fn execute<S>(&self, command: S) -> Result<(), CmdError>
    where
        S: AsRef<str>,
    {
        let falback = |args: &[&str]| {
            match args.len() {
                1 => {
                    if let Some(value) = self.vars.try_get_value(args[0]) {
                        info!("{} = {}", args[0], value);
                        return Ok(());
                    }
                }
                2 => {
                    if let Ok(()) = self.vars.try_set_value(args[0], args[1]) {
                        info!("{} = {}", args[0], args[1]);
                        return Ok(());
                    }
                }
                _ => {}
            }
            Err(CmdError::NotFound)
        };
        self.commands.execute(command, Some(&falback))
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn load_config<S>(&self, names: &[S])
    where
        S: AsRef<str>,
    {
        let cfg = names
            .iter()
            .map(
                |name| match self.load_resource(name.as_ref(), &read_config, ()) {
                    Ok(cfg) => Some(cfg),
                    Err(e) => {
                        warn!("Failed to load {}: {:?}", name.as_ref(), e);
                        None
                    }
                },
            )
            .into_iter()
            .fold(Table::default(), |mut a, b| {
                if b.is_some() {
                    deep_merge(&mut a, b.unwrap());
                }
                a
            });

        let _ = self
            .vars
            .set_table(cfg)
            .inspect_err(|e| warn!("Unable to set config table: {:?}", e));
    }

    pub fn save_config(&self, name: &str, value: String) {
        save_config(name, &self.files, value);
    }

    pub fn load_asset<S, L, A, Ctx>(
        &self,
        name: S,
        loader: &L,
        ctx: Ctx,
    ) -> Result<Arc<A>, AssetError>
    where
        S: Into<Box<str>> + Borrow<str>,
        L: Loader<A, Ctx> + 'static,
        A: Send + Sync + 'static,
    {
        self.assets
            .load(name, |n| self.files.buf_read(n).ok(), loader, ctx)
    }

    pub fn load_resource<S, L, A, Ctx>(
        &self,
        name: S,
        loader: &L,
        ctx: Ctx,
    ) -> Result<A, LoaderError>
    where
        S: AsRef<str>,
        L: Loader<A, Ctx> + 'static,
        A: Send + Sync + 'static,
    {
        self.files
            .buf_read(name.as_ref())
            .map_err(|_| LoaderError::NotFound(String::from(name.as_ref())))
            .and_then(|mut r| loader.load(&mut r, ctx))
    }
}

fn deep_merge(a: &mut Table, b: Table) {
    for (key, value) in b {
        match value {
            // If value is Table, and a have this key, merge recursively
            toml::Value::Table(b_inner_table) => {
                if let Some(toml::Value::Table(a_inner_table)) = a.get_mut(&key) {
                    deep_merge(a_inner_table, b_inner_table);
                } else {
                    a.insert(key, toml::Value::Table(b_inner_table));
                }
            }
            other_value => {
                a.insert(key, other_value);
            }
        }
    }
}
