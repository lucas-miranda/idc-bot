use poise::serenity_prelude::{self as serenity, CacheHttp};

pub trait PermissionsExtras {
    fn is_staff(&self, ctx: &serenity::Context, channel: &serenity::GuildChannel) -> bool;
}

impl PermissionsExtras for serenity::Member {
    fn is_staff(&self, ctx: &serenity::Context, channel: &serenity::GuildChannel) -> bool {
        if let Some(cache) = ctx.cache()
         && let Some(guild) = self.guild_id.to_guild_cached(cache)
        {
            let permissions = guild.user_permissions_in(channel, self);
            permissions.manage_channels()
        } else {
            false
        }
    }
}
