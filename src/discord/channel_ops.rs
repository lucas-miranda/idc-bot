use std::collections::HashSet;

use poise::serenity_prelude as serenity;

use crate::Error;
use crate::domain::access::is_staff as is_staff_member;
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
                tracing::warn!(%e, channel_id = %channel_id, "failed to fetch channel");
                None
            }
        }
    }

    fn is_staff(
        &self,
        member: &serenity::Member,
        owners: &HashSet<serenity::UserId>,
        reload_roles: &[serenity::RoleId],
    ) -> bool {
        let has_manage_guild = member
            .guild_id
            .to_guild_cached(&self.ctx.cache)
            .map(|guild| guild.member_permissions(member).manage_guild())
            .or_else(|| {
                member
                    .permissions
                    .map(|permissions| permissions.manage_guild())
            })
            .unwrap_or(false);

        is_staff_member(
            member.user.bot,
            owners.contains(&member.user.id),
            has_manage_guild,
            &member.roles,
            reload_roles,
        )
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

    fn connected_staff(
        &self,
        channel: &serenity::GuildChannel,
        owners: &HashSet<serenity::UserId>,
        reload_roles: &[serenity::RoleId],
        exclude: Option<serenity::UserId>,
    ) -> Option<serenity::Member> {
        match channel.members(&self.ctx.cache) {
            Ok(members) => members.into_iter().find(|member| {
                Some(member.user.id) != exclude && self.is_staff(member, owners, reload_roles)
            }),
            Err(e) => {
                tracing::warn!(%e, channel_id = %channel.id, "failed to list channel members");
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
