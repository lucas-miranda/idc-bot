use std::time::{Duration, Instant};

/// How long a given message must wait before it may be sent again.
#[derive(Clone, Debug)]
pub struct CooldownPolicy {
    duration: Duration,
}

impl CooldownPolicy {
    pub fn new(duration: Duration) -> Self {
        Self { duration }
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Decide whether a message may be sent at `now`, given when it was last sent.
    pub fn check(&self, now: Instant, last_sent: Option<Instant>) -> CooldownStatus {
        match last_sent {
            Some(last) => {
                let elapsed = now.duration_since(last);
                if elapsed < self.duration {
                    CooldownStatus::TooSoon { elapsed }
                } else {
                    CooldownStatus::Ready
                }
            }
            None => CooldownStatus::Ready,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CooldownStatus {
    Ready,
    TooSoon { elapsed: Duration },
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{CooldownPolicy, CooldownStatus};

    fn policy() -> CooldownPolicy {
        CooldownPolicy::new(Duration::from_secs(60 * 60))
    }

    #[test]
    fn ready_when_never_sent() {
        assert_eq!(policy().check(Instant::now(), None), CooldownStatus::Ready);
    }

    #[test]
    fn too_soon_within_the_window() {
        let now = Instant::now();
        let status = policy().check(now, Some(now - Duration::from_secs(10)));

        assert!(matches!(status, CooldownStatus::TooSoon { .. }));
    }

    #[test]
    fn ready_when_the_window_has_elapsed() {
        let now = Instant::now();
        let status = policy().check(now, Some(now - Duration::from_secs(60 * 60)));

        assert_eq!(status, CooldownStatus::Ready);
    }

    #[test]
    fn ready_after_the_window_has_passed() {
        let now = Instant::now();
        let status = policy().check(now, Some(now - Duration::from_secs(60 * 60 + 1)));

        assert_eq!(status, CooldownStatus::Ready);
    }
}
