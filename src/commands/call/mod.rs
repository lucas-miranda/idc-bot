mod listar;
mod registrar;
mod remover;

pub use listar::listar;
pub use registrar::registrar;
pub use remover::remover;

use crate::{commands::Context, error::Error};

/// Gerencia as calls registradas.
#[poise::command(
    slash_command,
    guild_only,
    subcommands("registrar", "remover", "listar")
)]
pub async fn call(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use poise::serenity_prelude as serenity;

    #[test]
    fn exposes_the_expected_subcommands() {
        let command = crate::commands::call();

        assert_eq!(command.name.as_ref(), "call");

        let names: Vec<&str> = command
            .subcommands
            .iter()
            .map(|subcommand| subcommand.name.as_ref())
            .collect();

        assert_eq!(names, ["registrar", "remover", "listar"]);
    }

    #[test]
    fn registrar_only_accepts_voice_channels() {
        let command = crate::commands::call();
        let registrar = command
            .subcommands
            .iter()
            .find(|subcommand| subcommand.name == "registrar")
            .expect("registrar subcommand should exist");
        let voice_channel = registrar
            .parameters
            .iter()
            .find(|parameter| parameter.name == "voice_channel")
            .expect("voice_channel parameter should exist");

        assert_eq!(
            voice_channel.channel_types.as_deref(),
            Some(&[serenity::ChannelType::Voice][..])
        );
    }

    #[test]
    fn remover_uses_autocomplete() {
        let command = crate::commands::call();
        let remover = command
            .subcommands
            .iter()
            .find(|subcommand| subcommand.name == "remover")
            .expect("remover subcommand should exist");
        let call = remover
            .parameters
            .iter()
            .find(|parameter| parameter.name == "call")
            .expect("call parameter should exist");

        assert!(call.autocomplete_callback.is_some());
    }
}
