extern crate self as rg_common;

pub use arguments::Arguments;
pub use commands::CommandRegistry;
pub use files::AppFiles;
use log::debug;
use log::error;
use log::info;
use log::warn;
pub use vars::FromStrMutator;
pub use vars::VarBag;
pub use vars::VarRegistry;
pub use vars::Variable;
pub use vars::VariableError;

pub mod arguments;
pub mod cmd_parser;
pub mod commands;
pub mod config;
pub mod files;
pub mod pool;
mod v_from;
mod v_from_str;
mod vars;
mod test;
