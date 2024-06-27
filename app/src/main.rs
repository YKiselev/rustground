use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use log::{info, LevelFilter};
use serde::{Deserialize, Serialize};

use core::arguments::Arguments;
use core::services::Services;

use crate::client::Client;
use crate::net::{ConnectData, Message};
use crate::server::Server;

mod client;
mod server;
mod app_logger;
mod net;

fn test_serde() {
    let mut buf: Vec<u8> = Vec::new();
    let mut ser1 = rmp_serde::Serializer::new(&mut buf);
    Message::Connect(ConnectData { name: "Alice".to_string(), password: "12345".to_string() }).serialize(&mut ser1).expect("aaa!!!");
    Message::Accepted.serialize(&mut ser1).expect("bbb!!!");
    Message::Connect(ConnectData { name: "Bob".to_string(), password: "12345".to_string() }).serialize(&mut ser1).expect("ccc!!!");
    info!("Buf size: {}", buf.len());
//let r = rmp_serde::from_slice(&[1,1,1,1,1,1]).expect("aaaaaaaaaaa");
    let des = rmp_serde::Deserializer::new(buf.as_slice());
    let mut des = rmp_serde::Deserializer::from_read_ref(buf.as_slice());
    let m1 = Message::deserialize(&mut des).expect("a!");
    info!("m1={m1:?}");
    let m2 = Message::deserialize(&mut des).expect("b!");
    info!("m2={m2:?}");
    let m3 = Message::deserialize(&mut des).expect("c!");
    info!("m3={m3:?}");
}

fn main() -> anyhow::Result<()> {
    let logger_buf = app_logger::init().unwrap();
    info!("Begin initialization...");

    let args = Arguments::parse();

    // init services
    let services = Services::new(&args);

    // server
    let mut server = Server::new(&args);

    // debug
    let server_addr = server.local_address().expect("Unable to get server address");

    // client
    let mut client = if !args.dedicated() {
        Some(Client::new(&args, server_addr))
    } else {
        None
    };

    let handle = thread::spawn(|| ());
    // serde test
    //test_serde();
    //exit(0);

    // main loop
    info!("Entering main loop...");
    let exit_flag = AtomicBool::new(false);
    let mut i = 0;
    while !exit_flag.load(Ordering::Acquire) {
        server.update()?;
        if let Some(ref mut client) = client.as_mut() {
            client.update();
        }
        std::thread::sleep(Duration::from_millis(10));
        //logger_buf.update();
        i += 1;
        if i > 100 {
            exit_flag.store(true, Ordering::Release);
        }
    }
    Ok(())
}
