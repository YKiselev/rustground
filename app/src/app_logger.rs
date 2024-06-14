use std::rc::Rc;
use log::{Record, Metadata, Log, LevelFilter};

pub(crate) struct AppLogger {
    delegate: Box<dyn Log>,
}

static mut APP_LOGGER: Option<AppLogger> = None;

pub(crate) fn init() -> &'static AppLogger {
    assert!(unsafe { APP_LOGGER.is_none() }, "App logger is already set!");
    unsafe {
        APP_LOGGER = Some(AppLogger {
            delegate: Box::new(env_logger::builder().build())
        })
    };
    let logger = unsafe { APP_LOGGER.as_ref().unwrap() };
    log::set_logger(logger).expect("Unable to install logger!");
    log::set_max_level(LevelFilter::Info);
    logger
}

impl Log for AppLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.delegate.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        self.delegate.log(record);
    }

    fn flush(&self) {
        self.delegate.flush();
    }
}