use std::collections::HashMap;

use poise::serenity_prelude as serenity;
use tracing::error;

use crate::commands::{Context, ensure_authorized, is_authorized};
use crate::config::edit::UpsertOutcome;
use crate::error::Error;

/// Registra ou atualiza uma call.
#[poise::command(slash_command, guild_only)]
pub async fn registrar(
    ctx: Context<'_>,
    #[description = "Canal de voz"]
    #[channel_types("Voice")]
    voice_channel: serenity::GuildChannel,
    #[description = "Nome da tag (role) a ser mencionada"]
    #[autocomplete = "autocomplete_role"]
    role: String,
) -> Result<(), Error> {
    if !ensure_authorized(ctx).await? {
        return Ok(());
    }

    if voice_channel.kind != serenity::ChannelType::Voice {
        send(ctx, "Por favor, selecione um canal de voz.").await?;
        return Ok(());
    }

    let Some(role_id) = resolve_role(ctx, &role).await? else {
        send(
            ctx,
            format!(
                "Tag '{}' ainda não existe, por favor, crie nas configurações antes de usar esse comando.",
                role.trim()
            ),
        )
        .await?;
        return Ok(());
    };

    ctx.defer_ephemeral().await?;

    match ctx
        .data()
        .upsert_call(ctx.serenity_context(), voice_channel.id, role_id)
        .await
    {
        Ok(UpsertOutcome::Added) => send(ctx, "Call registrada!").await?,
        Ok(UpsertOutcome::Updated) => send(ctx, "Call atualizada!").await?,
        Ok(UpsertOutcome::Unchanged) => send(ctx, "Nenhuma alteração necessária.").await?,
        Err(error) => {
            error!(%error, "failed to register call");
            send(ctx, format!("Falha ao registrar a call: {error}")).await?;
        }
    }

    Ok(())
}

async fn resolve_role(ctx: Context<'_>, typed: &str) -> Result<Option<serenity::RoleId>, Error> {
    let Some(guild_id) = ctx.guild_id() else {
        return Ok(None);
    };

    let roles = guild_roles(ctx.serenity_context(), guild_id).await?;

    Ok(find_role(&roles, typed))
}

async fn guild_roles(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
) -> Result<HashMap<serenity::RoleId, serenity::Role>, Error> {
    if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
        return Ok(guild.roles.clone());
    }

    Ok(ctx.http.get_guild(guild_id).await?.roles)
}

pub async fn autocomplete_role<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> serenity::CreateAutocompleteResponse {
    if !is_authorized(ctx).await {
        return serenity::CreateAutocompleteResponse::new();
    }

    let Some(guild_id) = ctx.guild_id() else {
        return serenity::CreateAutocompleteResponse::new();
    };

    let Ok(roles) = guild_roles(ctx.serenity_context(), guild_id).await else {
        return serenity::CreateAutocompleteResponse::new();
    };

    let needle = partial.trim().trim_start_matches('@').trim().to_lowercase();

    let mut matches: Vec<&serenity::Role> = roles
        .values()
        .filter(|role| role.id != guild_id.everyone_role())
        .filter(|role| needle.is_empty() || role.name.to_lowercase().contains(&needle))
        .collect();

    matches.sort_by_key(|role| role.name.to_lowercase());
    matches.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));

    let choices: Vec<serenity::AutocompleteChoice> = matches
        .into_iter()
        .take(25)
        .map(|role| serenity::AutocompleteChoice::from(role.name.clone()))
        .collect();

    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}

fn find_role(
    roles: &HashMap<serenity::RoleId, serenity::Role>,
    typed: &str,
) -> Option<serenity::RoleId> {
    roles
        .values()
        .find(|role| role_name_matches(&role.name, typed))
        .map(|role| role.id)
}

fn role_name_matches(name: &str, typed: &str) -> bool {
    let needle = typed.trim().trim_start_matches('@').trim();
    name.trim().eq_ignore_ascii_case(needle)
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

#[cfg(test)]
mod tests {
    use super::role_name_matches;

    #[test]
    fn matches_case_insensitively() {
        assert!(role_name_matches("Staff", "staff"));
    }

    #[test]
    fn matches_with_a_leading_at() {
        assert!(role_name_matches("Staff", "@Staff"));
    }

    #[test]
    fn trims_whitespace() {
        assert!(role_name_matches("Staff", "  staff  "));
    }

    #[test]
    fn rejects_unknown_names() {
        assert!(!role_name_matches("Staff", "member"));
    }
}
