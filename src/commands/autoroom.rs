use poise::serenity_prelude as serenity;
use ::serenity::all::{ChannelId, Mentionable};

use crate::{
    MonitoredAutoRoom,
    services::autoroom::{cleanup_categories_monitored_rooms, cleanup_db_monitored_rooms, grant_guest_privileges},
    sql::autoroom::AutoRoom
};

use super::{ CommandContext, CommandError };


#[poise::command(slash_command, subcommands("invite", "cleanup", "add"))]
pub async fn autoroom(ctx: CommandContext<'_>) -> Result<(), CommandError> {
    ctx.say(format!("Available commands: ({}, {})", "invite", "-")).await?;
    Ok(())
}

// #[poise::command(slash_command, prefix_command)]
#[poise::command(slash_command)]
pub async fn invite(
    ctx: CommandContext<'_>,
    #[description = "Invite a user to the apparts"] user: serenity::User,
    #[description = "Send a notify to user"] #[flag] notify: bool,
) -> Result<(), CommandError> {
    let pool = &ctx.data().pool;

    let author = ctx.author();
    let err_msg = format!("The connected voice channel was not found");
    let monitored_autoroom = match MonitoredAutoRoom::get_by_owner_id(pool, author.id.get() as i64).await {
        Ok(option) => match option {
            Some(monitored_autoroom_result) => monitored_autoroom_result,
            None => return Err(CommandError::from(err_msg))
        },
        Err(_) => return Err(CommandError::from(err_msg)),
    };

    let channel_id = ChannelId::new(monitored_autoroom.channel_id as u64);
    grant_guest_privileges(&ctx.http(), &channel_id, &user.id).await?;

    let user_info = match notify {
        true => user.mention().to_string(),
        false => user.name,
    };
    
    ctx.say(
        &format!(
            "{} has been successfully invited",
            &user_info
        )
    ).await?;

    Ok(())
}

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

#[poise::command(slash_command, owners_only, required_permissions = "ADMINISTRATOR")]
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
    let guild_id = match ctx.guild_id() {
        Some(_id) => _id.get() as i64,
        None => return Err("Call this command from guild".into())
    };
    let pool = &ctx.data().pool;
    let channel_id = from_channel.id;
    let category_id = placement_category.id;
    let suffix = match suffix {
        Some(suffix) => suffix,
        None => "room".to_string(),
    };


    let autoroom = AutoRoom {
        channel_id: channel_id.get() as i64,
        guild_id: guild_id,
        category_id: category_id.get() as i64,
        suffix: suffix.to_string() };
    if let Err(err) = autoroom.create(pool).await {
        return Err(err.into())
    };

    ctx.say(format!("Record was created! channel id: {}, category id: {}", channel_id, category_id)).await?;
    Ok(())
}