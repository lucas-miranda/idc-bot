pub mod app;
pub mod bootstrap;
pub mod config;
pub mod discord;
pub mod domain;
pub mod error;
pub mod event_dispatch;
pub mod ports;

pub use error::Error;

#[tokio::main]
async fn main() {
    bootstrap::run().await;
}
