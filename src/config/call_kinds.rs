use crate::domain::CallKind;

/// Everything about a call kind that used to be scattered across the manager:
/// where it happens, who to ping and what to say.
#[derive(Clone, Debug)]
pub struct CallKindConfig {
    pub kind: CallKind,
    pub voice_channels: Vec<poise::serenity_prelude::ChannelId>,
    pub role_id: poise::serenity_prelude::RoleId,
    pub message: &'static str,
}
