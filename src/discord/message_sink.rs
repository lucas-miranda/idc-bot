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

    async fn send(
        &self,
        channel_id: serenity::ChannelId,
        content: &str,
        embeds: &[serenity::Embed],
    ) -> Result<(), Error> {
        let embeds: Vec<serenity::CreateEmbed> = embeds.iter().cloned().map(Into::into).collect();

        channel_id
            .send_message(
                &self.ctx.http,
                serenity::CreateMessage::new()
                    .content(content)
                    .embeds(embeds),
            )
            .await?;

        Ok(())
    }
}
