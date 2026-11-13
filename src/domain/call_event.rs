/// A user-visible transition in a voice channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallEvent {
    /// The user was not in voice and is now connected.
    Enter,
    /// The user disconnected from voice.
    Leave,
    /// The user switched from one voice channel to another.
    Move,
    /// Any other state change (mute, deafen, ...).
    Other,
}

impl CallEvent {
    /// Classify a voice state update.
    ///
    /// `old_channel` is `None` when there was no previous voice state, and
    /// `Some(None)` when the previous state had no channel. Channel ids are
    /// plain `u64` to keep this function free of Discord types.
    pub fn classify(old_channel: Option<Option<u64>>, new_channel: Option<u64>) -> Self {
        match (old_channel, new_channel) {
            (None, _) => CallEvent::Enter,
            (Some(_), None) => CallEvent::Leave,
            (Some(old), Some(new)) if old != Some(new) => CallEvent::Move,
            _ => CallEvent::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CallEvent;

    #[test]
    fn entering_when_there_was_no_previous_state() {
        assert_eq!(CallEvent::classify(None, Some(1)), CallEvent::Enter);
    }

    #[test]
    fn leaving_when_the_new_state_has_no_channel() {
        assert_eq!(CallEvent::classify(Some(Some(1)), None), CallEvent::Leave);
    }

    #[test]
    fn moving_between_channels() {
        assert_eq!(CallEvent::classify(Some(Some(1)), Some(2)), CallEvent::Move);
    }

    #[test]
    fn moving_from_no_channel_to_a_channel() {
        assert_eq!(CallEvent::classify(Some(None), Some(2)), CallEvent::Move);
    }

    #[test]
    fn staying_in_the_same_channel_is_other() {
        assert_eq!(
            CallEvent::classify(Some(Some(1)), Some(1)),
            CallEvent::Other
        );
    }
}
