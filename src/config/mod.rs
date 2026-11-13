use std::time::Duration;

use poise::serenity_prelude as serenity;

use crate::domain::CallKind;

pub mod call_kinds;
pub use call_kinds::CallKindConfig;

/// Static configuration for the bot.
///
/// All ids live here now, so wiring a new channel or role is a single edit
/// instead of a hunt through the codebase.
#[derive(Clone, Debug)]
pub struct Config {
    pub guild_id: serenity::GuildId,
    pub broadcast_channel_id: serenity::ChannelId,
    pub ignored_voice_channels: Vec<serenity::ChannelId>,
    pub call_kinds: Vec<CallKindConfig>,
    pub cooldown: Duration,
}

impl Config {
    /// Find which call kind owns a voice channel.
    pub fn kind_for_channel(&self, channel_id: serenity::ChannelId) -> Option<&CallKindConfig> {
        self.call_kinds
            .iter()
            .find(|call_kind| call_kind.voice_channels.contains(&channel_id))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // guild id is hardcoded
            guild_id: serenity::GuildId::new(1067493141038501939),
            // #chat-idc
            broadcast_channel_id: serenity::ChannelId::new(1126997072961343660),
            ignored_voice_channels: vec![
                serenity::ChannelId::new(1162777377500303520),
                serenity::ChannelId::new(1287448364689916008),
                serenity::ChannelId::new(1072695360629260392),
            ],
            call_kinds: vec![
                CallKindConfig {
                    kind: CallKind::Social,
                    voice_channels: vec![serenity::ChannelId::new(1160706169963294740)],
                    role_id: serenity::RoleId::new(1274386535285788826),
                    message: " call aberta! ",
                },
                CallKindConfig {
                    kind: CallKind::Games,
                    voice_channels: vec![serenity::ChannelId::new(1374542425116246066)],
                    role_id: serenity::RoleId::new(1287419235886436375),
                    message: " call aberta! ",
                },
                CallKindConfig {
                    kind: CallKind::Movies,
                    voice_channels: vec![serenity::ChannelId::new(1419472150644916244)],
                    role_id: serenity::RoleId::new(1287419731447648396),
                    message: " call aberta! ",
                },
            ],
            cooldown: Duration::from_secs(60 * 60),
        }
    }
}
