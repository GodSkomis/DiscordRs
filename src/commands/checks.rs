use serenity::all::GuildId;

use crate::commands::{CommandContext, CommandError};


pub async fn is_admin_or_owner(ctx: CommandContext<'_>) -> Result<bool, CommandError> {
    let author_id = ctx.author().id;

    // 1. Проверяем, является ли пользователь владельцем бота
    // (фреймворк берет этот список из initialize_owners)
    let is_bot_owner = ctx.framework().options().owners.contains(&author_id);
    if is_bot_owner {
        return Ok(true);
    }

    // // 2. Проверяем, является ли пользователь администратором на сервере
    // if let Some(permissions) = ctx.author_member().await.map(|m| m.permissions).flatten() {
    //     if permissions.administrator() {
    //         return Ok(true);
    //     }
    // }

    // 3. Дополнительно: проверка на владельца сервера (на случай, если у него нет роли админа)
    if let Some(guild) = ctx.guild() {
        if guild.owner_id == author_id {
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