use std::time::Duration;

use thiserror::Error;
use poise::serenity_prelude::{self as serenity};

#[derive(Error, Debug)]
pub enum SendingMessageError<T: Clone> {
    #[error("message label was not found")]
    LabelNotFound(T),
    #[error("database is not accessible")]
    FailedToAccessDatabase,
    #[error(
        "cooldown ({}h{}m{}s) isn't completed yet, time elapsed: {}h{}m{}s",
        cooldown / 3600,
        cooldown / 60,
        cooldown % 60,
        elapsed.as_secs() / 3600,
        elapsed.as_secs() / 60,
        elapsed.as_secs() % 60
    )]
    TooSoon { elapsed: Duration, cooldown: u64 },
    #[error("internal error")]
    NotFound(#[from] serenity::Error),
}
