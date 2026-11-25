use poise::serenity_prelude as serenity;

use crate::Error;

/// Discord access needed by message mirroring.
pub trait MessageSink: Send + Sync {
    fn fetch_channel(
        &self,
        guild_id: serenity::GuildId,
        channel_id: serenity::ChannelId,
    ) -> impl Future<Output = Option<serenity::GuildChannel>> + Send;

    /// The voice channel a user is currently connected to in a guild.
    fn voice_channel_of(
        &self,
        guild_id: serenity::GuildId,
        user_id: serenity::UserId,
    ) -> Option<serenity::ChannelId>;

    fn send(
        &self,
        channel_id: serenity::ChannelId,
        content: &str,
        embeds: &[serenity::Embed],
    ) -> impl Future<Output = Result<(), Error>> + Send;
}
