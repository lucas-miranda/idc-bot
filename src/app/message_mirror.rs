use std::sync::Arc;

use poise::serenity_prelude as serenity;
use tracing::{debug, warn};

use crate::config::Config;
use crate::domain::overwrites;
use crate::error::Error;
use crate::ports::{EditOutcome, MessageSink};

/// Flip to `false` to mirror the source bot's content without a name prefix.
const PREFIX_SOURCE_BOT: bool = true;

#[derive(Clone, Debug, PartialEq, Eq)]
struct MirroredMessage {
    source_channel: serenity::ChannelId,
    source_message: serenity::MessageId,
    target_channel: serenity::ChannelId,
    target_message: serenity::MessageId,
}

/// Remembers the last mirrored pair so it can be replaced or cleaned up.
#[derive(Default)]
pub struct MirrorTracker {
    last: tokio::sync::Mutex<Option<MirroredMessage>>,
}

impl MirrorTracker {
    async fn snapshot(&self) -> Option<MirroredMessage> {
        self.last.lock().await.clone()
    }

    async fn set(&self, message: MirroredMessage) {
        *self.last.lock().await = Some(message);
    }

    async fn clear(&self) {
        *self.last.lock().await = None;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UpdateOutcome {
    Edited,
    Resent(serenity::MessageId),
}

/// Copies a bot's messages into the text chat of the voice channel it is in.
pub struct MessageMirror<S: MessageSink> {
    config: Arc<Config>,
    sink: S,
    tracker: Arc<MirrorTracker>,
}

impl<S: MessageSink> MessageMirror<S> {
    pub fn new(config: Arc<Config>, sink: S, tracker: Arc<MirrorTracker>) -> Self {
        Self {
            config,
            sink,
            tracker,
        }
    }

    pub async fn mirror(
        &self,
        message: &serenity::Message,
        bot_id: serenity::UserId,
    ) -> Result<(), Error> {
        let Some(mirror) = &self.config.mirror else {
            return Ok(());
        };

        if message.channel_id != mirror.source_channel
            || message.author.id != mirror.source_bot
            || message.author.id == bot_id
            || !message.author.bot
        {
            return Ok(());
        }

        if message.content.trim().is_empty() && message.embeds.is_empty() {
            return Ok(());
        }

        let content = mirrored_content(&message.author, &message.content);
        let embeds = message.embeds.clone();
        let target_id = self.current_target().await;

        if let Some(previous) = self.tracker.snapshot().await {
            match self
                .sink
                .fetch_message(previous.source_channel, previous.source_message)
                .await
            {
                Ok(None) => {
                    if target_id == Some(previous.target_channel) {
                        match self.update_mirrored(&previous, &content, &embeds).await {
                            Ok(UpdateOutcome::Edited) => {
                                self.tracker
                                    .set(MirroredMessage {
                                        source_channel: message.channel_id,
                                        source_message: message.id,
                                        target_channel: previous.target_channel,
                                        target_message: previous.target_message,
                                    })
                                    .await;
                                return Ok(());
                            }
                            Ok(UpdateOutcome::Resent(new_target)) => {
                                self.tracker
                                    .set(MirroredMessage {
                                        source_channel: message.channel_id,
                                        source_message: message.id,
                                        target_channel: previous.target_channel,
                                        target_message: new_target,
                                    })
                                    .await;
                                return Ok(());
                            }
                            Err(error) => {
                                warn!(%error, "failed to update the previous mirror");
                                return Ok(());
                            }
                        }
                    }

                    // The call moved (or ended): drop the tracker without
                    // deleting the old copy, as requested.
                    self.tracker.clear().await;
                }
                Ok(Some(_)) => {}
                Err(error) => {
                    warn!(%error, "failed to check the previous source message");
                }
            }
        }

        let Some(target_id) = target_id else {
            return Ok(());
        };

        match self.sink.send(target_id, &content, &embeds).await {
            Ok(target_message) => {
                self.tracker
                    .set(MirroredMessage {
                        source_channel: message.channel_id,
                        source_message: message.id,
                        target_channel: target_id,
                        target_message,
                    })
                    .await;
            }
            Err(error) => {
                warn!(%error, channel_id = %target_id, "failed to mirror message");
            }
        }

        Ok(())
    }

    pub async fn mirror_edit(
        &self,
        event: &serenity::MessageUpdateEvent,
        new: Option<&serenity::Message>,
        bot_id: serenity::UserId,
    ) -> Result<(), Error> {
        let Some(mirror) = &self.config.mirror else {
            return Ok(());
        };

        if event.channel_id != mirror.source_channel {
            return Ok(());
        }

        let Some(previous) = self.tracker.snapshot().await else {
            return Ok(());
        };

        if previous.source_channel != event.channel_id || previous.source_message != event.id {
            return Ok(());
        }

        let message = match new {
            Some(message) => message.clone(),
            None => match self.sink.fetch_message(event.channel_id, event.id).await? {
                Some(message) => message,
                None => return Ok(()),
            },
        };

        if message.author.id != mirror.source_bot
            || message.author.id == bot_id
            || !message.author.bot
        {
            return Ok(());
        }

        if message.content.trim().is_empty() && message.embeds.is_empty() {
            return Ok(());
        }

        let content = mirrored_content(&message.author, &message.content);
        let embeds = message.embeds.clone();

        match self.update_mirrored(&previous, &content, &embeds).await {
            Ok(UpdateOutcome::Edited) => {}
            Ok(UpdateOutcome::Resent(new_target)) => {
                self.tracker
                    .set(MirroredMessage {
                        source_channel: previous.source_channel,
                        source_message: previous.source_message,
                        target_channel: previous.target_channel,
                        target_message: new_target,
                    })
                    .await;
            }
            Err(error) => {
                warn!(%error, "failed to edit the mirrored message");
            }
        }

        Ok(())
    }

    /// The source bot's current voice channel, if it is a valid mirror target.
    async fn current_target(&self) -> Option<serenity::ChannelId> {
        let mirror = self.config.mirror.as_ref()?;

        let Some(target_id) = self
            .sink
            .voice_channel_of(self.config.guild_id, mirror.source_bot)
        else {
            debug!(
                bot_id = %mirror.source_bot,
                "source bot is not in a voice channel; skipping mirror"
            );
            return None;
        };

        if self.config.call_for_channel(target_id).is_none()
            || self.config.ignored_voice_channels.contains(&target_id)
        {
            debug!(
                channel_id = %target_id,
                "target channel is not a managed call; skipping mirror"
            );
            return None;
        }

        let channel = self
            .sink
            .fetch_channel(self.config.guild_id, target_id)
            .await?;

        if channel.kind != serenity::ChannelType::Voice {
            debug!(
                channel_id = %target_id,
                "target channel is not a voice channel; skipping mirror"
            );
            return None;
        }

        if !overwrites::allows_view(&channel.permission_overwrites, channel.guild_id) {
            debug!(
                channel_id = %target_id,
                "target voice channel is not visible; skipping mirror"
            );
            return None;
        }

        Some(target_id)
    }

    /// Edit the mirrored copy when it is the latest message, otherwise resend
    /// it at the bottom and remove the old copy.
    async fn update_mirrored(
        &self,
        previous: &MirroredMessage,
        content: &str,
        embeds: &[serenity::Embed],
    ) -> Result<UpdateOutcome, Error> {
        let is_last = match self.sink.latest_message_id(previous.target_channel).await {
            Ok(latest) => latest == Some(previous.target_message),
            Err(error) => {
                warn!(
                    %error,
                    "failed to check the latest message; resending the mirror"
                );
                false
            }
        };

        if is_last {
            return match self
                .sink
                .edit_message(
                    previous.target_channel,
                    previous.target_message,
                    content,
                    embeds,
                )
                .await?
            {
                EditOutcome::Edited => Ok(UpdateOutcome::Edited),
                EditOutcome::NotFound => {
                    let new_target = self
                        .sink
                        .send(previous.target_channel, content, embeds)
                        .await?;

                    Ok(UpdateOutcome::Resent(new_target))
                }
            };
        }

        let new_target = self
            .sink
            .send(previous.target_channel, content, embeds)
            .await?;

        if let Err(error) = self
            .sink
            .delete_message(previous.target_channel, previous.target_message)
            .await
        {
            warn!(%error, "failed to delete the previous mirrored message");
        }

        Ok(UpdateOutcome::Resent(new_target))
    }
}

fn mirrored_content(author: &serenity::User, content: &str) -> String {
    prefixed_content(author.display_name(), content)
}

fn prefixed_content(author_name: &str, content: &str) -> String {
    if PREFIX_SOURCE_BOT && !content.is_empty() {
        format!("**{author_name}:** {content}")
    } else {
        content.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use poise::serenity_prelude as serenity;

    use super::{MirrorTracker, MirroredMessage, PREFIX_SOURCE_BOT, prefixed_content};

    #[test]
    fn prefixes_non_empty_content() {
        let result = prefixed_content("CallBot", "hello");

        if PREFIX_SOURCE_BOT {
            assert_eq!(result, "**CallBot:** hello");
        } else {
            assert_eq!(result, "hello");
        }
    }

    #[test]
    fn does_not_prefix_empty_content() {
        assert_eq!(prefixed_content("CallBot", ""), "");
    }

    #[tokio::test]
    async fn tracker_stores_and_clears_the_last_pair() {
        let tracker = MirrorTracker::default();
        assert!(tracker.snapshot().await.is_none());

        let pair = MirroredMessage {
            source_channel: serenity::ChannelId::new(1),
            source_message: serenity::MessageId::new(2),
            target_channel: serenity::ChannelId::new(3),
            target_message: serenity::MessageId::new(4),
        };

        tracker.set(pair.clone()).await;
        assert_eq!(tracker.snapshot().await, Some(pair));

        tracker.clear().await;
        assert!(tracker.snapshot().await.is_none());
    }
}
