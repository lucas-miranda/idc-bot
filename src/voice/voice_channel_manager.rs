use std::collections::HashSet;
use poise::serenity_prelude::{self as serenity};
use crate::{channel::{ChannelExtras, ChannelIdExtras}, permissions::PermissionsExtras, voice::{Broadcaster, VoiceChannelManagerCreationError, MessageKind}};
use super::VoiceMoveAction;

pub struct VoiceChannelManager {
    pub ignore_voice_channels: HashSet<serenity::ChannelId>,
    broadcaster: Broadcaster<MessageKind>,
}

impl VoiceChannelManager {
    pub async fn new(
        ctx: &serenity::Context,
        guild_id: serenity::GuildId,
        ignore_voice_channels: Vec<serenity::ChannelId>,
    ) -> Result<VoiceChannelManager, VoiceChannelManagerCreationError> {
        // #chat-idc
        let broadcast_channel_id = serenity::ChannelId::new(1126997072961343660);

        // @Social
        let social_role_id = serenity::RoleId::new(1274386535285788826);

        match ctx.http.get_guild(guild_id).await {
                Ok(guild) => Ok({

                    VoiceChannelManager {
                        ignore_voice_channels: HashSet::from_iter(ignore_voice_channels),
                        broadcaster:
                            Broadcaster::new(
                                ctx,
                                broadcast_channel_id,
                                guild,
                                social_role_id,
                            )
                            .await?
                            .register_msg(
                                MessageKind::CallOpened,
                                |b| {
                                    let mut content = serenity::MessageBuilder::new();

                                    content.mention(&b.social_role)
                                           .push(" call aberta! ");

                                    let pride_heart_emoji_id = serenity::EmojiId::new(1073325566196990142);
                                    if let Some(pride_heart_emoji) = b.emojis.get(&pride_heart_emoji_id) {
                                        content.emoji(pride_heart_emoji);
                                    }

                                    content.build()
                                },
                            ),
                    }
                }),
                Err(e) => Err(e.into())
        }
    }

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
                    /*
                    println!("Something undefined happened at voice channel.");
                    println!("-> old: {:?}", old);
                    println!("-> new: {:?}", new);
                    */
                },
                VoiceMoveAction::Enter =>
                    if let Some(new_channel_id) = new.channel_id.as_ref()
                     && let Some(mut guild_channel) = new_channel_id.to_guild_channel(ctx).await
                    {
                        self.user_entering(ctx, member, &mut guild_channel).await?;
                    },
                VoiceMoveAction::Leave =>
                    if let Some(old_channel_id) = old.as_ref().and_then(|s| s.channel_id)
                     && let Some(mut guild_channel) = old_channel_id.to_guild_channel(ctx).await
                    {
                        self.user_leaving(ctx, member, &mut guild_channel).await?;
                    },
                VoiceMoveAction::Moving =>
                    if let Some(old_channel_id) = old.as_ref().and_then(|s| s.channel_id)
                     && let Some(new_channel_id) = new.channel_id.as_ref()
                    {
                        let from = old_channel_id.to_guild_channel(ctx).await;
                        let to = new_channel_id.to_guild_channel(ctx).await;
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
            self.staff_entering_public_voice_channel(ctx, member, guild_channel).await?;
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
            self.staff_leaving_public_voice_channel(ctx, member, guild_channel).await?;
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
            self.staff_leaving_public_voice_channel(ctx, member, &mut from).await?;
        }

        if let Some(mut to) = to_guild_channel
         && member.is_staff(ctx, &to)
         && self.is_public_voice_channel(&to)
        {
            println!("  to voice channel {}", to.name);
            self.staff_entering_public_voice_channel(ctx, member, &mut to).await?;
        }

        Ok(())
    }

    async fn staff_entering_public_voice_channel(
        &self,
        ctx: &serenity::Context,
        _member: &serenity::Member,
        guild_channel: &mut serenity::GuildChannel
    ) -> Result<(), crate::Error> {
        println!("  changing channel to be visible...");

        if guild_channel.make_visible(ctx).await? {
            println!("  done!");
            println!("  mentioning {} role", self.broadcaster.social_role.name);
            self.broadcaster.send_msg(ctx, &MessageKind::CallOpened).await?;
            println!("  done!");
        } else {
            println!("  there is nothing to do!");
        }

        Ok(())
    }

    async fn staff_leaving_public_voice_channel(
        &self,
        ctx: &serenity::Context,
        _member: &serenity::Member,
        guild_channel: &mut serenity::GuildChannel
    ) -> Result<(), crate::Error> {
        match guild_channel.get_connected_staff_member(ctx) {
            Some(m) => {
                println!("  Staff member {} is still connected to channel", m.display_name());
            },
            None => {
                println!("  changing channel to be invisible...");
                guild_channel.make_invisible(ctx).await?;
                println!("  kick remaining connected members...");

                match guild_channel.members(&ctx.cache) {
                    Ok(members) => {
                        // disconnect everyone

                        for m in members {
                            m.disconnect_from_voice(&ctx.http).await?;
                        }
                        println!("  done!");
                    },
                    Err(e) => {
                        println!("Failed to get channel members\n{}", e);
                    },
                }
            },
        }

        Ok(())
    }
}
