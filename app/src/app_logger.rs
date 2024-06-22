use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

use anyhow::Error;
use log::{LevelFilter, Record};
use log4rs::append::Append;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::Config;
use log4rs::config::{Appender, Root};
use log4rs::encode::{Encode, Write};
use log4rs::encode::pattern::PatternEncoder;

#[derive(Debug)]
pub(crate) struct AppLogger {
    tx: Sender<String>,
}

pub(crate) struct AppLoggerBuffer {
    rx: Receiver<String>,
}

pub(crate) fn init() -> Result<AppLoggerBuffer, Error> {
    let stdout = ConsoleAppender::builder().build();
    let file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build("app.log")?;
    let (tx, rx) = channel();
    let logger = AppLogger {
        tx
    };
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .appender(Appender::builder().build("app", Box::new(logger)))
        //.logger(Logger::builder().build("app", LevelFilter::Info))
        .build(Root::builder().appender("stdout").appender("app").appender("file").build(LevelFilter::Info))?;

    let handle = log4rs::init_config(config)?;
    Ok(AppLoggerBuffer { rx })
}

impl Append for AppLogger {
    fn append(&self, record: &Record) -> anyhow::Result<()> {
        self.tx.send(format!("{} - {}", record.level(), record.args())).map_err(anyhow::Error::from)
    }

    fn flush(&self) {}
}

impl AppLoggerBuffer {
    pub(crate) fn update(&self) {
        loop {
            match self.rx.try_recv() {
                Ok(record) => println!("Got record: {:?}", record),
                Err(e) => {
                    match e {
                        TryRecvError::Empty => {
                            break;
                        }
                        TryRecvError::Disconnected => {
                            println!("Error: {:?}", e);
                            break;

                        }
                    }
                }
            }
        }
    }
}