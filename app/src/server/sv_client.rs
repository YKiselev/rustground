use std::io;
use std::time::Instant;

use log::{error, info, warn};

use crate::error::AppError;

use super::server::ClientId;

#[derive(Debug)]
pub struct Client {
    id: ClientId,
    name: String,
    last_seen: Instant
}

impl Client {
    pub fn new(id: ClientId, name: &str) -> Self {
        Client {
            id,
            name: name.to_string(),
            last_seen: Instant::now()
        }
    }

    pub(crate) fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    // pub(crate) fn send(&mut self, msg: &Message) -> io::Result<usize> {
    //     self.endpoint.send(msg)
    // }

    // fn clear_buffers(&mut self) {
    //     self.endpoint.clear_buffers();
    // }

    // pub(crate) fn flush(&mut self) -> io::Result<usize> {
    //     self.endpoint.flush()
    // }

    // pub(crate) fn process_message(&mut self, msg: &Message) -> Result<(), AppError> {
    //     self.touch();
    //     info!("Got from connected client: {msg:?}");
    //     match msg {
    //         // Message::Ack(_) => {}
    //         // Message::Connect(_) => {}
    //         // Message::Accepted => {}
    //         // Message::Hello => {}
    //         Pong { time } => {
    //             info!(
    //                 "Ping to client is {:.6} sec.",
    //                 Instant::now().elapsed().as_secs_f64() - time
    //             );
    //         }
    //         Ping { time } => {
    //             self.endpoint.send(&Pong { time: *time })?;
    //         }
    //         m => {
    //             warn!("Ignoring unsupported message: {m:?}");
    //         }
    //     }
    //     Ok(())
    // }

    pub(crate) fn update(&mut self, buf: &mut Vec<u8>) -> Result<(), AppError> {
        // self.clear_buffers();
        // loop {
        //     match self.endpoint.receive_data(buf.as_mut()) {
        //         Ok(Some(mut data)) => {
        //             self.last_seen = Instant::now();
        //             while let Some(ref m) = data.read() {
        //                 self.process_message(m)?;
        //             }
        //         }

        //         Ok(None) => {
        //             break;
        //         }
        //         Err(e) => {
        //             error!("Failed to receive from client: {e:?}");
        //             break;
        //         }
        //     }
        // }
        Ok(())
    }
}
