use std::{collections::{hash_map::Entry, HashMap}, fmt::{Debug, Display}, hash::Hash, time::Instant};
use poise::serenity_prelude::{self as serenity, prelude::TypeMapKey};
use crate::voice::{BroadcasterCreationError, PreparedMessage, SendingMessageError};

pub trait MessageLabel: Eq + Hash + Clone + Debug + Display {
}

pub struct Broadcaster<T: MessageLabel> {
    pub social_role: serenity::Role,
    pub emojis: HashMap<serenity::EmojiId, serenity::Emoji>,
    pub target_channel: serenity::GuildChannel,
    msgs: HashMap<T, PreparedMessage>,
}

impl<T: MessageLabel> Broadcaster<T> {
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
        let content = func(&mut self);
        self.msgs.insert(label, PreparedMessage::new(content));
        self
    }

    pub async fn send_msg(&self, ctx: &serenity::Context, label: &T) -> Result<serenity::Message, SendingMessageError<T>> {
        match self.msgs.get(label) {
            Some(msg) => {
                // get message entry from broadcaster database
                let mut data = ctx.data.write().await;
                let db = data.get_mut::<BroadcasterDatabase>()
                             .ok_or(SendingMessageError::FailedToAccessDatabase)?;
                let entry = db.entry(label.to_string());

                msg.send(ctx, entry, self).await
            },
            None => Err(SendingMessageError::LabelNotFound(label.clone())),
        }
    }
}

pub struct BroadcasterDatabase;

impl TypeMapKey for BroadcasterDatabase {
    type Value = HashMap<String, Instant>;
}

pub type BroadcasterDatabaseEntry<'a> = Entry<'a, String, Instant>;
