use std::sync::{Arc, mpsc};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use log::warn;
use mio::{Events, Poll};
use mio::event::Event;

use crate::net::Endpoint;

pub(crate) struct ToSend {}

pub(crate) struct ServerNetwork {
    rx: Receiver<Event>,
    handle: JoinHandle<()>,
}

struct Worker {
    tx: Sender<Event>,
    poll: Poll,
    events: Events,
}

impl Worker {
    fn new(tx: Sender<Event>) -> anyhow::Result<Self> {
        let mut poll: Poll = Poll::new()?;
        let mut events = Events::with_capacity(1024);
        Ok(
            Worker {
                tx: tx.clone(),
                poll,
                events,
            }
        )
    }

    fn handle_event(&mut self, event: &Event) {
        let token = event.token();
    }

    fn run(&mut self) {
        loop {
            match self.poll.poll(&mut self.events, Some(Duration::from_millis(10))) {
                Ok(()) => {
                    for event in &self.events {
                        self.handle_event(event);
                    }
                }
                Err(e) => {
                    warn!("Polling failed: {e:?}");
                }
            }
        }
    }
}

impl ServerNetwork {
    pub(crate) fn new(endpoint: Endpoint) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            Worker::new(in_tx).run();
        });
        Ok(
            ServerNetwork {
                rx: out_rx,
                handle,
            }
        )
    }
}