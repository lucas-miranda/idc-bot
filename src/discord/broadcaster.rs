use std::{collections::HashMap, sync::Arc, time::Instant};

use poise::serenity_prelude as serenity;

use crate::config::Config;
use crate::domain::{CallKind, CooldownPolicy, CooldownStatus};
use crate::error::{Error, NotifierError, StartupError};
use crate::ports::{CooldownStore, Notifier};

/// Serenity-backed [`Notifier`].
pub struct DiscordNotifier {
    ctx: serenity::Context,
    target_channel: serenity::GuildChannel,
    social_role: serenity::Role,
    messages: HashMap<CallKind, String>,
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

        let social_config = config
            .call_kinds
            .iter()
            .find(|call_kind| call_kind.kind == CallKind::Social)
            .ok_or(StartupError::MissingCallKind(CallKind::Social))?;

        let social_role = guild
            .roles
            .get(&social_config.role_id)
            .cloned()
            .ok_or(StartupError::RoleNotFound(social_config.role_id))?;

        let emojis: HashMap<_, _> = guild
            .emojis(&ctx.http)
            .await?
            .into_iter()
            .map(|emoji| (emoji.id, emoji))
            .collect();

        let pride_heart_emoji_id = serenity::EmojiId::new(1073325566196990142);

        let mut messages = HashMap::new();
        for call_kind in &config.call_kinds {
            let content = match guild.roles.get(&call_kind.role_id) {
                Some(role) => {
                    Self::build_message(role, emojis.get(&pride_heart_emoji_id), call_kind.message)
                }
                None => {
                    tracing::warn!(
                        role_id = %call_kind.role_id,
                        call_kind = %call_kind.kind,
                        "role not found; announcement will be empty"
                    );
                    String::new()
                }
            };

            messages.insert(call_kind.kind.clone(), content);
        }

        Ok(Self {
            ctx,
            target_channel,
            social_role,
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
    fn social_role_name(&self) -> &str {
        &self.social_role.name
    }

    async fn notify(&self, kind: &CallKind) -> Result<(), Error> {
        let content = self
            .messages
            .get(kind)
            .ok_or_else(|| NotifierError::UnknownCallKind(kind.clone()))?;

        let key = kind.to_string();
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
