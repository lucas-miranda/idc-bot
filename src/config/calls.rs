use poise::serenity_prelude as serenity;

/// One registerable call: a voice channel, the role to ping and the message to
/// send when the call opens.
#[derive(Clone, Debug)]
pub struct CallConfig {
    pub voice_channel: serenity::ChannelId,
    pub role_id: serenity::RoleId,
    pub message: String,
}
