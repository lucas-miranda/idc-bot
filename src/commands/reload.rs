use tracing::error;

use crate::commands::{Context, ensure_authorized};
use crate::error::Error;

/// Recarrega a configuração do bot a partir do arquivo TOML.
#[poise::command(slash_command, guild_only)]
pub async fn reload(ctx: Context<'_>) -> Result<(), Error> {
    if !ensure_authorized(ctx).await? {
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
