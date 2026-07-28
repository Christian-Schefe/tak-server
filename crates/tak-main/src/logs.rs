use std::env;
use tracing_appender::non_blocking::{NonBlockingBuilder, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::Layer;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logger() -> WorkerGuard {
    let file_path = env::var("LOG_FILE_DIRECTORY").expect("LOG_FILE_DIRECTORY must be set");
    let file_name = env::var("LOG_FILE_NAME").expect("LOG_FILE_NAME must be set");

    let stderr_filter = EnvFilter::new("info,sqlx=warn");

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_writer(std::io::stderr)
        .with_filter(stderr_filter);

    std::fs::create_dir_all(&file_path).expect("Failed to create log directory");

    let file_appender = RollingFileAppender::new(Rotation::DAILY, file_path, file_name);

    let (file_writer, guard) = NonBlockingBuilder::default().finish(file_appender);

    let file_filter = EnvFilter::new("debug");

    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_writer(file_writer)
        .with_filter(file_filter);

    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .init();

    guard
}
