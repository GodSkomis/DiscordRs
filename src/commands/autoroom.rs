use poise::{CreateReply, serenity_prelude as serenity};
use ::serenity::all::Mentionable;

use crate::{
    services::autoroom::{self, cleanup_categories_monitored_rooms, cleanup_db_monitored_rooms}, sql::autoroom::{AutoRoom, AutoRoomDeleteStrategy}
};

use super::{ CommandContext, CommandError };
use super::checks::{ is_bot_or_guild_owner, parse_ctx_guild_id, have_ctx_guild_id};


#[poise::command(slash_command, subcommands("invite", "cleanup", "add", "list", "remove"), check = "have_ctx_guild_id")]
pub async fn autoroom(ctx: CommandContext<'_>) -> Result<(), CommandError> {
    ctx.say(format!("Available commands: ({}, {})", "invite", "-")).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn invite(
    ctx: CommandContext<'_>,
    #[description = "Invite a user to the connected voice channel"] user: serenity::User,
) -> Result<(), CommandError> {
    let pool = &ctx.data().pool;
    let author = ctx.author();
    
    autoroom::voice_channel::invite_user(ctx.http(), pool, author.id.get() as i64, &user).await?;

    ctx.send(
        CreateReply::default()
            .content(format!("{} has been successfully invited", &user.mention().to_string()))
            .ephemeral(false)
    ).await?;

    Ok(())
}

#[poise::command(context_menu_command = "Invite to PRIVATE Room", check = "have_ctx_guild_id")]
pub async fn context_invite(
    ctx: CommandContext<'_>,
    #[description = "Invite a user to the connected voice channel"] user: serenity::User,
) -> Result<(), CommandError> {
    let pool = &ctx.data().pool;
    let author = ctx.author();
    
    if let Err(err) = autoroom::voice_channel::invite_user(ctx.http(), pool, author.id.get() as i64, &user).await {
        ctx.send(
            CreateReply::default()
                .content(format!("{}", err))
                .ephemeral(true)
        ).await?;
    }

    Ok(())
}

// #[poise::command(slash_command, owners_only, global_cooldown = 3600)]
#[poise::command(slash_command, owners_only)]
pub async fn cleanup(ctx: CommandContext<'_>) -> Result<(), CommandError> {
    let handle = ctx.say("Starting cleanup").await?;

    cleanup_db_monitored_rooms(ctx.serenity_context()).await?;
    cleanup_categories_monitored_rooms(ctx.serenity_context()).await?;
    
    handle.edit(ctx, poise::CreateReply::default()
        .content("Cleanup completed. Check logs for more information")
    ).await?;

    Ok(())
}

#[poise::command(slash_command, check = "is_bot_or_guild_owner")]
pub async fn add(
    ctx: CommandContext<'_>,
    #[description = "VoiceChannelto move from"]
    #[channel_types("Voice")]
        from_channel: serenity::GuildChannel,
    #[description = "Category to move to"]
    #[channel_types("Category")]
        placement_category: serenity::GuildChannel,
    #[description = "Channel Suffix"] #[max_length = 10] suffix: Option<String>,
) -> Result<(), CommandError> {
    let guild_id = parse_ctx_guild_id(&ctx)?;
    let pool = &ctx.data().pool;
    let channel_id = from_channel.id;
    let category_id = placement_category.id;
    let suffix = match suffix {
        Some(suffix) => suffix,
        None => "room".to_string(),
    };

    let autoroom = AutoRoom {
        channel_id: channel_id.get() as i64,
        guild_id: guild_id.get() as i64,
        category_id: category_id.get() as i64,
        suffix: suffix.to_string() };
    if let Err(err) = autoroom.create(pool).await {
        return Err(err.into())
    };

    ctx.say(format!("Record was created! channel id: {}, category id: {}", channel_id, category_id)).await?;
    Ok(())
}

#[poise::command(slash_command, check = "is_bot_or_guild_owner", check = "have_ctx_guild_id")]
pub async fn list(ctx: CommandContext<'_>) -> Result<(), CommandError> {
    let guild_id = parse_ctx_guild_id(&ctx)?;
    
    let pool = &ctx.data().pool;
    let autorooms = AutoRoom::get_guild_autorooms(pool, guild_id.get() as i64).await?;
    let result = match autorooms.is_empty() {
        true => "Records not found".to_string(),
        false => {
            autorooms
                .iter()
                .map(|room| room.to_display_string())
                .collect::<Vec<String>>()
                .join("\n")
        },
    };

    ctx.say(result).await?;
    Ok(())
}

#[poise::command(slash_command, check = "is_bot_or_guild_owner", check = "have_ctx_guild_id")]
pub async fn remove(
    ctx: CommandContext<'_>,
    #[description = "VoiceChannelto move from"]
    #[channel_types("Voice")]
        from_channel: serenity::GuildChannel
) -> Result<(), CommandError> {
    let pool = &ctx.data().pool;
    AutoRoom::delete(
        pool,
        AutoRoomDeleteStrategy::SingleByChannelId(from_channel.id.get() as i64)
    ).await?;
    Ok(())
}