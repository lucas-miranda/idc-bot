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
            if let Err(e) = data.vc_manager.handle_state(ctx, event).await {
                println!("An error occurred:\n{}", e)
            }
        },
        _ => {}
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged()
                | serenity::GatewayIntents::GUILDS
                | serenity::GatewayIntents::GUILD_VOICE_STATES;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let vc_manager = VoiceChannelManager::new(
                        ctx,
                        vec![
                            From::from(1162777377500303520),

                            From::from(1287448364689916008),
                            From::from(1072695360629260392),
                        ]
                    )
                    .await?;

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
