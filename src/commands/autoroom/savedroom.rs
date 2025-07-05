use std::collections::{HashMap, HashSet};
use tokio::sync::Mutex;

use crate::{commands::{CommandContext, CommandError}, sql::{prelude::AutoRoom, savedroom::{SavedRoom, SavedRoomDTO}}};

#[derive(Debug, Clone)]
pub struct SaveRoomCacheRecord {
    owner_id: i64,
    guests: HashSet<i64>
}

impl SaveRoomCacheRecord {
    pub fn new(owner_id: i64, guest: Option<i64>) -> Self {
        let mut record = Self {
            owner_id: owner_id,
            guests: HashSet::new()
        };
        if let Some(guest_id) = guest {
            record.guests.insert(guest_id);
        };
        record
    }
}

pub type SavedRoomCache = Mutex<HashMap<i64, SaveRoomCacheRecord>>;


#[poise::command(slash_command)]
pub async fn save(
    ctx: CommandContext<'_>,
    #[description = "Name for a record"] name: Option<String>,
) -> Result<(), CommandError> {

    let guild = match ctx.guild() {
        Some(_guild) => _guild.clone(),
        None => return Err(CommandError::from("Autoroom works only inside the guild"))
    };
    let voice_channel_id = match guild.voice_states.get(&ctx.author().id) {
        Some(vocie_sate) => match vocie_sate.channel_id {
            Some(channel_id) => channel_id,
            None => return Err(CommandError::from("You are not connected to voice channel"))
        },
        None => return Err(CommandError::from("You are not connected to voice channel"))
    };

    let room_cache = {
        let cache = &ctx.data().savedroom_cache.lock().await;
        let record = match cache.get(&(voice_channel_id.get() as i64)) {
            Some(room_cache) => room_cache.clone(),
            None => return Err(CommandError::from("Nothing to save"))
        };
        record
    };

    if room_cache.guests.is_empty() {
        return Err(CommandError::from("Nothing to save"))
    }

    let pool = &ctx.data().pool;

    let voice_channel = match voice_channel_id.to_channel(&ctx.http()).await {
        Ok(channel) => channel,
        Err(err) => {
            println!("Failed to convert voice_channel_id to channel.\n{:?}", err);
            return Err(CommandError::from("You are not connected to voice channel"))
        }
    };
    let voice_channel_name = match voice_channel.clone().guild() {
        Some(_channel) => _channel.name,
        None => return Err(CommandError::from("This room is outside autoroom guild"))
    };
    let category_id = match voice_channel.clone().category() {
        Some(_category) => _category.id.get().clone(),
        None => return Err(CommandError::from(
            format!(
                "This room is outside autoroom category, VoiceChannelID: {}",
                &voice_channel.id().get()
            )
        ))
    };
    let autoroom = match AutoRoom::get_by_category_id(pool, category_id as i64).await {
        Ok(Some(_autoroom)) => _autoroom,
        Ok(None) => return Err(CommandError::from(
            format!(
                "This room is outside autoroom category, CategoryID: {}",
                &category_id
            )
        )),
        Err(err) => {
            println!("{:?}", err);
            return Err(CommandError::from("Something go wrong, please try again later"))
        }
    };
    
    let savedroom = SavedRoomDTO {
        autoroom_id: autoroom.id(),
        owner_id: room_cache.owner_id,
        name: match name {
            Some(_name) => _name,
            None => voice_channel_name.clone()
        },
        room_name: voice_channel_name,
    };
    let _ = match SavedRoom::insert(pool, &savedroom, &Vec::from_iter(room_cache.guests)).await {
        Ok(_) => ctx.say(
            format!(
                "The room record '{}' has been successfully saved",
                savedroom.name
            )
        ).await?,
        Err(err) => {
            println!("{:?}", err);
            return Err(CommandError::from("Something go wrong, please try again later"))
        }
    };

    Ok(())
}




    // let err_msg = format!("The connected voice channel was not found");
    // let monitored_autoroom = match MonitoredAutoRoom::get_by_owner_id(pool, author.id.get() as i64).await {
    //     Ok(option) => match option {
    //         Some(monitored_autoroom_result) => monitored_autoroom_result,
    //         None => return Err(CommandError::from(err_msg))
    //     },
    //     Err(_) => return Err(CommandError::from(err_msg)),
    // };

    // let channel_id = ChannelId::new(monitored_autoroom.channel_id as u64);
    // let result_msg = match grant_guest_privileges(&ctx, &channel_id, &user.id).await {
    //     Ok(_) => &format!(
    //         "{} has been successfully invited",
    //         user.mention().to_string()
    //     ),
    //     Err(_) => &format!(
    //         "Failed to invite {:?}",
    //         user.name
    //     ),
    // };
    
    // ctx.say(result_msg).await?;
    // Ok(())