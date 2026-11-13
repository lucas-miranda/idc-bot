pub mod app;
pub mod bootstrap;
pub mod commands;
pub mod config;
pub mod discord;
pub mod domain;
pub mod error;
pub mod event_dispatch;
pub mod ports;
pub mod state;
pub mod telemetry;

pub use error::Error;

#[tokio::main]
async fn main() {
    telemetry::init_tracing();

    if let Err(error) = bootstrap::run().await {
        tracing::error!(%error, "bot stopped with an error");
        std::process::exit(1);
    }
}
