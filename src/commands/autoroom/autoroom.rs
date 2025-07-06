use poise::serenity_prelude as serenity;
use ::serenity::all::{ChannelId, Mentionable};

use crate::{
    commands::{CommandContext, CommandError},
    services::autoroom::grant_guest_privileges,
    MonitoredAutoRoom
};

use super::savedroom::{save, load};


#[poise::command(
    slash_command,
    subcommands(
        "invite",
        "save",
        "load"
    )
)]
pub async fn autoroom(ctx: CommandContext<'_>) -> Result<(), CommandError> {
    ctx.say(format!("Available commands: ({}, {})", "invite", "-")).await?;
    Ok(())
}

// #[poise::command(slash_command, prefix_command)]
#[poise::command(slash_command)]
pub async fn invite(
    ctx: CommandContext<'_>,
    #[description = "Invite a user to the apparts"] user: serenity::User,
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
    let result_msg = match grant_guest_privileges(&ctx, &channel_id, &user.id).await {
        Ok(_) => &format!(
            "{} has been successfully invited",
            user.mention().to_string()
        ),
        Err(_) => &format!(
            "Failed to invite {:?}",
            user.name
        ),
    };
    
    ctx.say(result_msg).await?;
    Ok(())
}