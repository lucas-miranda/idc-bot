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
    /// `None` for entries recovered after a restart.
    source_message: Option<serenity::MessageId>,
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

        if !message_contains(message, mirror.message_contains.as_deref()) {
            debug!(
                message_id = %message.id,
                author_id = %message.author.id,
                content = %message.content,
                embed_titles = ?message.embeds.iter().filter_map(|embed| embed.title.as_deref()).collect::<Vec<_>>(),
                embed_descriptions = ?message.embeds.iter().filter_map(|embed| embed.description.as_deref()).collect::<Vec<_>>(),
                attachments = message.attachments.len(),
                attachment_names = ?message.attachments.iter().map(|attachment| attachment.filename.as_str()).collect::<Vec<_>>(),
                components = message.components.len(),
                sticker_items = message.sticker_items.len(),
                has_poll = message.poll.is_some(),
                "message did not match the mirror filter"
            );
            return Ok(());
        }

        let content = mirrored_content(&message.author, &message.content);
        let embeds = message.embeds.clone();
        let target_id = self.current_target().await;

        // After a restart the tracker is empty: adopt the last mirrored
        // message in the target chat, if it is ours, instead of sending a
        // fresh one.
        let mut previous = self.tracker.snapshot().await;
        if previous.is_none()
            && let Some(target_id) = target_id
        {
            previous = self.recover_from_target(target_id, bot_id).await;
        }

        if let Some(previous) = previous {
            let source_gone = match previous.source_message {
                Some(source_message) => {
                    match self
                        .sink
                        .fetch_message(previous.source_channel, source_message)
                        .await
                    {
                        Ok(None) => true,
                        Ok(Some(_)) => false,
                        Err(error) => {
                            warn!(%error, "failed to check the previous source message");
                            false
                        }
                    }
                }
                // Recovered entries do not know their source message.
                None => true,
            };

            if source_gone {
                if target_id == Some(previous.target_channel) {
                    match self.update_mirrored(&previous, &content, &embeds).await {
                        Ok(UpdateOutcome::Edited) => {
                            self.tracker
                                .set(MirroredMessage {
                                    source_channel: message.channel_id,
                                    source_message: Some(message.id),
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
                                    source_message: Some(message.id),
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
        }

        let Some(target_id) = target_id else {
            return Ok(());
        };

        match self.sink.send(target_id, &content, &embeds).await {
            Ok(target_message) => {
                self.tracker
                    .set(MirroredMessage {
                        source_channel: message.channel_id,
                        source_message: Some(message.id),
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

        if let Some(author) = &event.author
            && author.id != mirror.source_bot
        {
            return Ok(());
        }

        // Resolve before the tracker check so source-bot updates are logged
        // even when nothing is tracked yet.
        let message = match new {
            Some(message) => Some(message.clone()),
            None => self.sink.fetch_message(event.channel_id, event.id).await?,
        };

        if let Some(message) = &message
            && message.author.id == mirror.source_bot
        {
            debug!(
                message_id = %message.id,
                content = %message.content,
                embed_titles = ?message.embeds.iter().filter_map(|embed| embed.title.as_deref()).collect::<Vec<_>>(),
                embed_descriptions = ?message.embeds.iter().filter_map(|embed| embed.description.as_deref()).collect::<Vec<_>>(),
                attachments = message.attachments.len(),
                attachment_names = ?message.attachments.iter().map(|attachment| attachment.filename.as_str()).collect::<Vec<_>>(),
                components = message.components.len(),
                sticker_items = message.sticker_items.len(),
                has_poll = message.poll.is_some(),
                "received a source-bot message update"
            );
        }

        let Some(previous) = self.tracker.snapshot().await else {
            return Ok(());
        };

        if previous.source_channel != event.channel_id || previous.source_message != Some(event.id)
        {
            return Ok(());
        }

        let Some(message) = message else {
            return Ok(());
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

    /// Adopt the last message in a target chat when it was sent by this bot.
    async fn recover_from_target(
        &self,
        target_id: serenity::ChannelId,
        bot_id: serenity::UserId,
    ) -> Option<MirroredMessage> {
        let mirror = self.config.mirror.as_ref()?;

        let message = match self.sink.latest_message(target_id).await {
            Ok(Some(message)) => message,
            Ok(None) => return None,
            Err(error) => {
                warn!(%error, "failed to look up the latest message for recovery");
                return None;
            }
        };

        if message.author.id != bot_id {
            return None;
        }

        let pair = MirroredMessage {
            source_channel: mirror.source_channel,
            source_message: None,
            target_channel: target_id,
            target_message: message.id,
        };

        self.tracker.set(pair.clone()).await;
        Some(pair)
    }

    /// Edit the mirrored copy when it is the latest message, otherwise resend
    /// it at the bottom and remove the old copy.
    async fn update_mirrored(
        &self,
        previous: &MirroredMessage,
        content: &str,
        embeds: &[serenity::Embed],
    ) -> Result<UpdateOutcome, Error> {
        let is_last = match self.sink.latest_message(previous.target_channel).await {
            Ok(Some(message)) => message.id == previous.target_message,
            Ok(None) => false,
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

fn message_contains(message: &serenity::Message, filter: Option<&str>) -> bool {
    if filter.is_none() {
        return true;
    }

    let mut texts: Vec<&str> = vec![message.content.as_str()];

    for embed in &message.embeds {
        if let Some(title) = embed.title.as_deref() {
            texts.push(title);
        }

        if let Some(description) = embed.description.as_deref() {
            texts.push(description);
        }

        if let Some(footer) = embed.footer.as_ref() {
            texts.push(&footer.text);
        }

        for field in &embed.fields {
            texts.push(&field.name);
            texts.push(&field.value);
        }
    }

    text_contains(texts, filter)
}

fn text_contains<'a>(texts: impl IntoIterator<Item = &'a str>, filter: Option<&str>) -> bool {
    match filter {
        Some(expected) => {
            let needle = expected.to_lowercase();

            texts
                .into_iter()
                .any(|text| text.to_lowercase().contains(&needle))
        }
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use poise::serenity_prelude as serenity;

    use super::{
        MirrorTracker, MirroredMessage, PREFIX_SOURCE_BOT, prefixed_content, text_contains,
    };

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

    #[test]
    fn matches_text_case_insensitively() {
        assert!(text_contains(
            ["Now started playing: Song"],
            Some("Started Playing")
        ));
    }

    #[test]
    fn matches_any_of_the_message_texts() {
        assert!(text_contains(
            ["", "Started playing: Song", "description"],
            Some("started playing")
        ));
    }

    #[test]
    fn rejects_when_no_text_matches() {
        assert!(!text_contains(["Paused", "Idle"], Some("Started playing")));
        assert!(!text_contains([""], Some("Started playing")));
    }

    #[test]
    fn accepts_everything_without_a_text_filter() {
        assert!(text_contains(["Paused"], None));
        assert!(text_contains(std::iter::empty::<&str>(), None));
    }

    #[tokio::test]
    async fn tracker_stores_and_clears_the_last_pair() {
        let tracker = MirrorTracker::default();
        assert!(tracker.snapshot().await.is_none());

        let pair = MirroredMessage {
            source_channel: serenity::ChannelId::new(1),
            source_message: Some(serenity::MessageId::new(2)),
            target_channel: serenity::ChannelId::new(3),
            target_message: serenity::MessageId::new(4),
        };

        tracker.set(pair.clone()).await;
        assert_eq!(tracker.snapshot().await, Some(pair));

        tracker.clear().await;
        assert!(tracker.snapshot().await.is_none());
    }
}
