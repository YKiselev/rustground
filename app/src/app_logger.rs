use std::collections::VecDeque;
use std::collections::vec_deque::Iter;
use std::fs::File;
use std::sync::mpsc::{self, Receiver, SyncSender};

use crate::error::AppError;
use rg_common::Arguments;
use tracing::{Event, Subscriber};
use tracing::{Level, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Layer;
use tracing_subscriber::fmt::time::ChronoLocal;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::layer::{Context, Filter};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

///
/// App log layer
///
#[derive(Default)]
pub struct AppLogLayer;

impl<S: Subscriber> Layer<S> for AppLogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = metadata.level();
        let target = metadata.target();

        let mut fields = String::new();
        let mut visitor = EventVisitor {
            output: &mut fields,
        };
        event.record(&mut visitor);

        // todo - do something with it
        // println!(
        //     "Got: {}, Таргет: {}, Данные: {}",
        //     level, target, fields
        // );
    }
}

/// Вспомогательная структура (Visitor) для чтения полей из события `tracing`
struct EventVisitor<'a> {
    output: &'a mut String,
}

impl<'a> tracing::field::Visit for EventVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.output.push_str(&format!("{:?}", value));
        } else {
            self.output
                .push_str(&format!(" {}={:?}", field.name(), value));
        }
    }
}

//
// App logger buffer
//

pub(crate) struct AppLoggerBuffer {
    rx: Receiver<String>,
    max_size: usize,
    buffer: VecDeque<String>,
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

    pub(crate) fn iter(&self) -> Iter<'_, String> {
        self.buffer.iter()
    }
}

//
// Functions
//

// fn create_app_logger(max_size: usize) -> (AppLogger, AppLoggerBuffer) {
//     let (tx, rx): (SyncSender<String>, Receiver<String>) = mpsc::sync_channel(max_size);
//     let buf = AppLoggerBuffer {
//         rx,
//         max_size,
//         buffer: VecDeque::new(),
//     };
//     let logger = AppLogger { tx };
//     (logger, buf)
// }

pub(crate) fn init(args: &Arguments) -> Result<Vec<WorkerGuard>, AppError> {
    // let stdout = ConsoleAppender::builder()
    //     .encoder(Box::new(PatternEncoder::new(CONSOLE_PATTERN)))
    //     .build();
    // let file = FileAppender::builder()
    //     .encoder(Box::new(PatternEncoder::new(PATTERN)))
    //     .build("app.log")?;
    //let (_logger, buf) = create_app_logger(400);
    // let level = args.log_level.unwrap_or(LevelFilter::Info);
    // let config = Config::builder()
    //     .appender(Appender::builder().build("stdout", Box::new(stdout)))
    //     //.appender(Appender::builder().build("file", Box::new(file)))
    //     //.appender(Appender::builder().build("app", Box::new(logger)))
    //     .logger(Logger::builder().build("app", level))
    //     .build(
    //         Root::builder()
    //             .appender("stdout")
    //             //.appender("app")
    //             //.appender("file")
    //             .build(level),
    //     )?;

    let env_filter = EnvFilter::from_default_env();

    let (non_blocking_stdout, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());
    let time_format = ChronoLocal::new("%H:%M:%S%.3f".to_string());
    let mut stdout_format_layer = fmt::layer()
        .with_timer(time_format)
        .with_ansi(true)
        .with_level(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_writer(non_blocking_stdout);

    if false {
        stdout_format_layer = stdout_format_layer
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false);
    }

    if let Err(e) = File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open("./logs/app.log")
    {
        warn!("Unable to clear log file: {:?}", e);
    }

    let file_appender = tracing_appender::rolling::never("./logs", "app.log");
    let (non_blocking_file, file_guard) = tracing_appender::non_blocking(file_appender);
    let time_format = ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string());
    let file_format_layer = fmt::layer()
        .with_timer(time_format)
        .with_ansi(false)
        .with_writer(non_blocking_file);

    let _app_layer = AppLogLayer::default();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_format_layer)
        .with(file_format_layer)
        .init();

    Ok(vec![stdout_guard, file_guard])
}

// pub(crate) fn build_dedicated_config() -> Result<Config, AppError> {
//     let stdout = ConsoleAppender::builder().build();
//     let file = FileAppender::builder()
//         .encoder(Box::new(PatternEncoder::new(PATTERN)))
//         .build("app.log")?;
//     let config = Config::builder()
//         .appender(Appender::builder().build("stdout", Box::new(stdout)))
//         .appender(Appender::builder().build("file", Box::new(file)))
//         .build(
//             Root::builder()
//                 .appender("stdout")
//                 .appender("file")
//                 .build(LevelFilter::Info),
//         )?;

//     Ok(config)
// }

#[cfg(test)]
mod tests {}
