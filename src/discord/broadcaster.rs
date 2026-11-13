use std::{collections::HashMap, sync::Arc, time::Instant};

use poise::serenity_prelude as serenity;

use crate::config::Config;
use crate::domain::{CooldownPolicy, CooldownStatus};
use crate::error::{Error, NotifierError, StartupError};
use crate::ports::{CooldownStore, Notifier};

/// Serenity-backed [`Notifier`].
pub struct DiscordNotifier {
    ctx: serenity::Context,
    target_channel: serenity::GuildChannel,
    messages: HashMap<serenity::ChannelId, String>,
    cooldowns: Arc<dyn CooldownStore>,
    policy: CooldownPolicy,
}

impl DiscordNotifier {
    pub async fn new(
        ctx: serenity::Context,
        config: &Config,
        cooldowns: Arc<dyn CooldownStore>,
    ) -> Result<Self, StartupError> {
        let target_channel = match ctx.http.get_channel(config.broadcast_channel_id).await? {
            serenity::Channel::Guild(channel) => channel,
            _ => {
                return Err(StartupError::MissingTextChannel(
                    config.broadcast_channel_id,
                ));
            }
        };

        let guild = ctx.http.get_guild(config.guild_id).await?;

        let emojis: HashMap<_, _> = guild
            .emojis(&ctx.http)
            .await?
            .into_iter()
            .map(|emoji| (emoji.id, emoji))
            .collect();

        let pride_heart_emoji_id = serenity::EmojiId::new(1073325566196990142);

        let mut messages = HashMap::new();
        for call in &config.call_kinds {
            let content = match guild.roles.get(&call.role_id) {
                Some(role) => {
                    Self::build_message(role, emojis.get(&pride_heart_emoji_id), &call.message)
                }
                None => {
                    tracing::warn!(
                        role_id = %call.role_id,
                        channel_id = %call.voice_channel,
                        "role not found; announcement will be empty"
                    );
                    String::new()
                }
            };

            messages.insert(call.voice_channel, content);
        }

        Ok(Self {
            ctx,
            target_channel,
            messages,
            cooldowns,
            policy: CooldownPolicy::new(config.cooldown),
        })
    }

    fn build_message(
        role: &serenity::Role,
        emoji: Option<&serenity::Emoji>,
        suffix: &str,
    ) -> String {
        let mut content = serenity::MessageBuilder::new();
        content.mention(role).push(suffix);

        if let Some(emoji) = emoji {
            content.emoji(emoji);
        }

        content.build()
    }
}

impl Notifier for DiscordNotifier {
    async fn notify(&self, channel_id: serenity::ChannelId) -> Result<(), Error> {
        let content = self
            .messages
            .get(&channel_id)
            .ok_or(NotifierError::UnknownChannel(channel_id))?;

        let key = channel_id.to_string();
        let now = Instant::now();

        match self.policy.check(now, self.cooldowns.last_sent(&key)) {
            CooldownStatus::TooSoon { elapsed } => {
                return Err(NotifierError::TooSoon {
                    elapsed,
                    cooldown: self.policy.duration(),
                }
                .into());
            }
            CooldownStatus::Ready => {}
        }

        self.cooldowns.set_last_sent(&key, now);

        self.target_channel
            .send_message(
                &self.ctx.http,
                serenity::CreateMessage::new().content(content.as_str()),
            )
            .await?;

        Ok(())
    }
}
