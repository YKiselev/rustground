use std::fmt::Write;
use std::sync::{Arc, RwLock};

use anyhow::Error;
use log::{LevelFilter, Record};
use log4rs::append::Append;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::Config;
use log4rs::config::{Appender, Root};
use log4rs::encode::Encode;
use log4rs::encode::pattern::PatternEncoder;

#[derive(Debug)]
pub(crate) struct AppLogger {
    buffer: Arc<RwLock<String>>,
}

pub(crate) struct AppLoggerBuffer {
    buffer: Arc<RwLock<String>>,
}

pub(crate) fn init() -> Result<AppLoggerBuffer, Error> {
    let stdout = ConsoleAppender::builder().build();
    let file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build("app.log")?;
    let buf = Arc::new(RwLock::new(String::new()));
    let logger = AppLogger {
        buffer: buf.clone()
    };
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .appender(Appender::builder().build("app", Box::new(logger)))
        //.logger(Logger::builder().build("app", LevelFilter::Info))
        .build(Root::builder().appender("stdout").appender("app").appender("file").build(LevelFilter::Info))?;

    let handle = log4rs::init_config(config)?;
    Ok(AppLoggerBuffer {
        buffer: buf
    })
}

impl Append for AppLogger {
    fn append(&self, record: &Record) -> anyhow::Result<()> {
        let mut guard = self.buffer.write().unwrap();
        write!(guard, "{} - {}", record.level(), record.args()).map_err(anyhow::Error::from)
    }

    fn flush(&self) {}
}

impl AppLoggerBuffer {
    pub(crate) fn update(&self) {
        let mut guard = self.buffer.write().unwrap();
        if !guard.is_empty() {
            println!("Got record: {}", guard.as_str());
            guard.clear();
        }
    }
}