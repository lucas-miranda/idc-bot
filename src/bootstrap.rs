use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::app::VoiceService;
use crate::config::Config;
use crate::discord::{DiscordNotifier, DiscordVoiceChannels, InMemoryCooldownStore};
use crate::error::Error;
use crate::event_dispatch;
use crate::ports::CooldownStore;

/// Composition root: the concrete services the event handler receives.
pub struct AppState {
    pub voice: VoiceService<DiscordVoiceChannels, DiscordNotifier>,
}

pub async fn run() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_VOICE_STATES;

    let framework: poise::Framework<AppState, Error> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_dispatch::handle_event(ctx, event, framework, data))
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
        .await
        .expect("Error creating client");

    if let Err(e) = client.start().await {
        println!("Client error: {e:?}");
    }
}
