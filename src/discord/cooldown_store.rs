use std::{collections::HashMap, sync::Mutex, time::Instant};

use crate::ports::CooldownStore;

/// In-memory [`CooldownStore`].
///
/// The previous implementation reached into serenity's `TypeMap` from inside
/// the message-sending code; keeping the timestamps behind this small adapter
/// removes that coupling.
#[derive(Default)]
pub struct InMemoryCooldownStore {
    last_sent: Mutex<HashMap<String, Instant>>,
}

impl CooldownStore for InMemoryCooldownStore {
    fn last_sent(&self, key: &str) -> Option<Instant> {
        self.last_sent
            .lock()
            .expect("cooldown store poisoned")
            .get(key)
            .copied()
    }

    fn set_last_sent(&self, key: &str, at: Instant) {
        self.last_sent
            .lock()
            .expect("cooldown store poisoned")
            .insert(key.to_owned(), at);
    }
}
