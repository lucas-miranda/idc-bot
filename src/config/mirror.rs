use poise::serenity_prelude as serenity;

/// Configuration for mirroring another bot's messages.
#[derive(Clone, Debug)]
pub struct MirrorConfig {
    /// Text channel to listen on.
    pub source_channel: serenity::ChannelId,
    /// Only messages from this bot are mirrored.
    pub source_bot: serenity::UserId,
    /// When set, only messages whose content contains this text
    /// (case-insensitive) are mirrored.
    pub message_contains: Option<String>,
}
