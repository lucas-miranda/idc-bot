use std::{collections::HashMap, hash::Hash};
use poise::serenity_prelude::{self as serenity};
use crate::voice::{BroadcasterCreationError, SendingMessageError};

pub struct Broadcaster<T: Eq + Hash + Clone> {
    pub social_role: serenity::Role,
    pub emojis: HashMap<serenity::EmojiId, serenity::Emoji>,
    target_channel: serenity::GuildChannel,
    msgs: HashMap<T, String>,
}

impl<T: Eq + Hash + Clone> Broadcaster<T> {
    pub async fn new(
        ctx: &serenity::Context,
        target_channel_id: serenity::ChannelId,
        guild: serenity::PartialGuild,
        social_role_id: serenity::RoleId,
    ) -> Result<Broadcaster<T>, BroadcasterCreationError> {
        let target_channel = match ctx.http.get_channel(target_channel_id).await? {
            serenity::Channel::Guild(ch) => ch,
            _ => return Err(BroadcasterCreationError::MissingTextChannel(target_channel_id)),
        };

        let social_role = match guild.roles.get(&social_role_id) {
            Some(r) => r.clone(),
            None => return Err(BroadcasterCreationError::RoleNotFound(social_role_id))
        };

        let emojis = HashMap::from_iter(
            guild.emojis(&ctx.http)
                 .await?
                 .into_iter()
                 .map(|e| (e.id, e))
        );

        Ok(Broadcaster {
            target_channel,
            social_role,
            msgs: Default::default(),
            emojis,
        })
    }

    pub fn register_msg<F: FnOnce(&mut Broadcaster<T>) -> String>(mut self, label: T, func: F) -> Self {
        let msg = func(&mut self);
        self.msgs.insert(label, msg);
        self
    }

    pub async fn send_msg(&self, ctx: &serenity::Context, label: &T) -> Result<serenity::Message, SendingMessageError<T>> {
        match self.msgs.get(label) {
            Some(msg) => {
                let builder = serenity::CreateMessage::new().content(msg);
                self.target_channel.send_message(&ctx.http, builder)
                    .await
                    .map_err(SendingMessageError::from)
            },
            None => Err(SendingMessageError::LabelNotFound(label.clone())),
        }
    }
}
