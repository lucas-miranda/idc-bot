use std::collections::{HashMap, HashSet};
use poise::serenity_prelude::{self as serenity, PartialGuild};
use crate::{channel::{ChannelExtras, ChannelIdExtras}, permissions::PermissionsExtras, voice::{err::MessageError, Broadcaster, CallKind, MessageKind, VoiceChannelManagerCreationError}};
use super::VoiceMoveAction;

pub struct VoiceChannelManager {
    pub ignore_voice_channels: HashSet<serenity::ChannelId>,
    broadcaster: Broadcaster<MessageKind>,
    message_by_channel: HashMap<serenity::ChannelId, MessageKind>
}

impl VoiceChannelManager {
    pub async fn new(
        ctx: &serenity::Context,
        guild_id: serenity::GuildId,
        ignore_voice_channels: Vec<serenity::ChannelId>,
    ) -> Result<VoiceChannelManager, VoiceChannelManagerCreationError> {
        // #chat-idc
        let broadcast_channel_id = serenity::ChannelId::new(1126997072961343660);

        // voice channels
        let entries = [
            (
                MessageKind::CallOpened(CallKind::Social),
                [
                    serenity::ChannelId::new(1160706169963294740)
                ]
            ),
            (
                MessageKind::CallOpened(CallKind::Games),
                [
                    serenity::ChannelId::new(1374542425116246066)
                ]
            ),
            (
                MessageKind::CallOpened(CallKind::Movies),
                [
                    serenity::ChannelId::new(1419472150644916244)
                ]
            )
        ];

        let mut message_by_channel = HashMap::new();
        for entry in entries {
            for id in &entry.1 {
                message_by_channel.insert(*id, entry.0.clone());
            }
        }

        // roles ids
        let social_role_id = serenity::RoleId::new(1274386535285788826);
        let games_role_id = serenity::RoleId::new(1287419235886436375);
        let movies_role_id = serenity::RoleId::new(1287419731447648396);

        match ctx.http.get_guild(guild_id).await {
                Ok(guild) => Ok({
                    VoiceChannelManager {
                        ignore_voice_channels: HashSet::from_iter(ignore_voice_channels),
                        message_by_channel,
                        broadcaster:
                            Broadcaster::new(
                                ctx,
                                broadcast_channel_id,
                                guild.clone(),
                                social_role_id,
                            )
                            .await?
                            .register_msg(
                                MessageKind::CallOpened(CallKind::Social),
                                |b| {
                                    match Self::default_msg(&guild, b, &social_role_id) {
                                        Ok(msg) => msg,
                                        Err(e) => {
                                            println!("Failed to send message: {}", e);
                                            String::new()
                                        }
                                    }
                                }
                            )
                            .register_msg(
                                MessageKind::CallOpened(CallKind::Games),
                                |b| {
                                    match Self::default_msg(&guild, b, &games_role_id) {
                                        Ok(msg) => msg,
                                        Err(e) => {
                                            println!("Failed to send message: {}", e);
                                            String::new()
                                        }
                                    }
                                }
                            )
                            .register_msg(
                                MessageKind::CallOpened(CallKind::Movies),
                                |b| {
                                    match Self::default_msg(&guild, b, &movies_role_id) {
                                        Ok(msg) => msg,
                                        Err(e) => {
                                            println!("Failed to send message: {}", e);
                                            String::new()
                                        }
                                    }
                                }
                            ),
                    }
                }),
                Err(e) => Err(e.into())
        }
    }

    pub async fn handle_state(&self, ctx: &serenity::Context, event: &serenity::FullEvent) -> Result<(), crate::Error> {
         //&& let Some(member) = new.member.as_ref()
        if let serenity::FullEvent::VoiceStateUpdate { old, new } = event {
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
                    if let Some(member) = new.member.as_ref()
                     && let Some(new_channel_id) = new.channel_id.as_ref()
                     && let Some(mut guild_channel) = new_channel_id.to_guild_channel(ctx).await
                    {
                        self.user_entering(ctx, member, &mut guild_channel).await?;
                    },
                VoiceMoveAction::Leave =>
                    if let Some(state) = old.as_ref()
                     && let Some(ref member) = state.member
                     && let Some(old_channel_id) = state.channel_id
                     && let Some(mut guild_channel) = old_channel_id.to_guild_channel(ctx).await
                    {
                        self.user_leaving(ctx, member, &mut guild_channel).await?;
                    },
                VoiceMoveAction::Moving =>
                    if let Some(old_state) = old.as_ref()
                     && let Some(ref old_member) = old_state.member
                     && let Some(old_channel_id) = old_state.channel_id
                     && let Some(ref new_member) = new.member
                     && let Some(new_channel_id) = new.channel_id
                    {
                        let from = old_channel_id.to_guild_channel(ctx).await;
                        let to = new_channel_id.to_guild_channel(ctx).await;
                        self.user_moving(ctx, (old_member, from), (new_member, to)).await?;
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
            self.staff_entering_public_voice_channel(ctx, member, guild_channel, None).await?;
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
        old: (&serenity::Member, Option<serenity::GuildChannel>),
        new: (&serenity::Member, Option<serenity::GuildChannel>),
    ) -> Result<(), crate::Error> {
        let (old_member, mut old_channel) = old;
        let (new_member, new_channel) = new;
        println!("{} moved from voice channel", old_member.display_name());

        if let Some(from) = &mut old_channel
         && old_member.is_staff(ctx, from)
         && self.is_public_voice_channel(from)
        {
            println!("  from voice channel {}", from.name);
            self.staff_leaving_public_voice_channel(ctx, old_member, from).await?;
        }

        if let Some(mut to) = new_channel
         && new_member.is_staff(ctx, &to)
         && self.is_public_voice_channel(&to)
        {
            println!("  to voice channel {}", to.name);
            self.staff_entering_public_voice_channel(
                    ctx,
                    new_member,
                    &mut to,
                    old_channel.as_mut(),
                ).await?;
        }

        Ok(())
    }

    async fn staff_entering_public_voice_channel(
        &self,
        ctx: &serenity::Context,
        _member: &serenity::Member,
        guild_channel: &mut serenity::GuildChannel,
        previous_guild_channel: Option<&mut serenity::GuildChannel>,
    ) -> Result<(), crate::Error> {
        println!("  changing channel to be visible...");

        if guild_channel.make_visible(ctx).await? {
            println!("  done!");
            let everyone_role_kind = serenity::PermissionOverwriteType::Role(guild_channel.guild_id.everyone_role());

            let everyone_can_speak_on_channel
                = guild_channel
                    .get_permissions_overwrite(everyone_role_kind)
                    .map(|p| !p.deny.speak())
                    .unwrap_or(true);

            // if previous_guild_channel is none, it'll be false
            let everyone_can_speak_on_prev_channel
                = previous_guild_channel
                    .map(|ch| ch.get_permissions_overwrite(everyone_role_kind)
                                .map(|p| !p.deny.speak())
                                .unwrap_or(true))
                    .unwrap_or_default();

            if everyone_can_speak_on_prev_channel {
                println!("  not mentioning {} role, moving from another social channel", self.broadcaster.social_role.name);
            } else if !everyone_can_speak_on_channel {
                println!("  not mentioning {} role, isn't a social channel", self.broadcaster.social_role.name);
            } else {
                println!("  mentioning {} role", self.broadcaster.social_role.name);

                match self.message_by_channel.get(&guild_channel.id) {
                    Some(msg_kind) => {
                        println!("  done!");
                        self.broadcaster.send_msg(ctx, msg_kind).await?;
                    },
                    None => {
                        println!("  failed to find channel...");
                    },
                }
            }
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

    fn default_msg(guild: &PartialGuild, b: &mut Broadcaster<MessageKind>, role_id: &serenity::RoleId) -> Result<String, MessageError> {
        let mut content = serenity::MessageBuilder::new();

        let role = match guild.roles.get(role_id) {
            Some(r) => r.clone(),
            None => {
                content.build();
                return Err(MessageError::RoleNotFound(*role_id))
            }
        };

        content.mention(&role)
               .push(" call aberta! ");

        let pride_heart_emoji_id = serenity::EmojiId::new(1073325566196990142);
        if let Some(pride_heart_emoji) = b.emojis.get(&pride_heart_emoji_id) {
            content.emoji(pride_heart_emoji);
        }

        Ok(content.build())
    }
}
