use rg_macros::VarBag;
use serde::{Deserialize, Serialize};

#[derive(Default, VarBag, Serialize, Deserialize)]
pub(crate) struct Config {
    pub windowed: bool,
    pub width: usize,
    pub height: usize,
}
