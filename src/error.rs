use std::time::Duration;

use poise::serenity_prelude as serenity;
use thiserror::Error;

/// Crate-wide error type.
///
/// Each layer contributes one variant; `?` conversions keep call sites clean.
/// Serenity's error is boxed because it is much larger than the other variants.
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Discord(Box<serenity::Error>),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Notifier(#[from] NotifierError),
    #[error(transparent)]
    Startup(#[from] StartupError),
}

impl From<serenity::Error> for Error {
    fn from(error: serenity::Error) -> Self {
        Self::Discord(Box::new(error))
    }
}

/// Failures while loading, validating or editing the configuration file.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config file `{path}`: {source}")]
    Read {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write config file `{path}`: {source}")]
    Write {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to edit config file: {0}")]
    Edit(#[from] toml_edit::TomlError),
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

/// Failures while preparing or sending an announcement.
#[derive(Error, Debug)]
pub enum NotifierError {
    #[error("no call is registered for channel `{0}`")]
    UnknownChannel(serenity::ChannelId),
    #[error(
        "cooldown ({}h{}m{}s) isn't completed yet, time elapsed: {}h{}m{}s",
        .cooldown.as_secs() / 3600,
        (.cooldown.as_secs() % 3600) / 60,
        .cooldown.as_secs() % 60,
        .elapsed.as_secs() / 3600,
        (.elapsed.as_secs() % 3600) / 60,
        .elapsed.as_secs() % 60
    )]
    TooSoon {
        elapsed: Duration,
        cooldown: Duration,
    },
}

/// Failures while building the bot's services at startup.
#[derive(Error, Debug)]
pub enum StartupError {
    #[error("environment variable `DISCORD_TOKEN` is missing")]
    MissingToken,
    #[error(transparent)]
    Serenity(Box<serenity::Error>),
    #[error("guild with id `{0}` was not found")]
    GuildNotFound(serenity::GuildId),
    #[error("role with id `{0}` was not found")]
    RoleNotFound(serenity::RoleId),
    #[error("provided channel with id `{0}` isn't a text channel")]
    MissingTextChannel(serenity::ChannelId),
}

impl From<serenity::Error> for StartupError {
    fn from(error: serenity::Error) -> Self {
        Self::Serenity(Box::new(error))
    }
}
