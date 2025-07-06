use serenity::all::{Channel, ChannelId, GuildChannel, UserId};

use crate::commands::{CommandContext, CommandError};



pub async fn get_user_voice_channel(ctx: &CommandContext<'_>, user_id: Option<&UserId>) -> Result<Channel, CommandError> {
    let guild = match ctx.guild() {
        Some(_guild) => _guild.clone(),
        None => return Err(CommandError::from("Autoroom works only inside the guild"))
    };
    let user_id = match user_id {
        Some(_user_id) => _user_id,
        None => &ctx.author().id
    };
    let voice_channel_id = match guild.voice_states.get(user_id) {
        Some(vocie_sate) => match vocie_sate.channel_id {
            Some(channel_id) => channel_id,
            None => return Err(CommandError::from("You are not connected to voice channel"))
        },
        None => return Err(CommandError::from("You are not connected to voice channel"))
    };
    match voice_channel_id.to_channel(&ctx.http()).await {
        Ok(channel) => Ok(channel),
        Err(err) => {
            println!("Failed to convert voice_channel_id to channel.\n{:?}", err);
            return Err(CommandError::from("You are not connected to voice channel"))
        }
    }
}


pub async fn get_user_guild_voice_channel(ctx: &CommandContext<'_>, user_id: Option<&UserId>) -> Result<GuildChannel, CommandError> {
    let voice_channel = get_user_voice_channel(ctx, user_id).await?;
    match voice_channel.guild() {
        Some(_channel) => Ok(_channel),
        None => return Err(CommandError::from("This room is outside autoroom guild"))
    }
    
}


pub fn get_voice_channel_category(voice_channel: &GuildChannel) -> Result<ChannelId, CommandError> {
    match voice_channel.parent_id {
        Some(_category_id) => Ok(_category_id),
        None => return Err(CommandError::from(
            format!(
                "This room is outside category, VoiceChannelID: {}",
                voice_channel.id.get()
            )
        ))
    }
}