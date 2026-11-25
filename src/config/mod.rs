use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::Duration,
};

use poise::serenity_prelude as serenity;
use serde::Deserialize;

use crate::error::ConfigError;

mod calls;
pub use calls::CallConfig;

mod mirror;
pub use mirror::MirrorConfig;

pub mod edit;
pub use edit::DEFAULT_CALL_MESSAGE;

/// Where the bot looks for its configuration when `CONFIG_PATH` is unset.
pub const DEFAULT_CONFIG_PATH: &str = "/data/config.toml";

/// Resolve the configuration path from `CONFIG_PATH`, falling back to
/// [`DEFAULT_CONFIG_PATH`].
pub fn path_from_env() -> PathBuf {
    std::env::var_os("CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH))
}

/// Validated, runtime configuration.
///
/// There is intentionally no `Default`: every id comes from the TOML file.
#[derive(Clone, Debug)]
pub struct Config {
    pub guild_id: serenity::GuildId,
    pub broadcast_channel_id: serenity::ChannelId,
    pub ignored_voice_channels: Vec<serenity::ChannelId>,
    pub reload_roles: Vec<serenity::RoleId>,
    pub call_kinds: Vec<CallConfig>,
    pub cooldown: Duration,
    pub mirror: Option<MirrorConfig>,
}

impl Config {
    /// Read and validate the TOML file at `path`.
    pub async fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = tokio::fs::read_to_string(path)
            .await
            .map_err(|source| ConfigError::Read {
                path: path.to_path_buf(),
                source,
            })?;

        Self::from_toml_str(&text)
    }

    /// Parse and validate a TOML document.
    pub fn from_toml_str(text: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(text)?;
        raw.try_into()
    }

    /// Find the call registered for a voice channel.
    pub fn call_for_channel(&self, channel_id: serenity::ChannelId) -> Option<&CallConfig> {
        self.call_kinds
            .iter()
            .find(|call| call.voice_channel == channel_id)
    }
}

#[derive(Deserialize)]
struct RawConfig {
    guild_id: u64,
    broadcast_channel_id: u64,
    #[serde(default)]
    ignored_voice_channels: Vec<u64>,
    #[serde(default)]
    reload_roles: Vec<u64>,
    cooldown_secs: u64,
    #[serde(default)]
    call_kinds: Vec<RawCall>,
    #[serde(default)]
    mirror: Option<RawMirror>,
}

#[derive(Deserialize)]
struct RawMirror {
    source_channel: u64,
    source_bot: u64,
    #[serde(default, alias = "embed_title")]
    message_contains: Option<String>,
}

#[derive(Deserialize)]
struct RawCall {
    voice_channel: u64,
    role_id: u64,
    message: String,
}

impl TryFrom<RawConfig> for Config {
    type Error = ConfigError;

    fn try_from(raw: RawConfig) -> Result<Self, ConfigError> {
        ensure_non_zero(raw.guild_id, "guild_id")?;
        ensure_non_zero(raw.broadcast_channel_id, "broadcast_channel_id")?;

        let guild_id = raw.guild_id;

        if raw.cooldown_secs == 0 {
            return Err(ConfigError::Invalid(
                "`cooldown_secs` must be greater than 0".to_owned(),
            ));
        }

        let mut ignored_voice_channels = Vec::with_capacity(raw.ignored_voice_channels.len());
        for id in raw.ignored_voice_channels {
            ensure_non_zero(id, "ignored_voice_channels")?;
            ignored_voice_channels.push(serenity::ChannelId::new(id));
        }

        let mut reload_roles = Vec::with_capacity(raw.reload_roles.len());
        for id in raw.reload_roles {
            ensure_non_zero(id, "reload_roles")?;
            reload_roles.push(serenity::RoleId::new(id));
        }

        let mut seen_channels = HashSet::new();
        let mut call_kinds = Vec::with_capacity(raw.call_kinds.len());

        for raw_call in raw.call_kinds {
            ensure_non_zero(raw_call.voice_channel, "call_kinds.voice_channel")?;
            ensure_non_zero(raw_call.role_id, "call_kinds.role_id")?;

            if raw_call.role_id == guild_id {
                return Err(ConfigError::Invalid(
                    "a call role must not be @everyone".to_owned(),
                ));
            }

            if !seen_channels.insert(raw_call.voice_channel) {
                return Err(ConfigError::Invalid(format!(
                    "voice channel `{}` is registered more than once",
                    raw_call.voice_channel
                )));
            }

            call_kinds.push(CallConfig {
                voice_channel: serenity::ChannelId::new(raw_call.voice_channel),
                role_id: serenity::RoleId::new(raw_call.role_id),
                message: raw_call.message,
            });
        }

        let mirror = match raw.mirror {
            Some(raw_mirror) => {
                ensure_non_zero(raw_mirror.source_channel, "mirror.source_channel")?;
                ensure_non_zero(raw_mirror.source_bot, "mirror.source_bot")?;

                let message_contains = match raw_mirror.message_contains {
                    Some(text) => {
                        let text = text.trim().to_owned();

                        if text.is_empty() {
                            return Err(ConfigError::Invalid(
                                "`mirror.message_contains` must not be empty".to_owned(),
                            ));
                        }

                        Some(text)
                    }
                    None => None,
                };

                Some(MirrorConfig {
                    source_channel: serenity::ChannelId::new(raw_mirror.source_channel),
                    source_bot: serenity::UserId::new(raw_mirror.source_bot),
                    message_contains,
                })
            }
            None => None,
        };

        Ok(Config {
            guild_id: serenity::GuildId::new(guild_id),
            broadcast_channel_id: serenity::ChannelId::new(raw.broadcast_channel_id),
            ignored_voice_channels,
            reload_roles,
            call_kinds,
            cooldown: Duration::from_secs(raw.cooldown_secs),
            mirror,
        })
    }
}

fn ensure_non_zero(value: u64, field: &str) -> Result<(), ConfigError> {
    if value == 0 {
        Err(ConfigError::Invalid(format!("`{field}` must not be 0")))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::error::ConfigError;

    use super::Config;

    const VALID: &str = r#"
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600

[[call_kinds]]
voice_channel = 3
role_id = 4
message = "hi"
"#;

    #[test]
    fn parses_a_valid_config() {
        let config = Config::from_toml_str(VALID).expect("config should parse");

        assert_eq!(config.guild_id.get(), 1);
        assert_eq!(config.broadcast_channel_id.get(), 2);
        assert_eq!(config.cooldown, Duration::from_secs(3600));
        assert_eq!(config.call_kinds.len(), 1);

        let call = &config.call_kinds[0];
        assert_eq!(call.voice_channel.get(), 3);
        assert_eq!(call.role_id.get(), 4);
        assert_eq!(call.message, "hi");
        assert!(config.mirror.is_none());
    }

    #[test]
    fn accepts_an_empty_call_list() {
        let toml = r#"
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600
"#;

        let config = Config::from_toml_str(toml).expect("config should parse");
        assert!(config.call_kinds.is_empty());
    }

    #[test]
    fn optional_lists_default_to_empty() {
        let config = Config::from_toml_str(VALID).expect("config should parse");

        assert!(config.ignored_voice_channels.is_empty());
        assert!(config.reload_roles.is_empty());
    }

    #[test]
    fn rejects_a_missing_required_field() {
        let error = Config::from_toml_str("broadcast_channel_id = 2").expect_err("should fail");

        assert!(matches!(error, ConfigError::Parse(_)));
    }

    #[test]
    fn rejects_zero_ids() {
        let error = Config::from_toml_str(&VALID.replace("guild_id = 1", "guild_id = 0"))
            .expect_err("should fail");

        assert!(matches!(error, ConfigError::Invalid(_)));
    }

    #[test]
    fn rejects_duplicate_voice_channels() {
        let toml = r#"
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600

[[call_kinds]]
voice_channel = 3
role_id = 4
message = "hi"

[[call_kinds]]
voice_channel = 3
role_id = 5
message = "hi"
"#;

        let error = Config::from_toml_str(toml).expect_err("should fail");
        assert!(matches!(error, ConfigError::Invalid(_)));
    }

    #[test]
    fn rejects_the_everyone_role() {
        let toml = r#"
guild_id = 42
broadcast_channel_id = 2
cooldown_secs = 3600

[[call_kinds]]
voice_channel = 3
role_id = 42
message = "hi"
"#;

        let error = Config::from_toml_str(toml).expect_err("should fail");
        assert!(matches!(error, ConfigError::Invalid(_)));
    }

    #[test]
    fn parses_a_mirror_config() {
        let toml = r#"
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600

[mirror]
source_channel = 5
source_bot = 6
"#;

        let config = Config::from_toml_str(toml).expect("config should parse");
        let mirror = config.mirror.expect("mirror should be present");

        assert_eq!(mirror.source_channel.get(), 5);
        assert_eq!(mirror.source_bot.get(), 6);
        assert!(mirror.message_contains.is_none());
    }

    #[test]
    fn parses_a_mirror_message_contains_filter() {
        let toml = r#"
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600

[mirror]
source_channel = 5
source_bot = 6
message_contains = "  Started playing  "
"#;

        let config = Config::from_toml_str(toml).expect("config should parse");
        let mirror = config.mirror.expect("mirror should be present");

        assert_eq!(mirror.message_contains.as_deref(), Some("Started playing"));
    }

    #[test]
    fn accepts_the_legacy_embed_title_key() {
        let toml = r#"
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600

[mirror]
source_channel = 5
source_bot = 6
embed_title = "Started playing"
"#;

        let config = Config::from_toml_str(toml).expect("config should parse");
        let mirror = config.mirror.expect("mirror should be present");

        assert_eq!(mirror.message_contains.as_deref(), Some("Started playing"));
    }

    #[test]
    fn rejects_an_empty_mirror_message_contains() {
        let toml = r#"
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600

[mirror]
source_channel = 5
source_bot = 6
message_contains = "   "
"#;

        let error = Config::from_toml_str(toml).expect_err("should fail");
        assert!(matches!(error, ConfigError::Invalid(_)));
    }

    #[test]
    fn rejects_zero_mirror_ids() {
        let toml = r#"
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600

[mirror]
source_channel = 0
source_bot = 6
"#;

        let error = Config::from_toml_str(toml).expect_err("should fail");
        assert!(matches!(error, ConfigError::Invalid(_)));
    }
}
