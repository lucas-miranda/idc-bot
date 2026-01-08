use thiserror::Error;
use poise::serenity_prelude::{self as serenity};

#[derive(Error, Debug)]
pub enum MessageError {
    #[error("role with id `{0}` was not found")]
    RoleNotFound(serenity::RoleId),
}
