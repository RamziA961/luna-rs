use server::ServerError;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

pub mod actions;
pub mod checks;
pub mod commands;
pub mod configuration;
pub mod models;
mod server;

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                //.with_default_directive(LevelFilter::TRACE.into())
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()
                .unwrap()
                .add_directive(
                    format!(
                        "luna_rs_2={}",
                        if cfg!(debug_assertions) {
                            "trace"
                        } else {
                            "warn"
                        }
                    )
                    .parse()
                    .unwrap(),
                ),
        )
        .pretty()
        .init();

    let vars = configuration::ConfigurationVariables::new();

    info!("Starting server.");
    server::Server::new(vars).await.start().await?;

    Ok(())
}
