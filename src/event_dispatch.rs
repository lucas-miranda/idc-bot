use poise::FrameworkContext;
use poise::serenity_prelude as serenity;
use tracing::{error, info};

use crate::error::Error;
use crate::state::AppState;

/// Thin routing from serenity events to use cases.
pub async fn handle_event<'a>(
    framework: FrameworkContext<'a, AppState, Error>,
    event: &'a serenity::FullEvent,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            info!(user = %data_about_bot.user.name, "logged in");
        }
        serenity::FullEvent::VoiceStateUpdate { old, new } => {
            let voice = framework.user_data.voice.read().await.clone();

            if let Err(error) = voice.handle_transition(old.as_ref(), new).await {
                error!(%error, "failed to handle voice state update");
            }
        }
        _ => {}
    }

    Ok(())
}
