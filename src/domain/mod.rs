//! Pure business rules.
//!
//! Nothing in this module performs I/O or takes a [`poise::serenity_prelude::Context`].
//! It may use serenity's plain-data value types (ids, permissions, overwrites),
//! but never the client/runtime types.

mod call_event;
pub use call_event::CallEvent;

mod call_kind;
pub use call_kind::CallKind;

mod cooldown;
pub use cooldown::{CooldownPolicy, CooldownStatus};

pub mod overwrites;
