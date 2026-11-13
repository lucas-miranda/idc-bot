use poise::serenity_prelude as serenity;
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

use crate::error::ConfigError;

/// Message used when a call is registered without an explicit one.
pub const DEFAULT_CALL_MESSAGE: &str = " call aberta! ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpsertOutcome {
    Added,
    Updated,
    Unchanged,
}

/// Add or update the call for `voice_channel` in the document.
///
/// Only the role is compared when the entry already exists; the message is left
/// untouched.
pub fn upsert(
    document: &mut DocumentMut,
    voice_channel: serenity::ChannelId,
    role_id: serenity::RoleId,
) -> Result<UpsertOutcome, ConfigError> {
    let channel_id = voice_channel.get();
    let role_id = role_id.get();

    let tables = call_tables_mut(document)?;

    match find_index(tables, channel_id) {
        Some(index) => {
            let table = tables
                .get_mut(index)
                .expect("index came from the array of tables");
            let current_role = table.get("role_id").and_then(Item::as_integer);

            if current_role == Some(role_id as i64) {
                return Ok(UpsertOutcome::Unchanged);
            }

            table["role_id"] = value(role_id as i64);
            Ok(UpsertOutcome::Updated)
        }
        None => {
            let mut table = Table::new();
            table["voice_channel"] = value(channel_id as i64);
            table["role_id"] = value(role_id as i64);
            table["message"] = value(DEFAULT_CALL_MESSAGE);
            tables.push(table);
            Ok(UpsertOutcome::Added)
        }
    }
}

/// Remove the call for `voice_channel`. Returns whether an entry was removed.
pub fn remove(
    document: &mut DocumentMut,
    voice_channel: serenity::ChannelId,
) -> Result<bool, ConfigError> {
    let tables = call_tables_mut(document)?;

    match find_index(tables, voice_channel.get()) {
        Some(index) => {
            tables.remove(index);
            Ok(true)
        }
        None => Ok(false),
    }
}

fn call_tables_mut(document: &mut DocumentMut) -> Result<&mut ArrayOfTables, ConfigError> {
    if document.get("call_kinds").is_none() {
        document["call_kinds"] = Item::ArrayOfTables(ArrayOfTables::new());
    }

    document["call_kinds"]
        .as_array_of_tables_mut()
        .ok_or_else(|| ConfigError::Invalid("`call_kinds` must be an array of tables".to_owned()))
}

fn find_index(tables: &ArrayOfTables, channel_id: u64) -> Option<usize> {
    tables.iter().position(|table| {
        table.get("voice_channel").and_then(Item::as_integer) == Some(channel_id as i64)
    })
}

#[cfg(test)]
mod tests {
    use poise::serenity_prelude as serenity;
    use toml_edit::DocumentMut;

    use crate::config::Config;

    use super::{DEFAULT_CALL_MESSAGE, UpsertOutcome, remove, upsert};

    const BASE: &str = r#"
# keep this comment
guild_id = 1
broadcast_channel_id = 2
cooldown_secs = 3600
"#;

    fn document() -> DocumentMut {
        BASE.parse().expect("base document should parse")
    }

    fn channel(id: u64) -> serenity::ChannelId {
        serenity::ChannelId::new(id)
    }

    fn role(id: u64) -> serenity::RoleId {
        serenity::RoleId::new(id)
    }

    #[test]
    fn adds_a_call() {
        let mut doc = document();

        let outcome = upsert(&mut doc, channel(3), role(4)).expect("upsert should work");
        assert_eq!(outcome, UpsertOutcome::Added);

        let config = Config::from_toml_str(&doc.to_string()).expect("result should parse");
        assert_eq!(config.call_kinds.len(), 1);
        assert_eq!(config.call_kinds[0].voice_channel.get(), 3);
        assert_eq!(config.call_kinds[0].role_id.get(), 4);
        assert_eq!(config.call_kinds[0].message, DEFAULT_CALL_MESSAGE);
    }

    #[test]
    fn updates_only_when_the_role_changes() {
        let mut doc = document();
        upsert(&mut doc, channel(3), role(4)).expect("first upsert should work");

        let outcome = upsert(&mut doc, channel(3), role(4)).expect("second upsert should work");
        assert_eq!(outcome, UpsertOutcome::Unchanged);

        let outcome = upsert(&mut doc, channel(3), role(5)).expect("third upsert should work");
        assert_eq!(outcome, UpsertOutcome::Updated);

        let config = Config::from_toml_str(&doc.to_string()).expect("result should parse");
        assert_eq!(config.call_kinds.len(), 1);
        assert_eq!(config.call_kinds[0].role_id.get(), 5);
    }

    #[test]
    fn removes_a_call() {
        let mut doc = document();
        upsert(&mut doc, channel(3), role(4)).expect("upsert should work");

        assert!(remove(&mut doc, channel(3)).expect("remove should work"));

        let config = Config::from_toml_str(&doc.to_string()).expect("result should parse");
        assert!(config.call_kinds.is_empty());
    }

    #[test]
    fn removing_an_unknown_call_is_a_no_op() {
        let mut doc = document();

        assert!(!remove(&mut doc, channel(99)).expect("remove should work"));
    }

    #[test]
    fn preserves_comments() {
        let mut doc = document();
        upsert(&mut doc, channel(3), role(4)).expect("upsert should work");

        assert!(doc.to_string().contains("# keep this comment"));
    }

    #[test]
    fn can_add_again_after_removing_the_last_call() {
        let mut doc = document();
        upsert(&mut doc, channel(3), role(4)).expect("first upsert should work");
        assert!(remove(&mut doc, channel(3)).expect("remove should work"));

        let outcome = upsert(&mut doc, channel(3), role(4)).expect("second upsert should work");
        assert_eq!(outcome, UpsertOutcome::Added);
    }
}
