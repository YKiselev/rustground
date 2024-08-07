extern crate self as rg_common;

pub use files::AppFiles;
pub use vars::VarBag;
pub use vars::VariableError;
pub use vars::VarInfo;
pub use vars::Variable;

pub mod arguments;
pub mod files;
mod vars;
pub mod config;
mod v_from;

