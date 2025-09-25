use poise::serenity_prelude::{self as serenity, CacheHttp};
use crate::permissions::PermissionsExtras;

pub trait ChannelExtras {
    fn make_visible(&mut self, ctx: &serenity::Context) -> impl Future<Output = Result<(), crate::Error>>;
    fn make_invisible(&mut self, ctx: &serenity::Context) -> impl Future<Output = Result<(), crate::Error>>;
    fn get_connected_staff_member(&self, ctx: &serenity::Context) -> Option<serenity::Member>;
}

impl ChannelExtras for serenity::GuildChannel {
    async fn make_visible(&mut self, ctx: &serenity::Context) -> Result<(), crate::Error> {
        let everyone_role = serenity::PermissionOverwriteType::Role(self.guild_id.everyone_role());

        // get permissions which isn't assigned to everyone role
        // and chain it with the new everyone permission
        let make_visible_permissions = self.permission_overwrites
            .iter()
            .filter(|p| p.kind != everyone_role)
            .cloned()
            .chain(vec![serenity::PermissionOverwrite {
                allow: serenity::Permissions::VIEW_CHANNEL | serenity::Permissions::CONNECT,
                deny: serenity::Permissions::empty(),
                kind: everyone_role,
            }]);

        let edit = serenity::EditChannel::new().permissions(make_visible_permissions);
        self.edit(ctx.http(), edit).await?;

        Ok(())
    }

    async fn make_invisible(&mut self, ctx: &serenity::Context) -> Result<(), crate::Error> {
        let everyone_role = serenity::PermissionOverwriteType::Role(self.guild_id.everyone_role());

        // get permissions which isn't assigned to everyone role
        // and chain it with the new everyone permission
        let make_invisible_permissions = self.permission_overwrites
            .iter()
            .filter(|p| p.kind != everyone_role)
            .cloned()
            .chain(vec![serenity::PermissionOverwrite {
                allow: serenity::Permissions::empty(),
                deny: serenity::Permissions::VIEW_CHANNEL | serenity::Permissions::CONNECT,
                kind: everyone_role,
            }]);

        let edit = serenity::EditChannel::new().permissions(make_invisible_permissions);
        self.edit(ctx.http(), edit).await?;

        Ok(())
    }

    fn get_connected_staff_member(&self, ctx: &serenity::Context) -> Option<serenity::Member> {
        match self.members(&ctx.cache) {
            Ok(members) => members.into_iter().find(|channel_member| channel_member.is_staff(ctx, self)),
            Err(e) => {
                println!("Failed to get channel members\n{}", e);
                None
            }
        }
    }
}

pub trait ChannelIdExtras {
    fn to_guild_channel(&self, ctx: &serenity::Context) -> impl Future<Output = Option<serenity::GuildChannel>>;
}

impl ChannelIdExtras for serenity::ChannelId {
    async fn to_guild_channel(&self, ctx: &serenity::Context) -> Option<serenity::GuildChannel> {
        let channel = self.to_channel(ctx)
            .await
            .map(|channel| match channel {
                serenity::Channel::Guild(guild_channel) => Some(guild_channel),
                serenity::Channel::Private(_private_channel) => None,
                _ => None,
            });

        match channel {
            Ok(c) => c,
            Err(e) => {
                println!("Failed to get channel.\n{}", e);
                None
            }
        }
    }
}
