use tracing::error;

use super::Context;
use crate::domain::access::can_reload;
use crate::error::Error;

/// Reload the bot configuration from the TOML file.
#[poise::command(slash_command, guild_only)]
pub async fn reload(ctx: Context<'_>) -> Result<(), Error> {
    if !is_authorized(ctx).await {
        ctx.send(
            poise::CreateReply::default()
                .content("You are not allowed to reload the configuration.")
                .ephemeral(true),
        )
        .await?;

        return Ok(());
    }

    ctx.defer_ephemeral().await?;

    match ctx.data().reload(ctx.serenity_context()).await {
        Ok(()) => {
            ctx.send(
                poise::CreateReply::default()
                    .content("Configuration reloaded.")
                    .ephemeral(true),
            )
            .await?;
        }
        Err(error) => {
            error!(%error, "configuration reload failed");

            ctx.send(
                poise::CreateReply::default()
                    .content(format!("Reload failed: {error}"))
                    .ephemeral(true),
            )
            .await?;
        }
    }

    Ok(())
}

async fn is_authorized(ctx: Context<'_>) -> bool {
    let is_owner = ctx.framework().options().owners.contains(&ctx.author().id);

    if is_owner {
        return true;
    }

    let Some(member) = ctx.author_member().await else {
        return false;
    };

    let has_manage_guild = match member.permissions {
        Some(permissions) => permissions.manage_guild(),
        None => ctx
            .guild_id()
            .and_then(|guild_id| guild_id.to_guild_cached(&ctx.serenity_context().cache))
            .map(|guild| guild.member_permissions(&member).manage_guild())
            .unwrap_or(false),
    };

    let allowed_roles = {
        let voice = ctx.data().voice.read().await;
        voice.config().reload_roles.clone()
    };

    can_reload(false, has_manage_guild, &member.roles, &allowed_roles)
}
