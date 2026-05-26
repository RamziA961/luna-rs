use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

pub mod actions;
pub mod checks;
pub mod commands;
pub mod configuration;
pub mod embeds;
pub mod event_handlers;
pub mod metrics;
pub mod models;
mod server;
pub mod stream;

use models::LunaError;

const METRICS_ADDR: &str = "0.0.0.0:9000";

#[tokio::main]
async fn main() -> Result<(), LunaError> {
    use tokio::signal::unix as signal;
    init_tracing();

    use metrics_exporter_prometheus::PrometheusBuilder;
    use std::net::SocketAddr;

    info!("Starting metrics server");
    PrometheusBuilder::new()
        .with_http_listener(METRICS_ADDR.parse::<SocketAddr>().unwrap())
        .install()
        .expect("Failed to install prometheus recorder");
    info!("Metrics server started at {METRICS_ADDR}");

    let vars = configuration::ConfigurationVariables::new();

    info!("Starting luna-rs v{}", env!("CARGO_PKG_VERSION"));
    let mut server = server::Server::new(vars).await;

    let mut sigterm = signal::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = signal::signal(tokio::signal::unix::SignalKind::interrupt())?;

    tokio::select! {
        res = server.start() => {
            if let Err(e) = res {
                error!(error = %e, "Fatal runtime error.");
                return Err(LunaError::Runtime(Box::new(e)));
            }
        }
        _ = sigint.recv() => info!("SIGINT received. Shutting down..."),
        _ = sigterm.recv() => info!("SIGTERM received. Shutting down..."),
    };

    server.stop().await;
    info!("Graceful shutdown complete.");
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        .add_directive(
            format!(
                "luna_rs={}",
                if cfg!(debug_assertions) {
                    "trace"
                } else {
                    "warn"
                }
            )
            .parse()
            .expect("Invalid log directive"),
        );

    let subscriber = tracing_subscriber::fmt().with_env_filter(filter);

    #[cfg(debug_assertions)]
    subscriber.pretty().init();

    #[cfg(not(debug_assertions))]
    subscriber.json().init();
}
