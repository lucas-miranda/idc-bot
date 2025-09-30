use poise::serenity_prelude::{self as serenity, CacheHttp};
use crate::permissions::PermissionsExtras;

pub trait ChannelExtras {
    fn make_visible(&mut self, ctx: &serenity::Context) -> impl Future<Output = Result<bool, crate::Error>>;
    fn make_invisible(&mut self, ctx: &serenity::Context) -> impl Future<Output = Result<(), crate::Error>>;
    fn get_connected_staff_member(&self, ctx: &serenity::Context) -> Option<serenity::Member>;
}

impl ChannelExtras for serenity::GuildChannel {
    async fn make_visible(&mut self, ctx: &serenity::Context) -> Result<bool, crate::Error> {
        let everyone_role = serenity::PermissionOverwriteType::Role(self.guild_id.everyone_role());

        // set of permissions we'll be changing
        // it isn't set as is, but isn't applied on top of permissions list
        // an allow here means which a permission will not be denied anymore
        // but it'll not be overwrited to be allowed as it'll cause permission problems
        let mut replace_permissions = vec![serenity::PermissionOverwrite {
            allow: serenity::Permissions::VIEW_CHANNEL,
            //allow: serenity::Permissions::VIEW_CHANNEL | serenity::Permissions::CONNECT,
            deny: serenity::Permissions::empty(),
            kind: everyone_role,
        }];

        let updated_permission = self.permission_overwrites
            .iter()
            .filter(|p| {
                // find an entry with same kind
                // and which is denying permissions
                // which we'll be allowing
                replace_permissions
                    .iter()
                    .any(|extra_p| extra_p.kind == p.kind && (p.deny & extra_p.allow) == extra_p.allow)
            })
            .count();

        println!("permissions: {}, updated: {}", self.permission_overwrites.len(), updated_permission);

        if updated_permission == replace_permissions.len() {
            // there is nothing to do
            return Ok(false);
        }

        // at first try to find replace_permissions at permissions list and update them
        // consuming the entries at replace_permissions
        let mut make_visible_permissions: Vec<serenity::PermissionOverwrite>
            = self.permission_overwrites
                  .iter()
                  .cloned()
                  .map(|mut p| {
                      let i = replace_permissions.iter()
                                                 .enumerate()
                                                 .find_map(|(n, extra_p)| {
                                                     (extra_p.kind == p.kind).then_some(n)
                                                 });

                      if let Some(pos) = i {
                          // modify permission overwrite
                          let extra_p = replace_permissions.remove(pos);

                          p.deny &= !extra_p.allow;
                      }

                      p
                  })
                  .collect();

        // if replace_permissions wasn't completed consumed, append it to the permissions
        if !replace_permissions.is_empty() {
            make_visible_permissions
                .extend(
                    replace_permissions.into_iter()
                                       .map(|mut extra_p| {
                                           // we shouldn't allow permissions directly
                                           // it'll break everything
                                           extra_p.allow = serenity::Permissions::empty();
                                           extra_p
                                       })
                )
        }

        let edit = serenity::EditChannel::new().permissions(make_visible_permissions);
        self.edit(ctx.http(), edit).await?;

        Ok(true)
    }

    async fn make_invisible(&mut self, ctx: &serenity::Context) -> Result<(), crate::Error> {
        let everyone_role = serenity::PermissionOverwriteType::Role(self.guild_id.everyone_role());

        let mut replace_permissions = vec![serenity::PermissionOverwrite {
            allow: serenity::Permissions::empty(),
            //deny: serenity::Permissions::VIEW_CHANNEL | serenity::Permissions::CONNECT,
            deny: serenity::Permissions::VIEW_CHANNEL,
            kind: everyone_role,
        }];

        // at first try to find replace_permissions at permissions list and update them
        // consuming the entries at replace_permissions
        let mut make_invisible_permissions: Vec<serenity::PermissionOverwrite>
            = self.permission_overwrites
                  .iter()
                  .cloned()
                  .map(|mut p| {
                      let i = replace_permissions.iter()
                                                 .enumerate()
                                                 .find_map(|(n, extra_p)| (extra_p.kind == p.kind).then_some(n));

                      if let Some(pos) = i {
                          // modify permission overwrite
                          let extra_p = replace_permissions.remove(pos);

                          p.deny |= extra_p.deny;
                          p.allow &= !extra_p.deny;
                      }

                      p
                  })
                  .collect();

        // if replace_permissions wasn't completed consumed, append it to the permissions
        if !replace_permissions.is_empty() {
            make_invisible_permissions.extend(replace_permissions)
        }

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
