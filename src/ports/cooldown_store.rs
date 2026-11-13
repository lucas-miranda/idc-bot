use std::time::Instant;

/// Remembers when each labelled message was last sent.
///
/// This is a synchronous, object-safe port so it can live behind an `Arc<dyn
/// CooldownStore>` in the notifier.
pub trait CooldownStore: Send + Sync {
    fn last_sent(&self, key: &str) -> Option<Instant>;
    fn set_last_sent(&self, key: &str, at: Instant);
}
