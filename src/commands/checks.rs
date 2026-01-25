use serenity::all::GuildId;

use crate::commands::{CommandContext, CommandError};


pub async fn is_bot_or_guild_owner(ctx: CommandContext<'_>) -> Result<bool, CommandError> {
    if is_bot_owner(&ctx) == true {
        return Ok(true)
    }

    if is_guild_owner(&ctx) == true {
        return Ok(true)
    }

    Ok(false)
}

fn is_bot_owner(ctx: &CommandContext<'_>) -> bool {
    ctx.framework().options().owners.contains(&ctx.author().id)
}

fn is_guild_owner(ctx: &CommandContext<'_>) -> bool {
    if let Some(guild) = ctx.guild() {
        if guild.owner_id == ctx.author().id {
            return true;
        }
    }

    false
}

pub async fn is_admin(ctx: CommandContext<'_>) -> Result<bool, CommandError> {
    if let Some(permissions) = ctx.author_member().await.map(|m| m.permissions).flatten() {
        if permissions.administrator() {
            return Ok(true);
        }
    }

    Ok(false)
}

pub async fn have_ctx_guild_id(ctx: CommandContext<'_>) -> Result<bool, CommandError> {
    parse_ctx_guild_id(&ctx).map(|_| true)
}

pub fn parse_ctx_guild_id(ctx: &CommandContext<'_>) -> Result<GuildId, CommandError> {
    ctx.guild_id().ok_or_else(|| "Call this command from guild".into())
}