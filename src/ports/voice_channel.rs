use poise::serenity_prelude as serenity;

use crate::Error;

/// Read/write access to the Discord state around voice channels.
///
/// The application layer only uses this contract; [`crate::discord`] provides
/// the serenity-backed implementation.
pub trait VoiceChannelPort: Send + Sync {
    fn fetch_channel(
        &self,
        channel_id: serenity::ChannelId,
    ) -> impl Future<Output = Option<serenity::GuildChannel>> + Send;

    /// Whether `member` is a non-bot staff member for this channel.
    fn is_staff(&self, channel: &serenity::GuildChannel, member: &serenity::Member) -> bool;

    /// Make the channel visible. Returns `true` when an edit was performed.
    fn reveal(
        &self,
        channel: &mut serenity::GuildChannel,
    ) -> impl Future<Output = Result<bool, Error>> + Send;

    fn conceal(
        &self,
        channel: &mut serenity::GuildChannel,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    fn connected_staff(&self, channel: &serenity::GuildChannel) -> Option<serenity::Member>;

    fn connected_members(
        &self,
        channel: &serenity::GuildChannel,
    ) -> Result<Vec<serenity::Member>, Error>;

    fn disconnect(
        &self,
        member: &serenity::Member,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}
