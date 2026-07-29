use std::collections::VecDeque;
use std::collections::vec_deque::Iter;
use std::fs::File;
use std::iter::Rev;
use std::sync::mpsc::{self, Receiver, SyncSender};

use crate::error::AppError;
use chrono::Local;
use flume::Sender;
use rg_common::Arguments;
use tracing::{Event, Subscriber};
use tracing::{Level, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Layer;
use tracing_subscriber::fmt::time::{ChronoLocal, LocalTime};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::layer::{Context, Filter};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

pub struct LogRecord {
    pub level: Level,
    pub message: String,
    pub time: chrono::DateTime<Local>,
}

///
/// App log layer
///
pub struct AppLogLayer {
    tx: flume::Sender<LogRecord>,
    rx: flume::Receiver<LogRecord>,
}

impl AppLogLayer {
    fn new(tx: flume::Sender<LogRecord>, rx: flume::Receiver<LogRecord>) -> Self {
        Self { tx, rx }
    }
}
impl<S: Subscriber> Layer<S> for AppLogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut log_record = if let Ok(lr) = self.rx.try_recv() {
            lr
        } else {
            LogRecord {
                level: Level::INFO,
                message: String::with_capacity(200),
                time: Local::now(),
            }
        };

        let metadata = event.metadata();

        log_record.message.clear();
        log_record.level = *metadata.level();
        log_record.time = Local::now();

        let mut visitor = EventVisitor {
            message: &mut log_record.message,
        };
        event.record(&mut visitor);

        let _ = self.tx.try_send(log_record);
    }
}

struct EventVisitor<'a> {
    message: &'a mut String,
}

impl<'a> tracing::field::Visit for EventVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message.push_str(&format!("{:?}", value));
        }
    }
}

//
// App logger buffer
//

pub(crate) struct AppLoggerBuffer {
    rx: flume::Receiver<LogRecord>,
    tx: flume::Sender<LogRecord>,
    max_size: usize,
    buffer: VecDeque<LogRecord>,
}

impl AppLoggerBuffer {
    fn new(max_size: usize, tx: flume::Sender<LogRecord>, rx: flume::Receiver<LogRecord>) -> Self {
        Self {
            rx,
            tx,
            max_size,
            buffer: VecDeque::with_capacity(max_size),
        }
    }

    pub fn update(&mut self) {
        while let Ok(record) = self.rx.try_recv() {
            if self.buffer.len() == self.max_size {
                if let Some(record) = self.buffer.pop_front() {
                    let _ = self.tx.try_send(record);
                }
            }
            self.buffer.push_back(record);
        }
    }

    pub(crate) fn iter(&self) -> Rev<Iter<'_, LogRecord>> {
        self.buffer.iter().rev()
    }
}

//
// Functions
//

pub(crate) fn init(args: &Arguments) -> Result<(AppLoggerBuffer, Vec<WorkerGuard>), AppError> {
    let env_filter = EnvFilter::from_default_env();

    let (non_blocking_stdout, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());
    let time_format = ChronoLocal::new("%H:%M:%S%.3f".to_string());
    let mut stdout_format_layer = fmt::layer()
        .with_timer(time_format)
        .with_ansi(true)
        .with_level(true)
        .with_target(true)
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

    let (to_buf_tx, from_layer_rx) = flume::bounded(100);
    let (to_layer_tx, from_buf_rx) = flume::bounded(100);
    let app_layer = AppLogLayer::new(to_buf_tx, from_buf_rx);
    let app_log_buffer = AppLoggerBuffer::new(500, to_layer_tx, from_layer_rx);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_format_layer)
        .with(file_format_layer)
        .with(app_layer)
        .init();

    Ok((app_log_buffer, vec![stdout_guard, file_guard]))
}
