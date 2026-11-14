use std::collections::HashSet;

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

    /// Whether `member` is a staff member: a non-bot owner, a member with
    /// `MANAGE_GUILD`, or a member holding one of `reload_roles`.
    fn is_staff(
        &self,
        member: &serenity::Member,
        owners: &HashSet<serenity::UserId>,
        reload_roles: &[serenity::RoleId],
    ) -> bool;

    /// Make the channel visible. Returns `true` when an edit was performed.
    fn reveal(
        &self,
        channel: &mut serenity::GuildChannel,
    ) -> impl Future<Output = Result<bool, Error>> + Send;

    fn conceal(
        &self,
        channel: &mut serenity::GuildChannel,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    fn connected_staff(
        &self,
        channel: &serenity::GuildChannel,
        owners: &HashSet<serenity::UserId>,
        reload_roles: &[serenity::RoleId],
        exclude: Option<serenity::UserId>,
    ) -> Option<serenity::Member>;

    fn connected_members(
        &self,
        channel: &serenity::GuildChannel,
    ) -> Result<Vec<serenity::Member>, Error>;

    fn disconnect(
        &self,
        member: &serenity::Member,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}
