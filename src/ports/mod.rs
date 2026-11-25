//! Ports: the narrow contracts the application layer depends on.
//!
//! Adapters in [`crate::discord`] implement them with serenity; tests can
//! implement them with fakes. The application never sees a `Context`.

mod cooldown_store;
pub use cooldown_store::CooldownStore;

mod message_sink;
pub use message_sink::{EditOutcome, MessageSink};

mod notifier;
pub use notifier::Notifier;

mod voice_channel;
pub use voice_channel::VoiceChannelPort;
