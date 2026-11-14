mod call;
pub use call::call;

mod recarregar;
pub use recarregar::recarregar;

use std::{collections::HashSet, sync::Arc};

use poise::serenity_prelude as serenity;

use crate::{domain::access::can_reload, error::Error, state::AppState};

pub(crate) type Context<'a> = poise::Context<'a, AppState, Error>;

/// Gate the administrative commands behind the same rule as `/recarregar`.
pub(crate) async fn ensure_authorized(ctx: Context<'_>) -> Result<bool, Error> {
    if is_authorized(ctx).await {
        return Ok(true);
    }

    ctx.send(
        poise::CreateReply::default()
            .content("Você não tem permissão para usar esse comando.")
            .ephemeral(true),
    )
    .await?;

    Ok(false)
}

pub(crate) async fn is_authorized(ctx: Context<'_>) -> bool {
    let Some(member) = ctx.author_member().await else {
        return false;
    };

    let allowed_roles = {
        let voice = ctx.data().voice.read().await;
        voice.config().reload_roles.clone()
    };

    member_is_authorized(
        &ctx.serenity_context().cache,
        ctx.author().id,
        &member,
        ctx.guild_id(),
        &ctx.framework().options().owners,
        &allowed_roles,
    )
}

pub(crate) fn member_is_authorized(
    cache: &Arc<serenity::Cache>,
    user_id: serenity::UserId,
    member: &serenity::Member,
    guild_id: Option<serenity::GuildId>,
    owners: &HashSet<serenity::UserId>,
    allowed_roles: &[serenity::RoleId],
) -> bool {
    if owners.contains(&user_id) {
        return true;
    }

    let has_manage_guild = match member.permissions {
        Some(permissions) => permissions.manage_guild(),
        None => guild_id
            .and_then(|id| id.to_guild_cached(cache))
            .map(|guild| guild.member_permissions(member).manage_guild())
            .unwrap_or(false),
    };

    can_reload(false, has_manage_guild, &member.roles, allowed_roles)
}
