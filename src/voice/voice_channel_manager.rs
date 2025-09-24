use std::collections::HashSet;
use poise::serenity_prelude::{self as serenity};

use crate::{channel::{ChannelExtras, ChannelIdExtras}, permissions::PermissionsExtras};

use super::VoiceMoveAction;

#[derive(Default)]
pub struct VoiceChannelManager {
    pub ignore_voice_channels: HashSet<serenity::ChannelId>,
}

impl VoiceChannelManager {
    pub async fn handle_state(&self, ctx: &serenity::Context, event: &serenity::FullEvent) -> Result<(), crate::Error> {
        if let serenity::FullEvent::VoiceStateUpdate { old, new } = event
         && let Some(member) = new.member.as_ref()
        {
            // verify which action is being taken by member
            let action = {
                if old.is_none() {
                    VoiceMoveAction::Enter
                } else if new.channel_id.is_none() {
                    VoiceMoveAction::Leave
                } else if old.as_ref().is_some_and(|s| s.channel_id != new.channel_id) {
                    VoiceMoveAction::Moving
                } else {
                    VoiceMoveAction::Undefined
                }
            };

            // handle actions
            match action {
                VoiceMoveAction::Undefined => {
                    println!("Something undefined happened at voice channel.")
                },
                VoiceMoveAction::Enter =>
                    if let Some(new_channel_id) = new.channel_id.as_ref() {
                        match new_channel_id.to_guild_channel(ctx).await {
                            Ok(channel) => if let Some(mut guild_channel) = channel {
                                self.user_entering(ctx, member, &mut guild_channel).await?;
                            },
                            Err(e) => {
                                println!("Failed to get channel.\n{}", e);
                            },
                        }
                    },
                VoiceMoveAction::Leave =>
                    if let Some(old_channel_id) = old.as_ref().and_then(|s| s.channel_id) {
                        match old_channel_id.to_guild_channel(ctx).await {
                            Ok(channel) => if let Some(mut guild_channel) = channel {
                                self.user_leaving(ctx, member, &mut guild_channel).await?;
                            },
                            Err(e) => {
                                println!("Failed to get channel.\n{}", e);
                            },
                        }
                    },
                VoiceMoveAction::Moving =>
                    if let Some(old_channel_id) = old.as_ref().and_then(|s| s.channel_id)
                     && let Some(new_channel_id) = new.channel_id.as_ref()
                    {
                        let from = match old_channel_id.to_guild_channel(ctx).await {
                            Ok(channel) => channel,
                            Err(e) => {
                                println!("Failed to get channel.\n{}", e);
                                None
                            },
                        };

                        let to = match new_channel_id.to_guild_channel(ctx).await {
                            Ok(channel) => channel,
                            Err(e) => {
                                println!("Failed to get channel.\n{}", e);
                                None
                            },
                        };

                        self.user_moving(ctx, member, from, to).await?;
                    },
            }
        }

        Ok(())
    }

    pub fn is_public_voice_channel(&self, channel: &serenity::GuildChannel) -> bool {
        !self.ignore_voice_channels.contains(&channel.id)
    }

    /// An user just entered a voice channel.
    async fn user_entering(
        &self,
        ctx: &serenity::Context,
        member: &serenity::Member,
        guild_channel: &mut serenity::GuildChannel
    ) -> Result<(), crate::Error> {
        if member.is_staff(ctx, guild_channel) && self.is_public_voice_channel(guild_channel) {
            println!("{} entered voice channel {}", member.display_name(), guild_channel.name);
            println!("  changing channel to be visible...");
            guild_channel.make_visible(ctx).await?;
            println!("  done!");
        }

        Ok(())
    }

    /// An user is leaving a voice channel.
    async fn user_leaving(
        &self,
        ctx: &serenity::Context,
        member: &serenity::Member,
        guild_channel: &mut serenity::GuildChannel
    ) -> Result<(), crate::Error> {
        if member.is_staff(ctx, guild_channel) && self.is_public_voice_channel(guild_channel) {
            println!("{} left voice channel {}", member.display_name(), guild_channel.name);

            let is_staff_connected = match guild_channel.members(&ctx.cache) {
                Ok(members) => {
                    let staff_member = members.iter()
                                              .find(|channel_member| channel_member.is_staff(ctx, guild_channel));

                    if let Some(m) = staff_member {
                        println!("  Staff member {} is still connected to channel", m.display_name());
                        true
                    } else {
                        false
                    }
                },
                Err(e) => {
                    println!("Failed to get channel members\n{}", e);
                    false
                }
            };

            if !is_staff_connected {
                println!("  changing channel to be invisible...");
                guild_channel.make_invisible(ctx).await?;
                println!("  done!");
            }
        }

        Ok(())
    }

    /// An user is moving from a voice channel to another one.
    async fn user_moving(
        &self,
        ctx: &serenity::Context,
        member: &serenity::Member,
        from_guild_channel: Option<serenity::GuildChannel>,
        to_guild_channel: Option<serenity::GuildChannel>
    ) -> Result<(), crate::Error> {
        println!("{} moved from voice channel", member.display_name());

        if let Some(mut from) = from_guild_channel
         && member.is_staff(ctx, &from)
         && self.is_public_voice_channel(&from)
        {
            println!("  from voice channel {}", from.name);
            println!("  changing channel to be invisible...");
            from.make_invisible(ctx).await?;
            println!("  done!");
        }

        if let Some(mut to) = to_guild_channel
         && member.is_staff(ctx, &to)
         && self.is_public_voice_channel(&to)
        {
            println!("  to voice channel {}", to.name);
            println!("  changing channel to be visible...");
            to.make_visible(ctx).await?;
            println!("  done!");
        }

        Ok(())
    }
}
