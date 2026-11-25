use std::sync::Arc;

use poise::serenity_prelude as serenity;
use tracing::{debug, warn};

use crate::config::Config;
use crate::domain::overwrites;
use crate::error::Error;
use crate::ports::MessageSink;

/// Flip to `false` to mirror the source bot's content without a name prefix.
const PREFIX_SOURCE_BOT: bool = true;

/// Copies a bot's messages into the text chat of the voice channel it is in.
pub struct MessageMirror<S: MessageSink> {
    config: Arc<Config>,
    sink: S,
}

impl<S: MessageSink> MessageMirror<S> {
    pub fn new(config: Arc<Config>, sink: S) -> Self {
        Self { config, sink }
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

        let Some(target_id) = self
            .sink
            .voice_channel_of(self.config.guild_id, mirror.source_bot)
        else {
            debug!(
                bot_id = %mirror.source_bot,
                "source bot is not in a voice channel; skipping mirror"
            );
            return Ok(());
        };

        if self.config.call_for_channel(target_id).is_none()
            || self.config.ignored_voice_channels.contains(&target_id)
        {
            return Ok(());
        }

        let Some(channel) = self
            .sink
            .fetch_channel(self.config.guild_id, target_id)
            .await
        else {
            return Ok(());
        };

        if channel.kind != serenity::ChannelType::Voice {
            return Ok(());
        }

        if !overwrites::allows_view(&channel.permission_overwrites, channel.guild_id) {
            debug!(
                channel_id = %target_id,
                "target voice channel is not visible; skipping mirror"
            );
            return Ok(());
        }

        let content = mirrored_content(&message.author, &message.content);
        let embeds = message.embeds.clone();

        if let Err(error) = self.sink.send(target_id, &content, &embeds).await {
            warn!(%error, channel_id = %target_id, "failed to mirror message");
        }

        Ok(())
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
    use super::{PREFIX_SOURCE_BOT, prefixed_content};

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
}
