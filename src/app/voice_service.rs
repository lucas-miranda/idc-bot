use std::sync::Arc;

use poise::serenity_prelude as serenity;
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::domain::{CallEvent, overwrites};
use crate::error::Error;
use crate::ports::{Notifier, VoiceChannelPort};

/// Use cases around public voice channels.
///
/// This is the old `VoiceChannelManager`, but it no longer knows about Discord:
/// it talks to the [`VoiceChannelPort`] and [`Notifier`] contracts instead.
pub struct VoiceService<C: VoiceChannelPort, N: Notifier> {
    config: Arc<Config>,
    channels: C,
    notifier: N,
}

impl<C: VoiceChannelPort, N: Notifier> VoiceService<C, N> {
    pub fn new(config: Arc<Config>, channels: C, notifier: N) -> Self {
        Self {
            config,
            channels,
            notifier,
        }
    }

    /// The configuration currently backing this service.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Handle a single voice state update.
    pub async fn handle_transition(
        &self,
        old: Option<&serenity::VoiceState>,
        new: &serenity::VoiceState,
    ) -> Result<(), Error> {
        let old_channel = old.map(|state| state.channel_id.map(|id| id.get()));
        let new_channel = new.channel_id.map(|id| id.get());

        match CallEvent::classify(old_channel, new_channel) {
            CallEvent::Other => {}
            CallEvent::Enter => {
                if let Some(member) = new.member.as_ref()
                    && let Some(channel_id) = new.channel_id
                    && let Some(mut channel) = self.channels.fetch_channel(channel_id).await
                {
                    self.user_entering(member, &mut channel).await?;
                }
            }
            CallEvent::Leave => {
                if let Some(state) = old
                    && let Some(member) = state.member.as_ref()
                    && let Some(channel_id) = state.channel_id
                    && let Some(mut channel) = self.channels.fetch_channel(channel_id).await
                {
                    self.user_leaving(member, &mut channel).await?;
                }
            }
            CallEvent::Move => {
                if let Some(old_state) = old
                    && let Some(old_member) = old_state.member.as_ref()
                    && let Some(old_channel_id) = old_state.channel_id
                    && let Some(new_member) = new.member.as_ref()
                    && let Some(new_channel_id) = new.channel_id
                {
                    let from = self.channels.fetch_channel(old_channel_id).await;
                    let to = self.channels.fetch_channel(new_channel_id).await;

                    self.user_moving(old_member, from, new_member, to).await?;
                }
            }
        }

        Ok(())
    }

    fn is_public_voice_channel(&self, channel: &serenity::GuildChannel) -> bool {
        !self.config.ignored_voice_channels.contains(&channel.id)
    }

    /// A user just entered a voice channel.
    async fn user_entering(
        &self,
        member: &serenity::Member,
        channel: &mut serenity::GuildChannel,
    ) -> Result<(), Error> {
        if self.channels.is_staff(channel, member) && self.is_public_voice_channel(channel) {
            debug!(
                member = %member.display_name(),
                channel = %channel.name,
                "member entered voice channel"
            );

            self.staff_entering_public_voice_channel(member, channel, None)
                .await?;
        }

        Ok(())
    }

    /// A user is leaving a voice channel.
    async fn user_leaving(
        &self,
        member: &serenity::Member,
        channel: &mut serenity::GuildChannel,
    ) -> Result<(), Error> {
        if self.channels.is_staff(channel, member) && self.is_public_voice_channel(channel) {
            debug!(
                member = %member.display_name(),
                channel = %channel.name,
                "member left voice channel"
            );

            self.staff_leaving_public_voice_channel(member, channel)
                .await?;
        }

        Ok(())
    }

    /// A user is moving from one voice channel to another.
    async fn user_moving(
        &self,
        old_member: &serenity::Member,
        mut old_channel: Option<serenity::GuildChannel>,
        new_member: &serenity::Member,
        new_channel: Option<serenity::GuildChannel>,
    ) -> Result<(), Error> {
        debug!(member = %old_member.display_name(), "member moved voice channel");

        if let Some(from) = &mut old_channel
            && self.channels.is_staff(from, old_member)
            && self.is_public_voice_channel(from)
        {
            debug!(channel = %from.name, "leaving channel");

            self.staff_leaving_public_voice_channel(old_member, from)
                .await?;
        }

        if let Some(mut to) = new_channel
            && self.channels.is_staff(&to, new_member)
            && self.is_public_voice_channel(&to)
        {
            debug!(channel = %to.name, "entering channel");

            self.staff_entering_public_voice_channel(new_member, &mut to, old_channel.as_mut())
                .await?;
        }

        Ok(())
    }

    async fn staff_entering_public_voice_channel(
        &self,
        _member: &serenity::Member,
        channel: &mut serenity::GuildChannel,
        previous_channel: Option<&mut serenity::GuildChannel>,
    ) -> Result<(), Error> {
        debug!("making channel visible");

        if self.channels.reveal(channel).await? {
            debug!("channel is now visible");

            let everyone_can_speak_on_channel =
                overwrites::allows_speak(&channel.permission_overwrites, channel.guild_id);

            // If there is no previous channel, this is `false`.
            let everyone_can_speak_on_previous_channel = previous_channel
                .map(|previous| {
                    overwrites::allows_speak(&previous.permission_overwrites, previous.guild_id)
                })
                .unwrap_or_default();

            if everyone_can_speak_on_previous_channel {
                debug!(
                    role = %self.notifier.social_role_name(),
                    "not announcing: moved from another social channel"
                );
            } else if !everyone_can_speak_on_channel {
                debug!(
                    role = %self.notifier.social_role_name(),
                    "not announcing: not a social channel"
                );
            } else {
                info!(
                    channel = %channel.name,
                    role = %self.notifier.social_role_name(),
                    "announcing call"
                );

                match self.config.kind_for_channel(channel.id) {
                    Some(call_kind) => {
                        self.notifier.notify(&call_kind.kind).await?;
                    }
                    None => {
                        warn!(
                            channel_id = %channel.id,
                            "no call kind configured for channel; skipping announcement"
                        );
                    }
                }
            }
        } else {
            debug!("nothing to do; channel was already visible");
        }

        Ok(())
    }

    async fn staff_leaving_public_voice_channel(
        &self,
        _member: &serenity::Member,
        channel: &mut serenity::GuildChannel,
    ) -> Result<(), Error> {
        match self.channels.connected_staff(channel) {
            Some(member) => {
                debug!(
                    member = %member.display_name(),
                    "staff member still connected; keeping channel visible"
                );
            }
            None => {
                debug!("making channel invisible");
                self.channels.conceal(channel).await?;

                debug!("disconnecting remaining members");

                match self.channels.connected_members(channel) {
                    Ok(members) => {
                        for member in members {
                            self.channels.disconnect(&member).await?;
                        }
                        debug!("remaining members disconnected");
                    }
                    Err(error) => {
                        warn!(%error, "failed to list channel members; skipping disconnect");
                    }
                }
            }
        }

        Ok(())
    }
}
