extern crate self as rg_common;

pub use app::App;
pub use arguments::Arguments;
pub use commands::CommandRegistry;
pub use config::save_config;
pub use files::{FileError, Files, SeekAndRead, SeekAndWrite};
pub use loader::{Loader, LoaderError, load_bytes, load_deserializable};
pub use plugin::Plugin;
pub use vars::wrap_var_bag;
pub use vars::{
    FromStrMutator, FromValue, VarBag, VarRegistry, VarRegistryError, Variable, VariableError,
};

mod app;
mod arguments;
mod asset;
mod cmd_parser;
pub mod commands;
mod config;
mod files;
mod loader;
mod plugin;
pub mod ui;
mod v_from;
mod v_from_str;
mod v_from_value;
mod vars;
