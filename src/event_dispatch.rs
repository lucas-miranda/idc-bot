use poise::FrameworkContext;
use poise::serenity_prelude as serenity;

use crate::bootstrap::AppState;
use crate::error::Error;

/// Thin routing from serenity events to use cases.
pub async fn handle_event<'a>(
    _ctx: &'a serenity::Context,
    event: &'a serenity::FullEvent,
    _framework: FrameworkContext<'a, AppState, Error>,
    data: &'a AppState,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            println!("Logged in as {}", data_about_bot.user.name);
        }
        serenity::FullEvent::VoiceStateUpdate { old, new } => {
            if let Err(e) = data.voice.handle_transition(old.as_ref(), new).await {
                println!("An error occurred:\n{}", e);
            }
        }
        _ => {}
    }

    Ok(())
}
