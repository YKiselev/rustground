extern crate core;

use std::ops::DerefMut;
use std::process::exit;
use std::result;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use log::{error, info, LevelFilter, warn};
use rsa::signature::digest::Digest;

use rg_common::arguments::Arguments;
use rg_common::{files, VarBag};

use crate::bit_code_test::test_bitcode;
use rg_common::config::{Config, ServerConfig};
use crate::net::Message;

mod client;
mod server;
mod app_logger;
mod net;
mod bit_code_test;
mod app;
mod state;

fn main() -> anyhow::Result<()> {
    let logger_buf = app_logger::init().expect("Unable to init app logger!");
    info!("Begin initialization...");

    let args = Arguments::parse();
    let mut app = app::App::new(&args);
    app.run()
}
