use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

pub mod actions;
pub mod checks;
pub mod commands;
pub mod configuration;
pub mod embeds;
pub mod event_handlers;
pub mod models;
mod server;
pub mod stream;

use models::LunaError;

#[tokio::main]
async fn main() -> Result<(), LunaError> {
    use tokio::signal::unix as signal;
    init_tracing();

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

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .pretty()
        .init();
}
