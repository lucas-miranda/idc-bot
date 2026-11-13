use std::sync::Arc;

use poise::serenity_prelude as serenity;
use tracing::{error, info};

use crate::app::VoiceService;
use crate::config::Config;
use crate::discord::{DiscordNotifier, DiscordVoiceChannels, InMemoryCooldownStore};
use crate::error::{Error, StartupError};
use crate::event_dispatch;
use crate::ports::CooldownStore;
use crate::telemetry;

/// Composition root: the concrete services the event handler receives.
pub struct AppState {
    pub voice: VoiceService<DiscordVoiceChannels, DiscordNotifier>,
}

pub async fn run() -> Result<(), Error> {
    let token = std::env::var("DISCORD_TOKEN").map_err(|_| StartupError::MissingToken)?;

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_VOICE_STATES;

    let framework: poise::Framework<AppState, Error> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            event_handler: |framework, event| {
                Box::pin(event_dispatch::handle_event(framework, event))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let config = Arc::new(Config::default());
                let channels = DiscordVoiceChannels::new(ctx.clone());
                let cooldowns: Arc<dyn CooldownStore> = Arc::new(InMemoryCooldownStore::default());
                let notifier = DiscordNotifier::new(ctx.clone(), &config, cooldowns).await?;

                Ok(AppState {
                    voice: VoiceService::new(config, channels, notifier),
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
