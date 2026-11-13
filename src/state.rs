use std::{path::PathBuf, sync::Arc};

use poise::serenity_prelude as serenity;
use tokio::sync::RwLock;
use tracing::info;

use crate::app::VoiceService;
use crate::config::Config;
use crate::discord::{DiscordNotifier, DiscordVoiceChannels};
use crate::error::Error;
use crate::ports::CooldownStore;

/// Composition root shared with the poise framework.
pub struct AppState {
    pub voice: RwLock<Arc<VoiceService<DiscordVoiceChannels, DiscordNotifier>>>,
    pub config_path: PathBuf,
    pub cooldowns: Arc<dyn CooldownStore>,
}

impl AppState {
    /// Re-read the TOML file, rebuild the service and swap it in atomically.
    ///
    /// If anything fails, the previous service keeps running.
    pub async fn reload(&self, ctx: &serenity::Context) -> Result<(), Error> {
        let config = Arc::new(Config::load(&self.config_path).await?);
        let voice = build_voice_service(ctx, config, self.cooldowns.clone()).await?;

        *self.voice.write().await = Arc::new(voice);

        info!(path = %self.config_path.display(), "configuration reloaded");
        Ok(())
    }
}

/// Build the voice service from a validated configuration.
pub async fn build_voice_service(
    ctx: &serenity::Context,
    config: Arc<Config>,
    cooldowns: Arc<dyn CooldownStore>,
) -> Result<VoiceService<DiscordVoiceChannels, DiscordNotifier>, Error> {
    let channels = DiscordVoiceChannels::new(ctx.clone());
    let notifier = DiscordNotifier::new(ctx.clone(), &config, cooldowns).await?;

    Ok(VoiceService::new(config, channels, notifier))
}
