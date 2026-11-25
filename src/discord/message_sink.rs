use poise::serenity_prelude as serenity;

use crate::Error;
use crate::ports::MessageSink;

/// Serenity-backed [`MessageSink`].
pub struct DiscordMessageSink {
    ctx: serenity::Context,
}

impl DiscordMessageSink {
    pub fn new(ctx: serenity::Context) -> Self {
        Self { ctx }
    }
}

fn is_not_found(error: &serenity::Error) -> bool {
    matches!(
        error,
        serenity::Error::Http(http)
            if http.status_code().map(|status| status.as_u16()) == Some(404)
    )
}

impl MessageSink for DiscordMessageSink {
    async fn fetch_channel(
        &self,
        guild_id: serenity::GuildId,
        channel_id: serenity::ChannelId,
    ) -> Option<serenity::GuildChannel> {
        if let Some(guild) = self.ctx.cache.guild(guild_id)
            && let Some(channel) = guild.channels.get(&channel_id)
        {
            return Some(channel.clone());
        }

        match channel_id.to_channel(&self.ctx).await {
            Ok(serenity::Channel::Guild(channel)) => Some(channel),
            Ok(_) => None,
            Err(e) => {
                tracing::warn!(%e, channel_id = %channel_id, "failed to fetch channel for mirroring");
                None
            }
        }
    }

    fn voice_channel_of(
        &self,
        guild_id: serenity::GuildId,
        user_id: serenity::UserId,
    ) -> Option<serenity::ChannelId> {
        let guild = self.ctx.cache.guild(guild_id)?;

        guild
            .voice_states
            .get(&user_id)
            .and_then(|state| state.channel_id)
    }

    async fn fetch_message(
        &self,
        channel_id: serenity::ChannelId,
        message_id: serenity::MessageId,
    ) -> Result<Option<serenity::Message>, Error> {
        match channel_id.message(&self.ctx, message_id).await {
            Ok(message) => Ok(Some(message)),
            Err(error) if is_not_found(&error) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    async fn send(
        &self,
        channel_id: serenity::ChannelId,
        content: &str,
        embeds: &[serenity::Embed],
    ) -> Result<serenity::MessageId, Error> {
        let embeds: Vec<serenity::CreateEmbed> = embeds.iter().cloned().map(Into::into).collect();

        let message = channel_id
            .send_message(
                &self.ctx.http,
                serenity::CreateMessage::new()
                    .content(content)
                    .embeds(embeds),
            )
            .await?;

        Ok(message.id)
    }

    async fn delete_message(
        &self,
        channel_id: serenity::ChannelId,
        message_id: serenity::MessageId,
    ) -> Result<(), Error> {
        match channel_id.delete_message(&self.ctx.http, message_id).await {
            Ok(()) => Ok(()),
            Err(error) if is_not_found(&error) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}
