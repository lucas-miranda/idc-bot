use poise::serenity_prelude as serenity;

use crate::Error;
use crate::domain::overwrites;
use crate::ports::VoiceChannelPort;

/// Serenity-backed [`VoiceChannelPort`].
pub struct DiscordVoiceChannels {
    ctx: serenity::Context,
}

impl DiscordVoiceChannels {
    pub fn new(ctx: serenity::Context) -> Self {
        Self { ctx }
    }

    fn permissions_in(
        &self,
        channel: &serenity::GuildChannel,
        member: &serenity::Member,
    ) -> Option<serenity::Permissions> {
        if member.user.bot {
            return None;
        }

        let guild = member.guild_id.to_guild_cached(&self.ctx.cache)?;

        Some(guild.user_permissions_in(channel, member))
    }
}

impl VoiceChannelPort for DiscordVoiceChannels {
    async fn fetch_channel(
        &self,
        channel_id: serenity::ChannelId,
    ) -> Option<serenity::GuildChannel> {
        match channel_id.to_channel(&self.ctx).await {
            Ok(serenity::Channel::Guild(channel)) => Some(channel),
            Ok(_) => None,
            Err(e) => {
                println!("Failed to get channel.\n{}", e);
                None
            }
        }
    }

    fn is_staff(&self, channel: &serenity::GuildChannel, member: &serenity::Member) -> bool {
        !member.user.bot
            && self
                .permissions_in(channel, member)
                .map(|permissions| permissions.manage_channels())
                .unwrap_or_default()
    }

    async fn reveal(&self, channel: &mut serenity::GuildChannel) -> Result<bool, Error> {
        let Some(updated) = overwrites::reveal(&channel.permission_overwrites, channel.guild_id)
        else {
            return Ok(false);
        };

        channel
            .edit(
                &self.ctx.http,
                serenity::EditChannel::new().permissions(updated.clone()),
            )
            .await?;

        // Keep the in-memory copy in sync with what we just sent to Discord.
        channel.permission_overwrites = updated;

        Ok(true)
    }

    async fn conceal(&self, channel: &mut serenity::GuildChannel) -> Result<(), Error> {
        let updated = overwrites::conceal(&channel.permission_overwrites, channel.guild_id);

        channel
            .edit(
                &self.ctx.http,
                serenity::EditChannel::new().permissions(updated.clone()),
            )
            .await?;

        channel.permission_overwrites = updated;

        Ok(())
    }

    fn connected_staff(&self, channel: &serenity::GuildChannel) -> Option<serenity::Member> {
        match channel.members(&self.ctx.cache) {
            Ok(members) => members
                .into_iter()
                .find(|member| self.is_staff(channel, member)),
            Err(e) => {
                println!("Failed to get channel members\n{}", e);
                None
            }
        }
    }

    fn connected_members(
        &self,
        channel: &serenity::GuildChannel,
    ) -> Result<Vec<serenity::Member>, Error> {
        channel.members(&self.ctx.cache).map_err(Error::from)
    }

    async fn disconnect(&self, member: &serenity::Member) -> Result<(), Error> {
        member.disconnect_from_voice(&self.ctx.http).await?;

        Ok(())
    }
}
