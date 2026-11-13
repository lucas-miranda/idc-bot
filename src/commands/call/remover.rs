use poise::serenity_prelude as serenity;
use tracing::error;

use crate::commands::{Context, ensure_authorized, is_authorized};
use crate::config::CallConfig;
use crate::error::Error;

/// Remove uma call registrada.
#[poise::command(slash_command, guild_only)]
pub async fn remover(
    ctx: Context<'_>,
    #[description = "Call registrada"]
    #[autocomplete = "autocomplete_call"]
    call: String,
) -> Result<(), Error> {
    if !ensure_authorized(ctx).await? {
        return Ok(());
    }

    let calls = current_calls(ctx).await;
    if calls.is_empty() {
        send(ctx, "Não há calls registradas. 😔").await?;
        return Ok(());
    }

    let Ok(channel_id) = call.parse::<u64>().map(serenity::ChannelId::new) else {
        send(ctx, "Escolha uma call registrada.").await?;
        return Ok(());
    };

    if !calls
        .iter()
        .any(|registered| registered.voice_channel == channel_id)
    {
        send(ctx, "Escolha uma call registrada.").await?;
        return Ok(());
    }

    ctx.defer_ephemeral().await?;

    match ctx
        .data()
        .remove_call(ctx.serenity_context(), channel_id)
        .await
    {
        Ok(true) => send(ctx, "Call removida!").await?,
        Ok(false) => send(ctx, "Essa call não está mais registrada.").await?,
        Err(error) => {
            error!(%error, "failed to remove call");
            send(ctx, format!("Falha ao remover a call: {error}")).await?;
        }
    }

    Ok(())
}

pub async fn autocomplete_call<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> serenity::CreateAutocompleteResponse {
    if !is_authorized(ctx).await {
        return serenity::CreateAutocompleteResponse::new();
    }

    let calls = current_calls(ctx).await;
    let needle = partial.to_lowercase();

    let Some(guild_id) = ctx.guild_id() else {
        return serenity::CreateAutocompleteResponse::new();
    };

    let choices = calls
        .into_iter()
        .filter_map(|registered| {
            let name = channel_name(ctx.serenity_context(), guild_id, registered.voice_channel)
                .unwrap_or_else(|| "canal removido".to_owned());

            name.to_lowercase().contains(&needle).then(|| {
                serenity::AutocompleteChoice::new(name, registered.voice_channel.get().to_string())
            })
        })
        .take(25)
        .collect();

    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}

async fn current_calls(ctx: Context<'_>) -> Vec<CallConfig> {
    let voice = ctx.data().voice.read().await;
    voice.config().call_kinds.clone()
}

fn channel_name(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
    channel_id: serenity::ChannelId,
) -> Option<String> {
    let guild = ctx.cache.guild(guild_id)?;
    guild
        .channels
        .get(&channel_id)
        .map(|channel| channel.name.clone())
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
