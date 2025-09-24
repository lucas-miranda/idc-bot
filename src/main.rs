use poise::{serenity_prelude::{self as serenity}, FrameworkContext};

pub mod voice;
pub mod permissions;
pub mod channel;

use voice::VoiceChannelManager;

#[allow(unused)]
type Error = Box<dyn std::error::Error + Send + Sync>;
#[allow(unused)]
type Context<'a> = poise::Context<'a, Data, Error>;

struct Data {
    vc_manager: VoiceChannelManager,
}

async fn event_handler<'a>(
    ctx: &'a serenity::Context,
    event: &'a serenity::FullEvent,
    _framework: FrameworkContext<'a, Data, Error>,
    data: &'a Data
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            println!("Logged in as {}", data_about_bot.user.name);
        },
        serenity::FullEvent::VoiceStateUpdate { .. } => {
            data.vc_manager.handle_state(ctx, event).await?;
        },
        /*
        serenity::FullEvent::Message { new_message } => {
            if new_message.content.to_lowercase().contains("poise")
                && new_message.author.id != ctx.cache.current_user().id
            {
                let old_mentions = data.mentions.fetch_add(1, Ordering::SeqCst);
                new_message
                    .reply(
                        ctx,
                        format!("Poise has been mentioned {} times", old_mentions + 1),
                    )
                    .await?;
            }
        }
        */
        _ => {}
    }

    Ok(())
}

/*
/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}
*/

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged()
                | serenity::GatewayIntents::GUILDS
                | serenity::GatewayIntents::GUILD_VOICE_STATES;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            //commands: vec![age()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let mut vc_manager: VoiceChannelManager = Default::default();

                vc_manager.ignore_voice_channels.extend::<[serenity::ChannelId; _]>([
                    From::from(1162777377500303520),

                    From::from(1287448364689916008),
                    From::from(1072695360629260392),
                ]);

                Ok(Data {
                    vc_manager,
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
