use std::{path::PathBuf, sync::Arc};

use poise::serenity_prelude as serenity;
use tokio::sync::{Mutex, RwLock};
use toml_edit::DocumentMut;
use tracing::info;

use crate::app::{MessageMirror, MirrorTracker, VoiceService};
use crate::config::{self, Config};
use crate::discord::{DiscordMessageSink, DiscordNotifier, DiscordVoiceChannels};
use crate::error::{ConfigError, Error};
use crate::ports::CooldownStore;

/// Composition root shared with the poise framework.
pub struct AppState {
    pub voice: RwLock<Arc<VoiceService<DiscordVoiceChannels, DiscordNotifier>>>,
    pub mirror: RwLock<Arc<MessageMirror<DiscordMessageSink>>>,
    pub mirror_tracker: Arc<MirrorTracker>,
    pub config_path: PathBuf,
    pub cooldowns: Arc<dyn CooldownStore>,
    pub config_lock: Mutex<()>,
}

impl AppState {
    /// Re-read the TOML file, rebuild the services and swap them in atomically.
    ///
    /// If anything fails, the previous services keep running.
    pub async fn reload(&self, ctx: &serenity::Context) -> Result<(), Error> {
        let _guard = self.config_lock.lock().await;

        let config = Arc::new(Config::load(&self.config_path).await?);
        let services = build_services(
            ctx,
            config,
            self.cooldowns.clone(),
            self.mirror_tracker.clone(),
        )
        .await?;

        *self.voice.write().await = Arc::new(services.voice);
        *self.mirror.write().await = Arc::new(services.mirror);

        info!(path = %self.config_path.display(), "configuration reloaded");
        Ok(())
    }

    /// Add or update the call for a voice channel in the TOML file.
    pub async fn upsert_call(
        &self,
        ctx: &serenity::Context,
        channel_id: serenity::ChannelId,
        role_id: serenity::RoleId,
    ) -> Result<config::edit::UpsertOutcome, Error> {
        let _guard = self.config_lock.lock().await;

        let text = read_config_text(&self.config_path).await?;
        let mut document: DocumentMut = text.parse().map_err(ConfigError::from)?;

        let outcome = config::edit::upsert(&mut document, channel_id, role_id)?;
        if outcome == config::edit::UpsertOutcome::Unchanged {
            return Ok(outcome);
        }

        self.apply_config_text(ctx, document.to_string()).await?;
        Ok(outcome)
    }

    /// Remove the call for a voice channel. Returns whether it existed.
    pub async fn remove_call(
        &self,
        ctx: &serenity::Context,
        channel_id: serenity::ChannelId,
    ) -> Result<bool, Error> {
        let _guard = self.config_lock.lock().await;

        let text = read_config_text(&self.config_path).await?;
        let mut document: DocumentMut = text.parse().map_err(ConfigError::from)?;

        if !config::edit::remove(&mut document, channel_id)? {
            return Ok(false);
        }

        self.apply_config_text(ctx, document.to_string()).await?;
        Ok(true)
    }

    /// Validate, rebuild and persist a new config text, then swap it in.
    async fn apply_config_text(&self, ctx: &serenity::Context, text: String) -> Result<(), Error> {
        let config = Arc::new(Config::from_toml_str(&text)?);
        let services = build_services(
            ctx,
            config,
            self.cooldowns.clone(),
            self.mirror_tracker.clone(),
        )
        .await?;

        write_config_text(&self.config_path, &text).await?;

        *self.voice.write().await = Arc::new(services.voice);
        *self.mirror.write().await = Arc::new(services.mirror);

        info!(path = %self.config_path.display(), "configuration updated");
        Ok(())
    }
}

/// Services built from a single configuration snapshot.
pub struct Services {
    pub voice: VoiceService<DiscordVoiceChannels, DiscordNotifier>,
    pub mirror: MessageMirror<DiscordMessageSink>,
}

/// Build both services from a validated configuration.
pub async fn build_services(
    ctx: &serenity::Context,
    config: Arc<Config>,
    cooldowns: Arc<dyn CooldownStore>,
    tracker: Arc<MirrorTracker>,
) -> Result<Services, Error> {
    let channels = DiscordVoiceChannels::new(ctx.clone());
    let notifier = DiscordNotifier::new(ctx.clone(), &config, cooldowns).await?;
    let sink = DiscordMessageSink::new(ctx.clone());
    let mirror = MessageMirror::new(config.clone(), sink, tracker);

    Ok(Services {
        voice: VoiceService::new(config, channels, notifier),
        mirror,
    })
}

async fn read_config_text(path: &std::path::Path) -> Result<String, ConfigError> {
    tokio::fs::read_to_string(path)
        .await
        .map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })
}

async fn write_config_text(path: &std::path::Path, text: &str) -> Result<(), ConfigError> {
    let temporary = path.with_extension("toml.tmp");

    tokio::fs::write(&temporary, text)
        .await
        .map_err(|source| ConfigError::Write {
            path: temporary.clone(),
            source,
        })?;

    tokio::fs::rename(&temporary, path)
        .await
        .map_err(|source| ConfigError::Write {
            path: path.to_path_buf(),
            source,
        })?;

    Ok(())
}
