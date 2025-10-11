// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use miaoyu_desktop_lib::DynLoggingLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    dotenvy::dotenv().ok();

    let (layer, handle) = tracing_subscriber::reload::Layer::new(None::<DynLoggingLayer>);

    let logs_dir = {
        #[cfg(target_os = "macos")]
        let path = dirs::home_dir()
            .unwrap()
            .join("Library/Logs")
            .join("so.miaoyu.desktop");

        #[cfg(not(target_os = "macos"))]
        let path = dirs::data_local_dir()
            .unwrap()
            .join("so.miaoyu.desktop")
            .join("logs");
        path
    };

    // Ensure logs directory exists
    std::fs::create_dir_all(&logs_dir).expect("Failed to create logs directory");

    let file_appender = tracing_appender::rolling::daily(&logs_dir, "miaoyu-desktop.log");
    let (non_blocking, _logger_guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("trace"));

    let registry = tracing_subscriber::registry().with(env_filter).with(
        tracing_subscriber::filter::filter_fn(
            (|v| v.target().starts_with("miaoyu_")) as fn(&tracing::Metadata) -> bool,
        ),
    );

    registry
        .with(layer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_target(true),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_writer(non_blocking),
        )
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build multi threaded tokio runtime")
        .block_on(miaoyu_desktop_lib::run(handle));
}
