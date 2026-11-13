use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::Duration,
};

use poise::serenity_prelude as serenity;
use serde::Deserialize;

use crate::{domain::CallKind, error::ConfigError};

pub mod call_kinds;
pub use call_kinds::CallKindConfig;

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
    pub call_kinds: Vec<CallKindConfig>,
    pub cooldown: Duration,
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

    /// Find which call kind owns a voice channel.
    pub fn kind_for_channel(&self, channel_id: serenity::ChannelId) -> Option<&CallKindConfig> {
        self.call_kinds
            .iter()
            .find(|call_kind| call_kind.voice_channels.contains(&channel_id))
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
    call_kinds: Vec<RawCallKind>,
}

#[derive(Deserialize)]
struct RawCallKind {
    kind: CallKind,
    voice_channels: Vec<u64>,
    role_id: u64,
    message: String,
}

impl TryFrom<RawConfig> for Config {
    type Error = ConfigError;

    fn try_from(raw: RawConfig) -> Result<Self, ConfigError> {
        ensure_non_zero(raw.guild_id, "guild_id")?;
        ensure_non_zero(raw.broadcast_channel_id, "broadcast_channel_id")?;

        if raw.cooldown_secs == 0 {
            return Err(ConfigError::Invalid(
                "`cooldown_secs` must be greater than 0".to_owned(),
            ));
        }

        if raw.call_kinds.is_empty() {
            return Err(ConfigError::Invalid(
                "at least one `[[call_kinds]]` entry is required".to_owned(),
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

        let mut seen_kinds = HashSet::new();
        let mut seen_channels = HashSet::new();
        let mut call_kinds = Vec::with_capacity(raw.call_kinds.len());

        for raw_kind in raw.call_kinds {
            if !seen_kinds.insert(raw_kind.kind.clone()) {
                return Err(ConfigError::Invalid(format!(
                    "duplicate call kind `{}`",
                    raw_kind.kind
                )));
            }

            ensure_non_zero(raw_kind.role_id, "call_kinds.role_id")?;

            if raw_kind.voice_channels.is_empty() {
                return Err(ConfigError::Invalid(format!(
                    "call kind `{}` needs at least one voice channel",
                    raw_kind.kind
                )));
            }

            let mut voice_channels = Vec::with_capacity(raw_kind.voice_channels.len());
            for id in raw_kind.voice_channels {
                ensure_non_zero(id, "call_kinds.voice_channels")?;

                if !seen_channels.insert(id) {
                    return Err(ConfigError::Invalid(format!(
                        "voice channel `{id}` is assigned to more than one call kind"
                    )));
                }

                voice_channels.push(serenity::ChannelId::new(id));
            }

            call_kinds.push(CallKindConfig {
                kind: raw_kind.kind,
                voice_channels,
                role_id: serenity::RoleId::new(raw_kind.role_id),
                message: raw_kind.message,
            });
        }

        Ok(Config {
            guild_id: serenity::GuildId::new(raw.guild_id),
            broadcast_channel_id: serenity::ChannelId::new(raw.broadcast_channel_id),
            ignored_voice_channels,
            reload_roles,
            call_kinds,
            cooldown: Duration::from_secs(raw.cooldown_secs),
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

    use crate::{domain::CallKind, error::ConfigError};

    use super::Config;

    const VALID: &str = r#"
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600

[[call_kinds]]
kind = "social"
voice_channels = [3]
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

        let call_kind = &config.call_kinds[0];
        assert_eq!(call_kind.kind, CallKind::Social);
        assert_eq!(call_kind.voice_channels[0].get(), 3);
        assert_eq!(call_kind.role_id.get(), 4);
        assert_eq!(call_kind.message, "hi");
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
    fn rejects_duplicate_call_kinds() {
        let toml = r#"
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600

[[call_kinds]]
kind = "social"
voice_channels = [3]
role_id = 4
message = "hi"

[[call_kinds]]
kind = "social"
voice_channels = [5]
role_id = 6
message = "hi"
"#;

        let error = Config::from_toml_str(toml).expect_err("should fail");
        assert!(matches!(error, ConfigError::Invalid(_)));
    }

    #[test]
    fn rejects_duplicate_voice_channels() {
        let toml = r#"
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600

[[call_kinds]]
kind = "social"
voice_channels = [3]
role_id = 4
message = "hi"

[[call_kinds]]
kind = "games"
voice_channels = [3]
role_id = 5
message = "hi"
"#;

        let error = Config::from_toml_str(toml).expect_err("should fail");
        assert!(matches!(error, ConfigError::Invalid(_)));
    }

    #[test]
    fn rejects_an_empty_call_kind_list() {
        let toml = r#"
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600
call_kinds = []
"#;

        let error = Config::from_toml_str(toml).expect_err("should fail");
        assert!(matches!(error, ConfigError::Invalid(_)));
    }
}
