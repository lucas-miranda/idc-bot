use enum_display::EnumDisplay;
use serde::Deserialize;

/// The kind of voice call the bot announces.
///
/// Each kind maps to a voice channel (or group of channels), a role to ping and
/// a message template; see [`crate::config::CallKindConfig`].
#[derive(Hash, PartialEq, Eq, Clone, Debug, EnumDisplay, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CallKind {
    Social,
    Games,
    Movies,
}
