use crate::Error;
use crate::domain::CallKind;

/// Sends (and rate-limits) call announcements.
pub trait Notifier: Send + Sync {
    /// Name of the "social" role, used for the existing diagnostic logs.
    fn social_role_name(&self) -> &str;

    fn notify(&self, kind: &CallKind) -> impl Future<Output = Result<(), Error>> + Send;
}
