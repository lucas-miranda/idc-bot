use thiserror::Error;
use poise::serenity_prelude::{self as serenity};

#[derive(Error, Debug)]
pub enum SendingMessageError<T: Clone> {
    #[error("message label was not found")]
    LabelNotFound(T),
    #[error("internal error")]
    NotFound(#[from] serenity::Error),
}
