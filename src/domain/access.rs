use poise::serenity_prelude as serenity;

/// Whether a member may run `/recarregar`.
///
/// Bot owners and members with `MANAGE_GUILD` always may; otherwise the member
/// needs at least one of the roles configured in `reload_roles`.
pub fn can_reload(
    is_owner: bool,
    has_manage_guild: bool,
    member_roles: &[serenity::RoleId],
    allowed_roles: &[serenity::RoleId],
) -> bool {
    is_owner || has_manage_guild || member_roles.iter().any(|role| allowed_roles.contains(role))
}

/// The bot's definition of a staff member:
/// a non-bot user who is a bot owner, has `MANAGE_GUILD`, or holds one of the
/// roles configured in `reload_roles`.
pub fn is_staff(
    is_bot: bool,
    is_owner: bool,
    has_manage_guild: bool,
    member_roles: &[serenity::RoleId],
    allowed_roles: &[serenity::RoleId],
) -> bool {
    !is_bot && can_reload(is_owner, has_manage_guild, member_roles, allowed_roles)
}

#[cfg(test)]
mod tests {
    use poise::serenity_prelude as serenity;

    use super::{can_reload, is_staff};

    fn role(id: u64) -> serenity::RoleId {
        serenity::RoleId::new(id)
    }

    #[test]
    fn owners_are_allowed() {
        assert!(can_reload(true, false, &[], &[]));
    }

    #[test]
    fn manage_guild_is_allowed() {
        assert!(can_reload(false, true, &[], &[]));
    }

    #[test]
    fn configured_roles_are_allowed() {
        assert!(can_reload(false, false, &[role(1)], &[role(1), role(2)]));
    }

    #[test]
    fn unrelated_roles_are_denied() {
        assert!(!can_reload(false, false, &[role(1)], &[role(2)]));
    }

    #[test]
    fn members_without_matches_are_denied() {
        assert!(!can_reload(false, false, &[], &[role(2)]));
    }

    #[test]
    fn staff_excludes_bots() {
        assert!(!is_staff(true, true, true, &[role(1)], &[role(1)]));
    }

    #[test]
    fn staff_includes_owners() {
        assert!(is_staff(false, true, false, &[], &[]));
    }

    #[test]
    fn staff_includes_manage_guild() {
        assert!(is_staff(false, false, true, &[], &[]));
    }

    #[test]
    fn staff_includes_configured_roles() {
        assert!(is_staff(false, false, false, &[role(1)], &[role(1)]));
    }

    #[test]
    fn staff_excludes_unrelated_members() {
        assert!(!is_staff(false, false, false, &[role(1)], &[role(2)]));
    }
}
