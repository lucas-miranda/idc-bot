//! Pure permission-overwrite transformations for public voice channels.
//!
//! These functions never touch the network: they take the channel's current
//! overwrites and return the replacement list to submit. Keeping the diffing
//! here makes it testable and removes the duplicated patch logic that used to
//! live in the visibility helpers.

use poise::serenity_prelude as serenity;

fn everyone_overwrite_type(guild_id: serenity::GuildId) -> serenity::PermissionOverwriteType {
    serenity::PermissionOverwriteType::Role(guild_id.everyone_role())
}

/// Compute the overwrites that make a channel visible.
///
/// Returns `None` when the channel is already visible and nothing needs to be
/// sent to Discord.
pub fn reveal(
    current: &[serenity::PermissionOverwrite],
    guild_id: serenity::GuildId,
) -> Option<Vec<serenity::PermissionOverwrite>> {
    let kind = everyone_overwrite_type(guild_id);
    let granted = serenity::Permissions::VIEW_CHANNEL | serenity::Permissions::CONNECT;

    // An everyone overwrite that denies neither VIEW_CHANNEL nor CONNECT means
    // the channel is already visible.
    let already_visible = current
        .iter()
        .any(|overwrite| overwrite.kind == kind && (overwrite.deny & granted).is_empty());

    if already_visible {
        return None;
    }

    let mut pending = vec![serenity::PermissionOverwrite {
        allow: granted,
        deny: serenity::Permissions::empty(),
        kind,
    }];

    let mut updated = Vec::with_capacity(current.len() + 1);
    for overwrite in current {
        let mut overwrite = overwrite.clone();

        if let Some(position) = pending
            .iter()
            .position(|extra| extra.kind == overwrite.kind)
        {
            let extra = pending.remove(position);
            // Remove the denial instead of allowing the permission outright:
            // a direct allow would override per-member denies.
            overwrite.deny &= !extra.allow;
        }

        updated.push(overwrite);
    }

    // Missing entries are appended with their allow bits cleared, for the same
    // reason as above.
    for mut extra in pending {
        extra.allow = serenity::Permissions::empty();
        updated.push(extra);
    }

    Some(updated)
}

/// Compute the overwrites that make a channel invisible.
pub fn conceal(
    current: &[serenity::PermissionOverwrite],
    guild_id: serenity::GuildId,
) -> Vec<serenity::PermissionOverwrite> {
    let kind = everyone_overwrite_type(guild_id);
    let denied = serenity::Permissions::VIEW_CHANNEL | serenity::Permissions::CONNECT;

    let mut pending = vec![serenity::PermissionOverwrite {
        allow: serenity::Permissions::empty(),
        deny: denied,
        kind,
    }];

    let mut updated = Vec::with_capacity(current.len() + 1);
    for overwrite in current {
        let mut overwrite = overwrite.clone();

        if let Some(position) = pending
            .iter()
            .position(|extra| extra.kind == overwrite.kind)
        {
            let extra = pending.remove(position);
            overwrite.deny |= extra.deny;
            overwrite.allow &= !extra.deny;
        }

        updated.push(overwrite);
    }

    updated.extend(pending);
    updated
}

/// Whether `@everyone` is still allowed to speak in this channel.
///
/// A missing overwrite is treated as "allowed", matching Discord's behaviour.
pub fn allows_speak(
    current: &[serenity::PermissionOverwrite],
    guild_id: serenity::GuildId,
) -> bool {
    let kind = everyone_overwrite_type(guild_id);

    current
        .iter()
        .find(|overwrite| overwrite.kind == kind)
        .map(|overwrite| !overwrite.deny.speak())
        .unwrap_or(true)
}

/// Whether `@everyone` can see this channel.
///
/// A missing overwrite is treated as "allowed", matching Discord's behaviour.
pub fn allows_view(current: &[serenity::PermissionOverwrite], guild_id: serenity::GuildId) -> bool {
    let kind = everyone_overwrite_type(guild_id);

    current
        .iter()
        .find(|overwrite| overwrite.kind == kind)
        .map(|overwrite| !overwrite.deny.view_channel())
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use poise::serenity_prelude as serenity;

    use super::{allows_speak, allows_view, conceal, reveal};

    fn guild() -> serenity::GuildId {
        serenity::GuildId::new(1)
    }

    fn everyone() -> serenity::PermissionOverwriteType {
        serenity::PermissionOverwriteType::Role(guild().everyone_role())
    }

    fn hidden() -> serenity::PermissionOverwrite {
        serenity::PermissionOverwrite {
            allow: serenity::Permissions::empty(),
            deny: serenity::Permissions::VIEW_CHANNEL | serenity::Permissions::CONNECT,
            kind: everyone(),
        }
    }

    #[test]
    fn reveal_clears_the_denial_of_an_existing_overwrite() {
        let updated = reveal(&[hidden()], guild()).expect("channel should be edited");

        assert_eq!(updated.len(), 1);
        assert!(updated[0].deny.is_empty());
        assert!(updated[0].allow.is_empty());
    }

    #[test]
    fn reveal_appends_a_missing_everyone_overwrite_without_allowing_it() {
        let updated = reveal(&[], guild()).expect("channel should be edited");

        assert_eq!(updated.len(), 1);
        assert!(updated[0].allow.is_empty());
        assert!(updated[0].deny.is_empty());
    }

    #[test]
    fn reveal_is_a_no_op_when_the_channel_is_already_visible() {
        let visible = serenity::PermissionOverwrite {
            allow: serenity::Permissions::empty(),
            deny: serenity::Permissions::empty(),
            kind: everyone(),
        };

        assert!(reveal(&[visible], guild()).is_none());
    }

    #[test]
    fn conceal_denies_view_and_connect() {
        let visible = serenity::PermissionOverwrite {
            allow: serenity::Permissions::VIEW_CHANNEL,
            deny: serenity::Permissions::empty(),
            kind: everyone(),
        };

        let updated = conceal(&[visible], guild());

        assert_eq!(updated.len(), 1);
        assert!(updated[0].deny.view_channel());
        assert!(updated[0].deny.connect());
        assert!(!updated[0].allow.view_channel());
    }

    #[test]
    fn conceal_keeps_unrelated_overwrites_untouched() {
        let member = serenity::PermissionOverwrite {
            allow: serenity::Permissions::SPEAK,
            deny: serenity::Permissions::empty(),
            kind: serenity::PermissionOverwriteType::Member(serenity::UserId::new(42)),
        };

        let updated = conceal(std::slice::from_ref(&member), guild());

        assert_eq!(updated.len(), 2);
        assert_eq!(updated[0], member);
    }

    #[test]
    fn everyone_can_speak_by_default() {
        assert!(allows_speak(&[], guild()));
    }

    #[test]
    fn everyone_can_view_by_default() {
        assert!(allows_view(&[], guild()));
    }

    #[test]
    fn everyone_cannot_view_when_denied() {
        let hidden = serenity::PermissionOverwrite {
            allow: serenity::Permissions::empty(),
            deny: serenity::Permissions::VIEW_CHANNEL,
            kind: everyone(),
        };

        assert!(!allows_view(&[hidden], guild()));
    }

    #[test]
    fn everyone_cannot_speak_when_denied() {
        let muted = serenity::PermissionOverwrite {
            allow: serenity::Permissions::empty(),
            deny: serenity::Permissions::SPEAK,
            kind: everyone(),
        };

        assert!(!allows_speak(&[muted], guild()));
    }

    #[test]
    fn reveal_does_not_affect_speak_permission() {
        let hidden = hidden();
        let updated = reveal(&[hidden], guild()).expect("channel should be edited");

        assert!(allows_speak(&updated, guild()));
    }
}
