use std::{collections::hash_map::Entry, time::Instant};
use poise::serenity_prelude::{self as serenity};
use crate::voice::{Broadcaster, BroadcasterDatabaseEntry, MessageLabel, SendingMessageError};

pub struct PreparedMessage {
    pub content: String,
}

impl PreparedMessage {
    pub fn new<C: Into<String>>(content: C) -> Self {
        Self {
            content: content.into(),
        }
    }

    pub async fn send<T: MessageLabel>(
        &self,
        ctx: &serenity::Context,
        entry: BroadcasterDatabaseEntry<'_>,
        broadcaster: &Broadcaster<T>,
    ) -> Result<serenity::Message, SendingMessageError<T>> {
        // check cooldown
        let now = Instant::now();
        const COOLDOWN: u64 = 60 * 60;

        match &entry {
            Entry::Occupied(occupied) => {
                let elapsed = now.duration_since(*occupied.get());

                if elapsed.as_secs() < COOLDOWN {
                    return Err(SendingMessageError::TooSoon { elapsed, cooldown: COOLDOWN });
                }
            },
            Entry::Vacant(_vacant) => (),
        }

        // update last time used
        entry.insert_entry(now);

        // send
        broadcaster.target_channel
            .send_message(
                &ctx.http,
                serenity::CreateMessage::new()
                    .content(&self.content),
            )
            .await
            .map_err(SendingMessageError::from)
    }
}
