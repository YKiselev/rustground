extern crate core;

use std::collections::HashSet;
use std::io::Read;
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

use common::arguments::Arguments;
use common::{files, VarBag};
use crate::app::TestVars;

use crate::bit_code_test::test_bitcode;
use crate::client::Client;
use crate::config::{Config, ServerConfig};
use crate::net::Message;
use crate::server::{Server, server_init};

mod client;
mod server;
mod app_logger;
mod net;
mod config;
mod bit_code_test;
mod app;
mod state;

fn main() -> anyhow::Result<()> {
    let logger_buf = app_logger::init().unwrap();
    info!("Begin initialization...");

    let v = TestVars::default();
    let mut names = HashSet::new();
    let f : Option<Box<dyn VarBag>> = None;
    v.get_names(&mut names);
    info!("Structure fields are: {:?}", names);

    let args = Arguments::parse();
    let mut app = app::App::new(&args);
    //app.run()
    Ok(())
}
