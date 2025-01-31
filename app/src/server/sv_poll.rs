use std::{
    net::SocketAddr,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, scope, JoinHandle},
    time::Duration,
};

use mio::{Events, Poll};

use crate::net::NetEndpoint;

pub(crate) struct ServerPollThread {
    endpoint: NetEndpoint,
    tx: Sender<u32>,
    rx: Receiver<u32>,
    handle: JoinHandle<()>
}

impl ServerPollThread {
    fn new(addr: SocketAddr) -> Self {
        //let addr: SocketAddr = config.address.parse().expect("Invalid address!");
        let endpoint = NetEndpoint::with_address(addr).expect("Unable to create server endpoint!");
        let (in_tx, in_rx) = mpsc::channel();
        let (out_tx, out_rx) = mpsc::channel();
        let mut poll = Poll::new().expect("Unable to create poll object!");
        let timeout = Some(Duration::from_millis(200));
        let handle = thread::spawn(move || {
            let tx = out_tx;
            let rx = in_rx;
            let mut events = Events::with_capacity(256);
            loop {
                match poll.poll(&mut events, timeout) {
                    Ok(_) => {
                        for e in events.iter() {
                            // todo
                        }
                    }
                    Err(_) => {
                        // todo
                    }
                }
            }
        });
        Self {
            endpoint,
            tx: in_tx,
            rx: out_rx,
            handle
        }
    }
}
