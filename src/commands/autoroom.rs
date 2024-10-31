use poise::serenity_prelude as serenity;
use ::serenity::all::ChannelId;

use crate::{services::autoroom::grant_guest_privileges, MonitoredAutoRoom};

use super::{ CommandContext, CommandError };


#[poise::command(slash_command, subcommands("invite"))]
pub async fn autoroom(ctx: CommandContext<'_>) -> Result<(), CommandError> {
    ctx.say(format!("Available commands: ({}, {})", "invite", "-")).await?;
    Ok(())
}

// #[poise::command(slash_command, prefix_command)]
#[poise::command(slash_command)]
pub async fn invite(
    ctx: CommandContext<'_>,
    #[description = "Invite a user to the apparts"] user: Option<serenity::User>,
) -> Result<(), CommandError> {
    let pool = &ctx.data().pool;

    let author = ctx.author();
    let invited_user = match user.as_ref() {
        Some(user) => user,
        None => return Err("Please specify the user you want to invite".into())
    };
    let err_msg = format!("The connected voice channel was not found");
    let monitored_autoroom = match MonitoredAutoRoom::get_by_owner_id(pool, author.id.get() as i64).await {
        Ok(option) => match option {
            Some(monitored_autoroom_result) => monitored_autoroom_result,
            None => return Err(CommandError::from(err_msg))
        },
        Err(_) => return Err(CommandError::from(err_msg)),
    };

    let channel_id = ChannelId::new(monitored_autoroom.channel_id as u64);
    grant_guest_privileges(&ctx.http(), &channel_id, &invited_user.id).await?;

    ctx.say(
        &format!(
            "{} has been successfully invited",
            invited_user.name
        )
    ).await?;

    Ok(())
}