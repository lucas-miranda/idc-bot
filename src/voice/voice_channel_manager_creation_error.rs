use thiserror::Error;
use poise::serenity_prelude::{self as serenity};

use crate::voice::BroadcasterCreationError;

#[derive(Error, Debug)]
pub enum VoiceChannelManagerCreationError {
    #[error("guild with id `{0}` was not found")]
    GuildNotFound(serenity::GuildId),
    #[error("broadcaster creation failed")]
    Broadcaster(#[from] BroadcasterCreationError),
    #[error("internal error")]
    Serenity(#[from] serenity::Error),
}
