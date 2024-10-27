use serenity::all::{ ChannelId, Http, PermissionOverwrite, PermissionOverwriteType, Permissions, UserId};



pub async fn  grant_owner_privileges(http: &Http, channel: &ChannelId, user_id: &UserId) -> Result<(), serenity::Error> {
    let permissions = PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL
            | Permissions::SEND_MESSAGES
            | Permissions::MANAGE_CHANNELS
            | Permissions::MUTE_MEMBERS
            | Permissions::DEAFEN_MEMBERS,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(*user_id),
    };
    if let Err(err) = channel.create_permission(http, permissions).await {
        println!(
            "Failed to grant channel({:?}) permissions to the user({:?}). Error: \"{:?}\"",
            &channel.get(),
            &user_id.get(),
            &err
        );
        return Err(err);
    }
    Ok(())
}


pub async fn  grant_guest_privileges(http: &Http, channel: &ChannelId, user_id: &UserId) -> Result<(), serenity::Error> {
    let permissions = PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(*user_id),
    };
    if let Err(err) = channel.create_permission(http, permissions).await {
        println!(
            "Failed to grant channel({:?}) permissions to the user({:?}). Error: \"{:?}\"",
            &channel.get(),
            &user_id.get(),
            &err
        );
        return Err(err);
    }
    Ok(())
}