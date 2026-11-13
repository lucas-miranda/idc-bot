use std::sync::Arc;

use poise::serenity_prelude as serenity;

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
            println!(
                "{} entered voice channel {}",
                member.display_name(),
                channel.name
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
            println!(
                "{} left voice channel {}",
                member.display_name(),
                channel.name
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
        println!("{} moved from voice channel", old_member.display_name());

        if let Some(from) = &mut old_channel
            && self.channels.is_staff(from, old_member)
            && self.is_public_voice_channel(from)
        {
            println!("  from voice channel {}", from.name);

            self.staff_leaving_public_voice_channel(old_member, from)
                .await?;
        }

        if let Some(mut to) = new_channel
            && self.channels.is_staff(&to, new_member)
            && self.is_public_voice_channel(&to)
        {
            println!("  to voice channel {}", to.name);

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
        println!("  changing channel to be visible...");

        if self.channels.reveal(channel).await? {
            println!("  done!");

            let everyone_can_speak_on_channel =
                overwrites::allows_speak(&channel.permission_overwrites, channel.guild_id);

            // If there is no previous channel, this is `false`.
            let everyone_can_speak_on_previous_channel = previous_channel
                .map(|previous| {
                    overwrites::allows_speak(&previous.permission_overwrites, previous.guild_id)
                })
                .unwrap_or_default();

            if everyone_can_speak_on_previous_channel {
                println!(
                    "  not mentioning {} role, moving from another social channel",
                    self.notifier.social_role_name()
                );
            } else if !everyone_can_speak_on_channel {
                println!(
                    "  not mentioning {} role, isn't a social channel",
                    self.notifier.social_role_name()
                );
            } else {
                println!("  mentioning {} role", self.notifier.social_role_name());

                match self.config.kind_for_channel(channel.id) {
                    Some(call_kind) => {
                        println!("  done!");
                        self.notifier.notify(&call_kind.kind).await?;
                    }
                    None => {
                        println!("  failed to find channel...");
                    }
                }
            }
        } else {
            println!("  there is nothing to do!");
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
                println!(
                    "  Staff member {} is still connected to channel",
                    member.display_name()
                );
            }
            None => {
                println!("  changing channel to be invisible...");
                self.channels.conceal(channel).await?;
                println!("  kick remaining connected members...");

                match self.channels.connected_members(channel) {
                    Ok(members) => {
                        for member in members {
                            self.channels.disconnect(&member).await?;
                        }
                        println!("  done!");
                    }
                    Err(e) => {
                        println!("Failed to get channel members\n{}", e);
                    }
                }
            }
        }

        Ok(())
    }
}
