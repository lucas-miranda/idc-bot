use std::collections::HashSet;

use poise::serenity_prelude as serenity;
use tracing::warn;

use crate::commands::Context;
use crate::domain::access::is_staff;
use crate::error::Error;

/// Lista as calls registradas.
#[poise::command(slash_command, guild_only)]
pub async fn listar(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let (calls, ignored, reload_roles) = {
        let voice = ctx.data().voice.read().await;
        let config = voice.config();

        (
            config.call_kinds.clone(),
            config.ignored_voice_channels.clone(),
            config.reload_roles.clone(),
        )
    };

    if calls.is_empty() {
        send(ctx, "Não há calls registradas. 😔").await?;
        return Ok(());
    }

    let Some(guild_id) = ctx.guild_id() else {
        send(ctx, "Use este comando em um servidor.").await?;
        return Ok(());
    };

    let owners = ctx.framework().options().owners.clone();
    let mut lines = Vec::with_capacity(calls.len());

    for call in calls {
        let (channel_name, active) =
            match fetch_channel(ctx.serenity_context(), guild_id, call.voice_channel).await {
                Some(channel) => {
                    let is_public = !ignored.contains(&channel.id);
                    let has_staff =
                        has_staff_member(ctx.serenity_context(), &channel, &owners, &reload_roles);

                    (channel.name, is_public && has_staff)
                }
                None => ("canal removido".to_owned(), false),
            };

        let emoji = if active { "🟢" } else { "🔴" };
        let channel_display = if active {
            format!("<#{}>", call.voice_channel.get())
        } else {
            channel_name
        };

        lines.push(format!(
            "- {emoji} <@&{}>: {channel_display}",
            call.role_id.get()
        ));
    }

    send(ctx, format!("Calls:\n{}", lines.join("\n"))).await?;
    Ok(())
}

async fn fetch_channel(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
    channel_id: serenity::ChannelId,
) -> Option<serenity::GuildChannel> {
    if let Some(guild) = ctx.cache.guild(guild_id)
        && let Some(channel) = guild.channels.get(&channel_id)
    {
        return Some(channel.clone());
    }

    match channel_id.to_channel(ctx).await {
        Ok(serenity::Channel::Guild(channel)) => Some(channel),
        Ok(_) => None,
        Err(error) => {
            warn!(%error, channel_id = %channel_id, "failed to fetch channel for /call listar");
            None
        }
    }
}

fn has_staff_member(
    ctx: &serenity::Context,
    channel: &serenity::GuildChannel,
    owners: &HashSet<serenity::UserId>,
    reload_roles: &[serenity::RoleId],
) -> bool {
    let members = match channel.members(&ctx.cache) {
        Ok(members) => members,
        Err(error) => {
            warn!(%error, channel_id = %channel.id, "failed to list channel members");
            return false;
        }
    };

    let guild = channel.guild_id.to_guild_cached(&ctx.cache);

    members
        .into_iter()
        .filter(|member| !member.user.bot)
        .any(|member| {
            let has_manage_guild = match guild.as_ref() {
                Some(guild) => guild.member_permissions(&member).manage_guild(),
                None => member
                    .permissions
                    .map(|permissions| permissions.manage_guild())
                    .unwrap_or(false),
            };

            is_staff(
                member.user.bot,
                owners.contains(&member.user.id),
                has_manage_guild,
                &member.roles,
                reload_roles,
            )
        })
}

async fn send(ctx: Context<'_>, content: impl Into<String>) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default()
            .content(content)
            .ephemeral(true),
    )
    .await?;

    Ok(())
}
