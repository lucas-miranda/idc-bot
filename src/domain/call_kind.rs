use enum_display::EnumDisplay;

/// The kind of voice call the bot announces.
///
/// Each kind maps to a voice channel (or group of channels), a role to ping and
/// a message template; see [`crate::config::CallKindConfig`].
#[derive(Hash, PartialEq, Eq, Clone, Debug, EnumDisplay)]
pub enum CallKind {
    Social,
    Games,
    Movies,
}
