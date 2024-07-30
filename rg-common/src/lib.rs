pub mod arguments;
pub mod files;
mod vars;

pub use files::AppFiles;
pub use vars::VarBag;
pub use vars::VarInfo;
pub use vars::NoSuchVariableError;

extern crate self as rg_common;
