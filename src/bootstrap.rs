use std::sync::Arc;

use poise::serenity_prelude as serenity;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};

use crate::commands;
use crate::config::{self, Config};
use crate::discord::InMemoryCooldownStore;
use crate::error::{Error, StartupError};
use crate::event_dispatch;
use crate::ports::CooldownStore;
use crate::state::{AppState, build_services};
use crate::telemetry;

pub async fn run() -> Result<(), Error> {
    let token = std::env::var("DISCORD_TOKEN").map_err(|_| StartupError::MissingToken)?;

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_VOICE_STATES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework: poise::Framework<AppState, Error> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![commands::recarregar(), commands::call()],
            event_handler: |framework, event| {
                Box::pin(event_dispatch::handle_event(framework, event))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let config_path = config::path_from_env();
                let config = Arc::new(Config::load(&config_path).await?);
                info!(path = %config_path.display(), "loaded configuration");

                let cooldowns: Arc<dyn CooldownStore> = Arc::new(InMemoryCooldownStore::default());
                let services = build_services(ctx, config, cooldowns.clone()).await?;

                Ok(AppState {
                    voice: RwLock::new(Arc::new(services.voice)),
                    mirror: RwLock::new(Arc::new(services.mirror)),
                    config_path,
                    cooldowns,
                    config_lock: Mutex::new(()),
                })
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;

    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        telemetry::shutdown_signal().await;
        info!("shutdown signal received, closing gateway connection");
        shard_manager.shutdown_all().await;
    });

    if let Err(error) = client.start().await {
        error!(%error, "gateway connection failed");
        return Err(error.into());
    }

    info!("gateway connection closed");
    Ok(())
}
