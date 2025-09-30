use thiserror::Error;
use poise::serenity_prelude::{self as serenity};

#[derive(Error, Debug)]
pub enum BroadcasterCreationError {
    #[error("role with id `{0}` was not found")]
    RoleNotFound(serenity::RoleId),
    #[error("internal error")]
    Serenity(#[from] serenity::Error),
    #[error("provided channel with id `{0}` isn't a text channel")]
    MissingTextChannel(serenity::ChannelId),
}
