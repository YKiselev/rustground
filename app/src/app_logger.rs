use std::collections::VecDeque;
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
    max_size: usize,
    buffer: Arc<RwLock<VecDeque<String>>>,
}

pub(crate) struct AppLoggerBuffer {
    buffer: Arc<RwLock<VecDeque<String>>>,
}

pub(crate) fn init() -> Result<AppLoggerBuffer, Error> {
    let stdout = ConsoleAppender::builder().build();
    let file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build("app.log")?;
    let buf = Arc::new(RwLock::new(VecDeque::new()));
    let logger = AppLogger {
        max_size: 100,
        buffer: buf.clone(),
    };
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .appender(Appender::builder().build("app", Box::new(logger)))
        //.logger(Logger::builder().build("app", LevelFilter::Info))
        .build(Root::builder()
            .appender("stdout")
            .appender("app")
            .appender("file")
            .build(LevelFilter::Debug)
        )?;

    let handle = log4rs::init_config(config)?;
    Ok(AppLoggerBuffer {
        buffer: buf
    })
}

impl Append for AppLogger {
    fn append(&self, record: &Record) -> anyhow::Result<()> {
        let mut guard = self.buffer.write().unwrap();
        let line = format!("{} - {}", record.level(), record.args());
        while guard.len() >= self.max_size {
            guard.pop_front();
        }
        guard.push_back(line);
        Ok(())
    }

    fn flush(&self) {}
}

impl AppLoggerBuffer {
    pub(crate) fn iterate<F>(&self, handler: F)
        where F: FnMut(&String) -> ()
    {
        let mut guard = self.buffer.write().unwrap();
        guard.iter().for_each(handler);
    }
}