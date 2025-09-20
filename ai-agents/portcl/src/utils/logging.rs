use tracing::{info, warn, error, debug};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};
use std::path::Path;

pub fn setup_logging(log_level: &str, log_file: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let level = match log_level {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    let subscriber = tracing_subscriber::registry()
        .with(fmt::layer().pretty().with_target(false))
        .with(tracing_subscriber::filter::LevelFilter::from_level(level));

    // Add file logging if specified
    if let Some(log_path) = log_file {
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        let file_layer = fmt::layer()
            .json()
            .with_writer(file)
            .with_ansi(false);

        subscriber.with(file_layer).init();
    } else {
        subscriber.init();
    }

    info!("Logging initialized with level: {}", log_level);
    Ok(())
}