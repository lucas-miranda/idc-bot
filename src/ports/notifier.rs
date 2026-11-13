use poise::serenity_prelude as serenity;

use crate::Error;

/// Sends (and rate-limits) call announcements.
pub trait Notifier: Send + Sync {
    fn notify(
        &self,
        channel_id: serenity::ChannelId,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}
