use std::collections::vec_deque::Iter;
use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, SyncSender};

use log::{LevelFilter, Record};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::append::Append;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::{Config, Handle};
use rg_common::Arguments;

use crate::error::AppError;

const CONSOLE_PATTERN: &str = "{d(%H:%M:%S%.3f)} {h({l})} [{T}] {M} - {m}{n}";
const PATTERN: &str = "{d(%Y-%m-%d %H:%M:%S%.3f)} {l} [{T}] {M} - {m}{n}";

#[derive(Debug)]
pub(crate) struct AppLogger {
    tx: SyncSender<String>,
}

pub(crate) struct AppLoggerBuffer {
    rx: Receiver<String>,
    max_size: usize,
    buffer: VecDeque<String>,
}

impl AppLoggerBuffer {}

fn create_app_logger(max_size: usize) -> (AppLogger, AppLoggerBuffer) {
    let (tx, rx): (SyncSender<String>, Receiver<String>) = mpsc::sync_channel(max_size);
    let buf = AppLoggerBuffer {
        rx,
        max_size,
        buffer: VecDeque::new(),
    };
    let logger = AppLogger { tx };
    (logger, buf)
}

pub(crate) fn init(args: &Arguments) -> Result<(Handle, AppLoggerBuffer), AppError> {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(CONSOLE_PATTERN)))
        .build();
    let file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(PATTERN)))
        .build("app.log")?;
    let (logger, buf) = create_app_logger(400);
    let level = args.log_level();
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .appender(Appender::builder().build("app", Box::new(logger)))
        .logger(Logger::builder().build("app", level))
        .build(
            Root::builder()
                .appender("stdout")
                .appender("app")
                .appender("file")
                .build(level),
        ).map_err(|e| AppError::IllegalState { message: e.to_string() })?;

    let handle = log4rs::init_config(config)?;
    Ok((handle, buf))
}

pub(crate) fn build_dedicated_config() -> Result<Config, AppError> {
    let stdout = ConsoleAppender::builder().build();
    let file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(PATTERN)))
        .build("app.log")?;
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .build(
            Root::builder()
                .appender("stdout")
                .appender("file")
                .build(LevelFilter::Info),
        )
        .map_err(|e| AppError::IllegalState { message: e.to_string() })?;

    Ok(config)
}

impl Append for AppLogger {
    fn append(&self, record: &Record) -> anyhow::Result<()> {
        let msg = format!("{} - {}", record.level(), record.args());
        match self.tx.try_send(msg) {
            Ok(_) => Ok(()),
            Err(e) => {
                match e {
                    mpsc::TrySendError::Full(_) => Ok(()), // drop message
                    mpsc::TrySendError::Disconnected(_) => Err(anyhow::Error::from(e)),
                }
            }
        }
    }

    fn flush(&self) {}
}

impl AppLoggerBuffer {
    pub fn update(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            if self.buffer.len() == self.max_size {
                self.buffer.pop_front();
            }
            self.buffer.push_back(msg);
        }
    }

    pub(crate) fn iter(&self) -> Iter<String> {
        self.buffer.iter()
    }
}

#[cfg(test)]
mod test {

    use log::Record;
    use log4rs::append::Append;

    use crate::app_logger::create_app_logger;

    #[test]
    fn buffer_overflow() {
        let (logger, mut buf) = create_app_logger(3);
        assert_eq!(0, buf.buffer.len());
        logger
            .append(&Record::builder().level(log::Level::Info).build())
            .unwrap();
        buf.update();
        assert_eq!(1, buf.buffer.len());
        logger
            .append(&Record::builder().level(log::Level::Info).build())
            .unwrap();
        logger
            .append(&Record::builder().level(log::Level::Warn).build())
            .unwrap();
        buf.update();
        assert_eq!(3, buf.buffer.len());
        logger
            .append(&Record::builder().level(log::Level::Error).build())
            .unwrap();
        logger
            .append(&Record::builder().level(log::Level::Info).build())
            .unwrap();
        buf.update();
        assert_eq!(3, buf.buffer.len());
        buf.iter().rev().for_each(|v| println!("Got line: {v}"));
    }

    #[test]
    fn channel_overflow() {
        let (logger, mut buf) = create_app_logger(5);
        assert_eq!(0, buf.buffer.len());
        for _ in 0..100 {
            logger
                .append(&Record::builder().level(log::Level::Info).build())
                .unwrap();
        }
        buf.update();
        assert_eq!(5, buf.buffer.len());
    }
}
