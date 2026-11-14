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

#[cfg(test)]
mod tests {
    use poise::serenity_prelude as serenity;

    use super::can_reload;

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
}
