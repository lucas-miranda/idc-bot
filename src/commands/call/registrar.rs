use poise::serenity_prelude as serenity;
use tracing::error;

use crate::commands::{Context, ensure_authorized};
use crate::config::edit::UpsertOutcome;
use crate::error::Error;

/// Registra ou atualiza uma call.
#[poise::command(slash_command, guild_only)]
pub async fn registrar(
    ctx: Context<'_>,
    #[description = "Canal de voz"]
    #[channel_types("Voice")]
    voice_channel: serenity::GuildChannel,
    #[description = "Role a ser mencionada"] role: serenity::Role,
) -> Result<(), Error> {
    if !ensure_authorized(ctx).await? {
        return Ok(());
    }

    if voice_channel.kind != serenity::ChannelType::Voice {
        send(ctx, "Por favor, selecione um canal de voz.").await?;
        return Ok(());
    }

    let Some(guild_id) = ctx.guild_id() else {
        send(ctx, "Use este comando em um servidor.").await?;
        return Ok(());
    };

    if role_is_everyone(role.id, guild_id) {
        send(ctx, "A role @everyone não é permitida.").await?;
        return Ok(());
    }

    ctx.defer_ephemeral().await?;

    match ctx
        .data()
        .upsert_call(ctx.serenity_context(), voice_channel.id, role.id)
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

async fn send(ctx: Context<'_>, content: impl Into<String>) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default()
            .content(content)
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

fn role_is_everyone(role_id: serenity::RoleId, guild_id: serenity::GuildId) -> bool {
    role_id == guild_id.everyone_role()
}

#[cfg(test)]
mod tests {
    use poise::serenity_prelude as serenity;

    use super::role_is_everyone;

    #[test]
    fn rejects_the_everyone_role() {
        let guild_id = serenity::GuildId::new(42);

        assert!(role_is_everyone(guild_id.everyone_role(), guild_id));
        assert!(!role_is_everyone(serenity::RoleId::new(7), guild_id));
    }
}
