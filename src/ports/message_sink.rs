use poise::serenity_prelude as serenity;

use crate::Error;

/// Result of trying to edit a mirrored message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditOutcome {
    Edited,
    NotFound,
}

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

    fn fetch_message(
        &self,
        channel_id: serenity::ChannelId,
        message_id: serenity::MessageId,
    ) -> impl Future<Output = Result<Option<serenity::Message>, Error>> + Send;

    /// The newest message id in a channel.
    fn latest_message_id(
        &self,
        channel_id: serenity::ChannelId,
    ) -> impl Future<Output = Result<Option<serenity::MessageId>, Error>> + Send;

    fn send(
        &self,
        channel_id: serenity::ChannelId,
        content: &str,
        embeds: &[serenity::Embed],
    ) -> impl Future<Output = Result<serenity::MessageId, Error>> + Send;

    fn edit_message(
        &self,
        channel_id: serenity::ChannelId,
        message_id: serenity::MessageId,
        content: &str,
        embeds: &[serenity::Embed],
    ) -> impl Future<Output = Result<EditOutcome, Error>> + Send;

    fn delete_message(
        &self,
        channel_id: serenity::ChannelId,
        message_id: serenity::MessageId,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}
