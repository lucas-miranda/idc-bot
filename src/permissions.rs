use poise::serenity_prelude::{self as serenity, CacheHttp};

pub trait PermissionsExtras {
    fn is_staff(&self) -> bool;
    fn get_permissions(&self, ctx: &serenity::Context, channel: &serenity::GuildChannel) -> Option<serenity::Permissions>;
}

impl PermissionsExtras for serenity::Member {
    /// Check if member is a staff member for provided channel.
    fn is_staff(&self) -> bool {
        !self.user.bot && self.permissions.map(|p| p.manage_channels()).unwrap_or_default()
    }

    fn get_permissions(&self, ctx: &serenity::Context, channel: &serenity::GuildChannel) -> Option<serenity::Permissions> {
        if !self.user.bot
         && let Some(cache) = ctx.cache()
         && let Some(guild) = self.guild_id.to_guild_cached(cache)
        {
            Some(guild.user_permissions_in(channel, self))
        } else {
            None
        }
    }
}
