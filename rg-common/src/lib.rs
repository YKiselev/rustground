extern crate self as rg_common;

pub use commands::CommandRegistry;
pub use files::AppFiles;
pub use vars::FromStrMutator;
pub use vars::VarBag;
pub use vars::VarRegistry;
pub use vars::Variable;
pub use vars::VariableError;

pub mod arguments;
mod commands;
pub mod config;
pub mod files;
mod v_from;
mod v_from_str;
mod vars;
