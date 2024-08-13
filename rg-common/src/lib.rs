extern crate self as rg_common;

pub use files::AppFiles;
pub use vars::VarBag;
pub use vars::VariableError;
pub use vars::Variable;
pub use vars::VarRegistry;
pub use vars::FromStrMutator;

pub mod arguments;
pub mod files;
mod vars;
pub mod config;
mod v_from;
mod v_from_str;

